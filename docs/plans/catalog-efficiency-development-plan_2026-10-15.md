# Catalog efficiency: development execution and evidence preparation

**Proposed, 2026-10-15.** Companion to the [coordinator](catalog-efficiency-plan_2026-10-15.md),
which owns sole current disposition. This document specifies F01/F09, the source-observation
half of I03 and I08 from the [source review](../design_review/reviews/design_review_catalog-completion-efficiency-target_2026-10-15.md).
CD00 is source-integrated with focused host controls passed; the coordinator owns the dated
receipt and remaining acceptance. Other development packages retain their separate scope.

## 1. Responsibility, baseline and target

The host run/delegation owners govern authenticated processes, launch and stop requests, writer/
observer lifetime and cleanup. Evidence owners govern captured channels, attribution and reporting
limits. Host cleanup, command exit, capture health, native certainty and storage release remain
independent. Tooling helps the sole operator/Codex act on these distinctions; it cannot fabricate
native finality or executable provenance.

Read the [development owner](../design/DESIGN.md#section-1-2), ADR-0172 and the existing
[workspace-storage guide](../workspace-storage.md). Current registered owners, bounded detailed
run hydration, shared process snapshots, explicit complete safety inventory and native recordings
are foundations to retain. No new mandatory runner, hook, agent scheduler, concurrency cap,
semantic-search system or run database is required.

| Package | Responsibility and input | Output |
|---|---|---|
| CD00 | Actual authenticated live participants and existing recovery receipts | Order-independent stop issuance/escalation, truthful protected cleanup |
| CD01 | Exact run reference or page request under the selected root | Direct exact resolution or justified page/complete-inventory contract |
| CD02 | Actual source-observation consumer and invocation context | Adequate cheaper/shared observation, with honest provenance limits |
| CD03 | Actual consumed evidence channels and diagnostic question | Known-loss/attribution contract and justified selected signal/preparation changes |

CD00 is high priority because later native controls need reliable host cancellation. The other
packages are independent except for provenance/observation contracts shared by CD02 and CD03.

## 2. CD00 — F01 actual owner requests govern delegated cancellation

[request_stop](../../scripts/run_delegation.py) currently fabricates a same-owner request timestamp
before deciding whether a registration is live. A completed-first registration can suppress the
live owner's initial signal and restart grace at every poll. The review's substituted-signal
[probe](../design_review/evidence/2026-10-15_catalog-completion-efficiency/delegated-stop-probe.py)
establishes this branch defect, not actual host drainage.

Group participants by full authenticated owner identity. Classify terminal registrations, active
owner locks, child groups, finalizing observers and unresolved obligations before deciding whether
the owner has actually received a request. Reuse existing run/delegation and recovery ownership;
one owner-level issuance fact may be reflected in multiple child-specific receipts, but enumeration
order must not create that fact.

Persist actual request state under the appropriate authenticated ownership before relying on it
to suppress another issuance or age grace. Distinguish request persistence/attempt from successful
signal delivery. If persistence or signalling fails, report that state and preserve a valid retry/
escalation route; never claim a delivered request or start a new grace clock on each poll. Coordinate
concurrent stop callers against the same owner. Do not signal an unauthenticated replacement PID,
a foreign owner, the calling owner or a reusable shared daemon.

Escalation uses the original durable request time and the composed owner's actual live obligations.
Keep independently finalizing observers separate and preserve their established protection; a
completed setup registration is neither proof the owner ended nor a reason to stop supervising
its later writer. Recover child groups only through their authenticated existing owner route.
Original command/child outcomes remain immutable; parent recovery receipts add separate facts.

**Delivery:** migrate delegated stop and its direct cancellation/reporting consumers to the actual
owner request interpretation, remove manufactured request state, and retain run/storage protection
until authenticated writer and observer cleanup. No replacement whole-cgroup kill model is assumed.

**Controls:** completed-before-live, reversed/shuffled order, multiple live registrations for one
owner, repeated polls, concurrent stop callers, dead owner, PID reuse/foreign identity, failed
persistence, failed signal, observer finalization and shared daemon protection. Supplement the pure
branch control with a small wholly owned host process/control using existing run ownership, with
authenticated teardown and no database. It establishes host behavior only; native finality remains
independent. No surviving task-owned process is an acceptable passing cleanup result.

## 3. CD01 — F09 selected run discovery and page preparation

### Direct exact identity

Resolve a validated exact child under the selected run root before preparing historical inventory.
Check basename/path containment, symlink policy and applicable run identity/ownership before use.
Refuse traversal and foreign targets. Keep damaged/corrupt metadata explicit where the existing
reporting contract supports inspection; damaged is not absent and grants no cleanup authority.

If no exact run exists, retain current last/latest, unique prefix/suffix and newest-label behavior,
including ambiguity and missing errors. Complete safety discovery still enumerates the full relevant
inventory and fails closed on unresolved/corrupt ownership. A fast user lookup never narrows a safety
caller's coverage.

### Page consistency decision and delivery

[run_listing](../../scripts/run_listing.py) presently enumerates/hash-prepares history and repeats
enumeration for membership stability. Compare one preparation under the existing stronger contract
with anchored continuation/explicit coverage that avoids whole-history invalidation. Include root
identity, selection changes, new/deleted directories, damaged records and state filters.

RC02 approves changing the direction if the alternative is selected; it does not require a specific
cursor format. Specify the selected page consistency semantics before implementation, including what
inserts/deletes may be omitted, how an anchor survives removal, which selection/root changes invalidate
it, and how examined coverage differs from hydrated rows. A persistent derived index is eligible only
if its maintenance/recovery burden beats direct preparation.

If continuation semantics change, migrate help/README, responses, cursor producers/consumers and the
development owner through the affected ADR-0172 decision route. No compatibility-only old cursor
reader is needed in design phase; reject obsolete cursors explicitly. If keeping membership-based
invalidation, retain its necessary observations and remove redundant preparation. Exact-ID work can
finish independently.

**Controls:** exact-ID lookup proves no unrelated enumeration; missing/ambiguous prefix/suffix/
label resolution; traversal/symlink refusal; malformed receipt; empty history; insert/delete during
pagination; root/selection change; active/unresolved/complete state; complete safety coverage.
Use temporary task-owned receipts and explicit discovery-call observations, not a fragile timing threshold.

<a id="cd02"></a>
## 4. CD02 — I03 consumer-shaped source observations

This package owns auxiliary repository observations, not actual analysis-capture integrity.
[CC01](catalog-efficiency-compiler-plan_2026-10-15.md#cc01)
retains that separate responsibility.

Each fresh [SourceObserver](../../scripts/verify_evidence/context.py) hashes allowed tracked and
untracked files, including docs/vendor source; nested CaptureOwners have separate caches.
First establish which actual reader needs full-tree before/after context and which only needs
invocation/build identity. Current observations already admit concurrent-change limits and
source-to-binary unknown; preserve that honesty.

Compare cheap invocation/build observations, explicitly requested full-tree context, and shared
compatible observations under one existing run/capture owner. Define compatibility by root,
configuration, requested scope, observation time/generation and actual consumer need. Shared
stat/digest caches are hints, not proof that intermediate mutations never occurred. Preserve
argv, cwd, configuration, unavailable/symlink/changed-during-observation facts and explicit exclusions.

Do not simply omit vendor or docs because they are large: they can be relevant to a selected
build/context question. Conversely, do not hash the whole repository merely because a nested
writer started. Reuse observations only under their actual premise and name what they cannot prove.
No new provenance registry or build-closure proof system is authorized.

**Decision and delivery:** choose adequate observation scopes/lifetimes; migrate capture/context
producers and actual report/export consumers together; remove redundant observations or keep
the stronger scope with a supported rationale. Coordinate CD03's channel-attribution needs;
I03 closes only when both this half and CC01 are settled.

**Controls:** nested capture owners; compatible/incompatible roots/scopes; untracked input;
source change between and during observations; changed configuration; missing/symlink files;
no compiled closure evidence; portable report context. Assert actual selected coverage and
reuse boundaries, never a stronger source-to-binary claim.

## 5. CD03 — I08 source-specific diagnostic coverage and preparation

The concurrent operation-diagnostics review has a separate operational target. It supplies
leads and evidence; this plan does not import its full scope, gate verdicts or always-on platform.
Preserve the source review's narrower independent conclusions:

- The [runtime adapter](../../scripts/verify_evidence/runtime_adapter.py) retains matching flat
  fields and ordered JSON span context. Flat text may lose parentage or overwrite repeated names;
  do not describe this as absence of all association.
- [Projection](../../scripts/verify_evidence/projection.py) can prepare observations from native
  envelopes and native.log, while reporting labels supplemental observations. Duplicate preparation
  may be inefficient without being a false unique-test count.
- [Nextest coverage](../../scripts/verify_evidence/nextest.py) reports per-output truncation unknown.
  Establish the relationship between a known marker and the actual consumed payload before changing it.
- [Phase](../../crates/lctx-native/src/phase.rs) captures its actual span/dispatcher and emits through
  that owner. Drop does not invent a terminal. Earlier propagation and omitted event kinds remain questions.

### Decision questions

Identify which consumer needs structured parentage rather than retained flat association, which
consumed channels have known loss, and which live/post-run operational question existing data cannot
answer. Determine whether one attributed preparation can serve overlapping channels while retaining
original provenance, source identity, locators, coverage and replay.

For a consumed known-truncated payload, expose known loss and its exact locator there; keep
unknown coverage for channels where loss cannot be determined. An unmatched begin means incomplete
observation, not proof an operation stalled or remains live. Supplemental phase/attempt observations
remain distinct from executions and unique tests.

Compare existing structured tracing, DataFusion metrics, server statistics and native statement/
query-plan evidence where they answer an actual question. Use exact library capability routes and
Context7 for new documentation questions; availability alone does not justify periodic sampling,
observer threads, mandatory journals or new diagnostic infrastructure.

### Conditional delivery and controls

Implement useful selected attribution/coverage/preparation changes through capture, projection,
reporting, export and portable readers together. Preserve original raw channels, receipt identities,
host/native cleanup axes and stated limitations. Any needed new signal has a named consumer,
bounded cost/lifetime and an existing owner; no signal reports guessed progress or terminality.

Controls include text fields with repeated names, ordered JSON parentage, duplicate supplemental
channels, known truncation in one consumed payload, another channel with unknown coverage,
unmatched begin, lost terminal, late error, filtered/scoped dispatcher and portable original
locators. Use synthetic small channel records plus retained real examples where decisive; no
long journey is needed to establish parsing semantics. A new native/metric signal additionally
needs its own minimal real functional control.

G7's bounded uncertainty resolves only for the covered sources/consumers. F01 and native defects
do not become closed merely because their observations improve.

## 6. Integration, verification and release

Use existing command, run, capture and storage owners. CD00 may change registered host request
state; CD01 may change public discovery continuation; CD02/CD03 may change evidence schemas or
trust boundaries. Migrate actual consumers in the delivering package, use the required decision
route, and include shared receipt/trust changes in coordinator CE90.

During implementation run focused Python/host controls and warnings-inclusive Ruff/Pyrefly for
affected code under Python 3.14.7. Native signal/query preparation changes receive affected compile/
native controls; mocked records are not proof of actual native behavior. Scope-end applicable
leaves run once, with root scoped turn-end to preserve concurrent work.

Generated evidence has an actual owner/consumer through cleanup. Preserve raw failures, citations,
unresolved capture/native outcomes and other sessions' work; release a consumer only when its
obligation transfers or ends. No storage deletion follows from age, size, duplicates or an improved
observation. Existing plan documents and operator state are not effects of this plan.
