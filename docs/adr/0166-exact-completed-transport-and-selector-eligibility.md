---
id: ADR-0166
title: Bind portable copies and native substitution independently of semantic identity
status: accepted
date: 2026-10-13
supersedes: []
superseded-by: null
design: [§6.1, §6.2, §15.1]
evidence: Interface-checked
---

## Context

Catalog F02/F04/F07 expose a distinction the flat six-family v4 identity intentionally does not
encode: which exact contributor copy supplies each root selector's local aliases. Native selectors
retain that correspondence, but descriptor-only cold allocation and first-copy restoration lose it.
Publication reuse and retained attachment also require actual substitution eligibility, rather than
content-union equality. The operator selected a separate envelope on 2026-10-13. ADR-0165's protected
completion and direct canonical preparation remain in force.

## Options

1. **Refuse all nonempty portable aliases and unequal native collisions.** Safe refusal is preferable
   to inferred ownership, but leaves supported native states untransportable and prevents legitimate
   equal-content/different-eligibility publications from coexisting.
2. **Replace semantic v4 with a new incidence-bearing semantic identity.** Coherent if product content
   equality must imply selector eligibility, but changes all semantic consumers and conflates stable
   content meaning with a separately useful capture/substitution contract.
3. **Separate capture correspondence and substitution eligibility.** Selected. Reuse protected native
   inventories, canonical ordered runs and independent admission. The carrier preserves exact copies;
   a canonical eligibility projection permits equivalent fresh copies to converge without requiring
   physical identity or graph-isomorphism normalization. This adds narrow model/codec contracts and
   explicit caller receipts instead of another cache, database authority or signature service.

## Decision

The model owns a versioned completed transport envelope and receipt, independently of unchanged
semantic format4/v4. Ordered root-qualified slots select dense first-use portable copy labels. Every
copy retains its full completed descriptor and complete membership/local-alias inventories, including
zero inventories. Equal descriptors may name distinct copies. Counts, full descriptors, codec and
terminal EOF bind capture identity; cold adapters independently reconstruct the actual inventories.
Detached copy addresses derive from the receiving attempt plus a capture/label discriminator, never
a transported source attempt. Installation schema8 declares that physical discriminator explicitly.

Eligibility binds the full ordered roots and root-qualified logical slots, completed descriptors and
complete local aliases. Physical addresses, capture labels and incidental sharing do not participate.
This conservative sufficient equivalence is intentionally not graph-isomorphism canonicalization.
SnapshotHandle and publication recipes advance to v3 with typed eligibility. Incoming and winning
boundaries independently earn protection and eligibility before unchanged reuse or staging convergence.
Different eligibility receives a different publication identity even with equal semantic v4.

A retained receiver compares only actual collisions in the same root namespace. Unequal roots refuse
attachment before effects; distinct roots retain independent dependency mappings. Overlapping local
writers serialize compatibility checks, native attachment and metadata publication. No unchecked
first logical candidate or `or_insert` supplies substitution authority.

Exports/artifacts/selected backups return typed caller-retained expectations only after complete EOF,
checked owned finalizers, output sync, staged commit and relevant directory sync. Imports/restores
require those expectations explicitly, never manufacture them from incoming headers or sidecars, and
independently earn fresh native physical and semantic admission. Receipts are comparison data, not
portable native grants. Backup decoding remains bounded data-only parsing. Native holds, pins, guards,
incarnations and source attempts are not destination authority.

CU01 uses dump-owned demanded metadata preparation; CU02 prepares exact input-port positions; CU04
retains one immutable versioned Rust recipe response per checked Python specification. These are local
consumer mechanisms under ADR-0165, without a universal preparation object or process-global cache.

An admitted artifact may outlive the producer's read catalog. Its borrowed admission runtime retains
the original execution owner: spill files, bridge, native calls/store, provider joins, cache session
and signal. Both callers share the existing checked drainage policy; retaining opaque resources alone
does not establish finality. Explicit workspace drainage still terminates shared work. After the last
artifact operation, the caller drains owned execution, performs required reconciliation/cleanup, then
completes the original signal with that enclosing report. Export finishes its delivery operation;
neither export nor a borrowed publication call implicitly ends the artifact's whole lifetime.

## Consequences

Same semantic content can have multiple legitimate native publications; callers must preserve the
complete current handle. Portable copies and independently supplied receipts become explicit API/CLI
inputs. Changed codecs cut over completely, without historical readers or dual runtime paths. Useful
current content may be regenerated from pinned inputs; unrelated operator/shared state is untouched.

Acceptance of this decision is not implementation or runtime qualification. The [Catalog coordinator
§6](../plans/catalog-completion-plan_2026-10-14.md#6-sole-disposition-and-current-checkpoint) owns exact
CU01–CU04, overlapping CP04 and CE01 dispositions. Revisit eligibility only for a concrete consumer
requiring semantic equality to imply substitution or a supported equivalence rejected by this
conservative projection; do not infer such a requirement from transport deduplication alone.

## Amendments

2026-10-14 — Explicit operator authorization permits this navigation-only correction: the plan link now names the replacement Catalog creation coordinator. Decisions and dated evidence are unchanged.

2026-10-14 — Explicit operator authorization permits this navigation-only correction: Catalog-plan links now name the replacement Catalog completion coordinator. Decisions and dated evidence are unchanged.
