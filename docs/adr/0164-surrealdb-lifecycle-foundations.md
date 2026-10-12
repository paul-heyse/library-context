---
id: ADR-0164
title: Own disposable development targets and protected native lifecycle observations
status: accepted
date: 2026-10-12
supersedes: []
superseded-by: null
design: [§6, §15.1]
evidence: Proposed
---

## Context

The lifecycle target review identifies protected-serving bypasses, repeated independent
observation, equal-peer completion conflicts and lost durable failure causality (F01–F05).
The operator approved RC01–RC06 and a distinct foundation/investigation slice in the
[lifecycle coordinator](../plans/surrealdb-lifecycle-plan_2026-10-12.md).
SurrealKit 1.0.0-beta.1 is the selected development library; its public tester creates private
clients and random databases. Ordinary shared validation attempts cannot contain its DDL,
permissions and cleanup behavior. Existing native ownership, immutable content and request
finality remain useful foundations. Implementation and verification remain separately recorded.

## Options

1. **Continue only shared logical validation.** It remains the simplest route for ordinary
   product data attempts, but cannot safely host beta's privileged schema suites.
2. **Reuse the existing campaign/service/run owners for disposable development targets.**
   Whole-service containment costs one campaign daemon, reused across parallel suites, and
   supplies a final executor boundary for beta's private clients. It avoids a second manager.
3. **Add durable proof caches or serialize compilation.** Neither is needed to fix equal peers
   or repeated observation inside a held scope. Additional invalidation and scheduling machinery
   is justified only by a real remaining consumer after the scoped correction.
4. **Compile schema generation through a native build dependency.** This duplicates the large
   host native dependency closure. Explicit deterministic generation plus embedded assets and
   correspondence controls keeps one authority without requiring that build consequence.

## Decision

Retain managed main/validation as the ordinary default. Permit schema/permission/destructive
tests only on an explicitly issued, wholly owned disposable campaign target. Persist purpose,
exact installation/generation, endpoint, credential reference, effect scope and consumer before
launch. Beta root credentials never attach to the shared or operator service. Reuse existing
host admission, managed processes and executable/storage owners; no new scheduler or registry.

The parent owns the whole target even if beta creates no report. Report command outcome,
capture health, worker cleanup, native certainty and target disposal independently. Disposal
authenticates the exact campaign, closes admission and confirms owned worker/native-executor
drainage before stopping the exact daemon and releasing datastore storage. Logical DROP alone
does not establish drainage. Confirmed whole-target shutdown/discard may settle failed logical
cleanup without preserving obsolete data. Unknown drainage retains the current obligation.

Native selected access resolves physical members and contributor aliases under exact live
readers. Independent audit uses bounded indexed sets and full physical terminality. Reuse an
observation only inside owners protecting its exact incarnation and necessary validity premises;
per-layout/range/root admission remains independent. A retained cross-owner witness is a
separate fit decision, not a prerequisite.
For an already completed blob, the retained blob pin and independently validated blob-to-piece
holds protect its immutable completed declaration and bytes. Check that exact declaration and
same active positive incarnation before/after observation and on reuse; equal replay's guard
revision churn is not a byte-validity premise. Pre-completion finalization still validates its
captured revision and uses the typed recapture protocol below.

Equal peers use same-incarnation recapture and full revalidation after a conclusively changed
audit premise. A bounded typed payload outcome is earned atomically with the original request;
read and validate it while its outcome reference remains held, including reconciliation/replay.
Premise-change records terminal request outcome without a blob effect. Missing/mismatched
outcome or uncertain request never authorizes a fresh effect; retirement, ABA and owner fencing
are nonretryable. Preserve existing cancellation/deadline ownership and normal concurrency.

Model-owned diagnostic values project known native cause identities and bounded causal chains
without depending on native implementation types. Opaque/pre-admission causes retain absent
identities. Actual current writers/readers migrate together; diagnostics confer no release
authority and do not collapse local/remote/storage/committed outcomes.

Keep exact beta tooling outside product dependencies, with separate directly owned Any clients.
Generate deterministic embedded assets from native/model authority and verify beta definition
transformation against native installation. Tester projections use the full ordered dependency
closure. Render lifecycle-sensitive inputs before hashing and refuse unsupported target/module
combinations. Canonical installation retains its maintenance owner; watch/pruning is disposable
target tooling. Preserve the patched SDK/server family and qualify actual sessions/capabilities.

## Consequences

This narrows repeated physical work while preserving independent completeness, selected-copy
meaning and finality. Whole-service ownership contains beta's noninjectable clients but requires
explicit campaign inventory and disposal. No historical data, benchmarks, compatibility readers,
rollback assets or paused-campaign recovery are prerequisites: regenerate from pinned inputs,
while respecting actual live owners before discard.

Revisit when a concrete consumer requires retained completeness, secure remote deployment or
canonical beta installation, or when pinned library behavior contradicts containment/finality.
Do not silently broaden capabilities or weaken a protected route. The coordinator §6 owns
implementation and findings; accepting this decision does not close them. The foundation slice
ends with focused controls, independent integrated review and one assembled qualification on
its fresh campaign; final whole-track LC01/LC02/LC03 remain separate.
