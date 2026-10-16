---
id: ADR-0165
title: Prepare protected contributions and realize only independently required boundaries
status: superseded
date: 2026-10-12
supersedes: [ADR-0164]
superseded-by: ADR-0173
design: [§6, §6.1, §6.2, §15.1]
evidence: Proposed
---

## Context

The [Catalog review](../design_review/reviews/design_review_catalog-execution-patterns-target_2026-10-12.md)
establishes repeated full logical reconstruction and automatic durable per-view realization despite
protected fragment reuse (F01/F02), plus related assurance and consumer amplification (F03–F08).
The operator accepted RC01/RC02 and retained flat v4 under RC03 on 2026-10-12. The combined
[plan](../plans/catalog-completion-plan_2026-10-14.md) preserves lifecycle F01–F05, their current
implementation/evidence limits and approved lifecycle RC01–RC06. This decision replaces ADR-0164
self-containedly; its unaffected lifecycle/target ownership decisions remain below.

Typed semantic views, independent admission, exact protected copies and physical finality are useful
foundations. Their meaning does not require each internal relation view to become an independently
retained physical object. ADR-0152 continues to govern independently retained completion, including
full actual-state validation, immutable identity and freeze; this decision refines where that boundary
is required. Accepted target and inherited implemented/scoped-Tested foundations have different
maturity. This record does not establish new runtime acceptance or timeout causation.

## Options

1. **Keep universal per-view completion and add fragment caches.** Smallest caller change and
   conservative independent authority, but still sorts/hashes overlapping unions and creates/checks
   member/effect lifecycles for internal dependency selection. It leaves central amplification intact.
2. **Separate earned contribution, internal selection and retained boundary; compose existing runs.**
   Selected. Independently validate actual immutable contribution content once under its real owner;
   select internal views through that capability; realize and hash independently needed boundaries.
   Existing ordered runs, bounded native access and Arrow/root preparation supply local mechanisms.
   Requires explicit retained/cold/codec consumer migration and stronger lifetime clarity, but removes
   unnecessary state/effects without a second scheduler, database platform or portable proof cache.
3. **Replace semantic identity with a Merkle/DAG representation now.** Could reduce repeated boundary
   hashing but adds a new semantic/transport/admission version before removing unnecessary demand.
   Retain flat v4 and revisit only for a concrete remaining consumer after the selected correction.
4. **Use generic DataFusion sorting as the entire native completion mechanism.** Appropriate for
   Arrow relations, but native normalized IDs, byte equality/conflict refusal, terminality, physical
   authority and resource lifetimes still need adapters. Extending existing charged ordered runs has
   a smaller complete integration burden for the canonical families.
5. **Use shared logical test attempts for privileged beta suites, or add another target manager.**
   Logical attempts remain suitable for ordinary data controls but cannot contain beta's private
   root clients/DDL. Existing service/run/storage owners already provide whole-executor containment;
   reuse them for explicitly owned disposable targets rather than add another manager.

## Decision

### Completion and prepared access

The compiler/native/model owners distinguish independently validated contribution completion,
operation-local exact internal view selection and frozen independently retained boundary completion.
A contribution's capability protects actual immutable descriptors, membership/backing/originals,
local aliases, selected positive active incarnations and terminal native results. Exact internal
selectors preserve relation/dependency role, complete-empty versus absent, profile/model meaning and
composed directed aliases; they do not automatically allocate durable member copies or logical roots.
Retain actual parent ownership through borrowers; a descriptor surviving Workspace drop grants no
independent capability. Failure, cancellation or Unknown publishes no reusable success.

A frozen admitted/published operation earns a compound physical boundary over exact contributors and
bindings; logical view descriptors select only their declared content within it. Separately retained
products earn separate boundaries when their lifetime demands it. A retained view's validated exact
selector plus protected completed boundary constitutes its view-local physical realization under
ADR-0152: complete selected contributor/dependency closure, incarnations, codec and full member/backing,
collision and missing/extra validation remain required. Boundary completion alone grants no unchecked
selector authority or wider membership. Freeze selectors/mappings among identity-bearing surfaces;
internal operation capabilities cannot be serialized into retained authority. Useful member/access
indexes are prepared once per actual boundary. Version changed physical selector codec/schema/definition identity
and migrate internal, frozen, admitted/published, retained, sparse, backup and cold/import consumers
together. Keep exact selected-copy and per-selector absence semantics; union authority does not grant
a view extra members. No old-format runtime reader or compatibility authority survives hard cutover.

Normalize logical IDs before `RecordId::Ord`; fold the six canonical v4 JSON families with exact
key/equal-payload deduplication and unequal-payload refusal. Operation-owned immutable sorted runs
are reusable non-consumingly across unions. Bind preparation to complete family dependencies,
physical incarnations/full expected content and interpretation context; logical IDs alone do not
supply authority. Hash only required exact boundary unions, with successful EOF/finality before
publishing identity. Flat v4 remains authoritative; do not concatenate fragment digests. Direct
logical checksum and separately demanded portable export share useful immutable preparation.
Export requires frozen or independently retained read-only content, not merely read admission.
Detached import/restore independently validate actual incoming and installed state.

Remaining native assurance uses charged indexed identity-correlated record/guard/exact-hold windows,
with optional discovery distinct from required refusal and receiving effects still atomically
checking their exact premises. Directed aliases use prepared source adjacency/frontiers, preserving
cross-contributor reachability and audit's extra unreachable-source refusal. Semantic distinctions
do not require another physical stage. No generic proof registry, global compile lock or extra
platform follows; a retained cross-owner witness remains a separate consumer-driven fit decision.

### Preserved lifecycle, mutation and diagnostic contracts

Selected access resolves exact physical copies and contributor aliases under real live readers.
Independent observation may be reused only under owners protecting exact incarnation and validity;
per-layout/range/association admission remains independent. An already completed blob's pin and
independently validated blob-to-piece holds protect its exact immutable declaration/bytes. Check
same active positive incarnation and declaration before/after observation and on reuse; equal replay's
generic guard churn alone does not invalidate those bytes. Pre-completion finalization still captures
its revision and requires the owned recapture protocol. Retirement/recreation and privileged
maintenance cannot reuse stale observations; maintenance requires real quiescence/invalidation.

Equal peers recapture and fully revalidate only after conclusively observed same-incarnation premise
change. The original request atomically earns a bounded typed `applied` or `premise_changed` outcome;
read/validate it while its outcome reference remains held, including reconciliation/replay.
Premise-change records terminal outcome without a blob effect. Missing/mismatched outcome or uncertain
request never authorizes a fresh effect; retirement, ABA and owner fencing remain nonretryable.
Preserve cancellation/deadline ownership and normal concurrency. Continuous equal replay retains
conservative recapture unless transaction-local specialization preserves every owner/incarnation/
declaration, retirement/completion exclusion, incidence predicate and new-hold obligation. A client
precheck or skipped predicate touch is insufficient; the plan owns this bounded decision.

Model-owned bounded diagnostics retain known native cause identities and causal chains without native
implementation types. Opaque/pre-admission identities remain absent. Actual writers/readers migrate
together. Diagnostics grant no release authority and never collapse host, remote, storage or committed
outcomes. Inspection composes real owners and honest unobserved state; it does not reopen maintenance.

### Preserved development-target and tooling contracts

Managed main/validation remain the ordinary default. Privileged schema/permission/destructive controls
require explicitly issued wholly owned remote campaigns. Persist purpose, installation/generation,
endpoint, credential reference, effect scope and consumer before launch. Beta private root sessions
never attach to shared/operator targets. Reuse host admission, managed processes and executable/storage
owners. The parent owns the whole target even if the worker emits no report or creates private databases.

Report command outcome, capture health, worker cleanup, native certainty and target disposal separately.
Authenticate the exact campaign, close admission and drain owned workers/native executors before exact
daemon shutdown and datastore release. Logical DROP alone proves neither drainage nor reclamation.
Bounded disposal-only logical children are journaled and reconciled through their existing owners.
Confirmed whole-target shutdown/discard can settle failed logical cleanup without retaining old data;
Unknown executor drainage retains the obligation. Token substitution journals original placeholders
before mutation and restores through exact authenticated ownership before credential cleanup.

Keep exact SurrealKit 1.0.0-beta.1 development-only, outside product dependencies; product SDK/RPC
remain patched 3.3.0. Direct Any/Sync clients are separately configured/owned. Beta private clients
are contained by the whole target, not claimed as injected sessions. Deterministic native/model-owned
schema assets serve Sync and full ordered module-closure tester projections; no native build dependency
or handwritten semantic equivalent. Render lifecycle-sensitive inputs before hashing, refuse unsupported
combinations and compare actual required definitions/permissions/functions. Canonical installation
retains maintenance ownership; watch/pruning remain disposable-tool concepts. Feature availability,
source recipes and running binary/session qualification are distinct. Preserve explicit current support
limits and all conditional capability decisions; new adoption needs a concrete complete consumer.

## Consequences

Internal composition performs less repeated sorting, serialization, membership construction and native
effect work. The expected benefit is structural and unmeasured. This requires deliberate physical-codec,
retained-consumer and borrower-lifetime migration; generic caches or a changed completion flag cannot
substitute. The plan schedules all downstream restore/partition/identity/recipe corrections separately.

Regenerate project state/evidence from pinned inputs. No old data, historical benchmarks, converters,
compatibility readers, rollback assets or paused-campaign recovery is a prerequisite. Respect actual
owners before replacement, and preserve failed/Unknown outcomes even after checked discard.

The predecessor's former assembled-qualification consequence is retired in favor of the operator's
scoped functional acceptance: affected compile/contract controls, independent integrated design review,
and actual generated-analysis/admission/publication/native-query evidence. Broad repo-quality repair,
external embeddings, MCP-submitted query execution and operator adoption remain separately scoped.
The plan §6 owns implementation and closure; accepting this ADR closes no finding by itself.

Revisit for a concrete residual repeated boundary-identity consumer, independently retained proof need,
secure remote/canonical-beta deployment requirement, or pinned library behavior contradicting finality/
containment. Such evidence reopens the relevant bounded choice; it does not weaken exact authority.

## Amendments

2026-10-14 — Explicit operator authorization permits this navigation-only correction: the plan link now names the replacement Catalog creation coordinator. Decisions and dated evidence are unchanged.

2026-10-14 — Explicit operator authorization permits this navigation-only correction: Catalog-plan links now name the replacement Catalog completion coordinator. Decisions and dated evidence are unchanged.
