---
id: ADR-0175
title: Observe development runs at consumer-shaped scope and regenerate authenticated project targets
status: accepted
date: 2026-10-16
supersedes: [ADR-0172]
superseded-by: null
design: ["§1.2", "§6.5"]
evidence: Proposed
revisit: A consumer requires snapshot-stable discovery, verified immutable source closure, or an actual retained project consumer cannot regenerate.
---

## Context

[Efficiency F09/I03/I08/RC02](../design_review/reviews/design_review_catalog-completion-efficiency-target_2026-10-15.md)
identify repeated full-history and source preparation beyond actual development consumers. The
operator approved anchored discovery and lightweight invocation context with explicit full mode.
The [coordinator §6](../plans/catalog-efficiency-plan_2026-10-15.md#6-sole-disposition-and-current-checkpoint)
owns delivery and acceptance. This supersedes ADR-0172 self-containedly; authenticated delegated
ownership, readiness, capture, display and independent native finality remain in force.

## Options

1. Retain whole-history membership hashes and full source hashing on every invocation. They offer
   stricter continuation invalidation and broader observations but repeatedly prepare unrelated
   history/bytes and still do not prove a source-to-binary closure or filesystem immutability.
2. Select direct IDs, anchored live discovery and consumer-shaped source/diagnostic preparation.
   Chosen: existing filesystem, run and evidence owners supply the route; full safety enumeration
   and explicitly requested full source observation retain their real consumers.
3. Add a persistent discovery database, immutable-source VFS, generic diagnostics service, mandatory
   supervisor or single transient cgroup. These introduce synchronization and lifecycle machinery
   without a current consumer advantage. The prior inert systemd comparison also killed a reusable
   daemon and independently finalizing observer; it does not reject every possible cgroup design.
4. Migrate obsolete project runtime data or preserve rollback generations. Rejected for the
   operator-selected design-phase target: pinned inputs can regenerate it. Exact owned discard and
   fresh initialization are simpler than compatibility readers. Unexpected/operator scopes remain
   protected and require separate authority.

## Decision


Managed discovery/execution observe readiness after acquiring effective shared resource ownership,
held through actual consumption. Explicit exclusive preparation rechecks and validates before
release. Readiness never syncs; pure Rust controls need no Python/native state. ADR-0134 and
ADR-0144 remain the environment/storage authorities.

A run accepts prospective delegated owner/launch registrations before commands become usable.
Closing admission and observing the accepted child set share one short synchronization boundary.
Cancellation stops accepting work and requests registered owners to settle. Every child retains
its receipt's writer and authenticated identity; the enclosing run composes references and cannot
complete host/capture release while a registered writer is unresolved. Owner death retains the
references needed for exact selected recovery. No historical delegation is invented retrospectively.

Direct child/group exit, launch/supervisor failure, delegated-host cleanup, capture health and native
finality remain independent observations. Unknown/failed cleanup protects pruning, storage expiry
and worktree removal. Native Unknown is not settled by an empty process group/cgroup. Persistent
services, shared cache daemons and profiler observers retain their separate actual owners; detached
best-effort display is a reader whose failure cannot change command/capture supervision.

Current run writers/readers preserve historical IDs, bytes, exits and captures without bulk rewrite.
Selected recovery holds the owner lock, revalidates exact identity and can publish its recovery
observation without discarding original results. Full logs are authoritative; direct file capture
avoids display backpressure and suffix access handles small tails.

Exact run IDs resolve directly under the selected physical run root, refusing traversal and
symlinks. Discovery pages use a versioned anchored live cursor over that root and query: an upper
name anchor and last examined entry avoid full-history membership hashing. Deletions do not
invalidate continuation; new names above the anchor wait for a new listing, and backdated entries
in the remaining range may appear. Coverage distinguishes examined entries, hydrated records and
returned rows. Filters are observations, never a simultaneous historical snapshot. Complete safety
consumers retain explicit full inventory and fail closed on unresolved/corrupt records. No persistent
run index or cached safety answer is introduced.

Source observation defaults to versioned lightweight invocation context. Explicit full mode hashes
the declared source inventory, sharing compatible hints only within its actual owner. Hints and file
metadata do not certify immutability; unavailable or concurrently changing observations remain
visible. Only compatible full observations establish a source comparison. Reruns preserve the
original selected scope. Captured-tree compiler/provider verification remains full and independent.

Diagnostic preparation preserves ordered repeated flat fields, unknown parentage, ambiguous
identity and native JSON context. Unattributed data stays unattributed. Original envelope and source
line locators are separate from semantic interpretation; an adapter version invalidates derived
preparation without changing raw originals. Known loss is channel-specific: archive truncation
cannot weaken a complete JUnit channel. No generic diagnostic platform or causal inference is added.

Explicit selected-complete verification broadens each selected ordinary boundary and audits its
Cargo/Python coverage. It distinguishes selected completeness from whole-repository completeness,
reports declared exclusions and fails on unexpected omissions. Initial narrowing arguments are
refused; recorded boundary reruns retain their complete-scope meaning. Whole `test-all`/`qualify`
continue to mean their unchanged complete scope, including submitted MCP runtime.

Opt-in exact-command native capture extends existing run/fixture/verification launch seams and
ADR-0147's native-original authority. A short single registration operation attaches invocation
writers to the existing manifest; each writer owns its UUID receipt/raw artifacts and actual cwd.
Original/effective arguments, fixture context, command exit, capture health, postprocessing and
cleanup remain separate. Unsupported capture runs the original command. Bare tools remain
first-class. No shell parsing, synthetic test families, inferred source-to-binary closure or generic
telemetry store is introduced. Exact-case rerun planning still requires declared prerequisites.

Fixture diagnostics reuse actual content/configuration and native envelopes. Invocation-local
direct HTTP MCP remains limited to explicitly authorized non-sensitive disposable contexts;
native errors are not presumed sanitized. No global client registration or second datastore.
Codex remains the primary personal runtime, with explicit overrides and available parallelism.


Explicit project regeneration belongs to the existing installed-service owner. Require the exact
installation ID and both `library_context` main/validation scopes, actual root/namespace inventory,
exclusive admission and borrower locks, current authenticated clients/leases and exact daemon
shutdown before datastore discard. Keep installation identity, unit, endpoint and default binding;
rotate native generation and credentials and initialize/check the current definitions before
reopening. Target-owned executable provenance remains mandatory. No old-schema reader or rollback
copy survives. An immutable private operation journal pins candidates, scope and predecessor/
successor identities; intent precedes shutdown/discard, retries cannot discard successor data, and
failure leaves admission closed. Retire predecessor discovery bindings into operation evidence.
Separate discard acknowledgements settle old attachment obligations while preserving their original
failed/Unknown native results. Ordinary verification never invokes regeneration implicitly.

## Consequences

Implementation reduces repeated preparation while keeping safety and source claims explicit.
Anchored discovery sacrifices snapshot membership; lightweight observation sacrifices full source
comparison unless explicitly requested. These are response contracts, not hidden optimizations.
Regeneration requires a deliberately authorized destructive operation and exact current drainage;
metadata, local exit alone and installation labels cannot establish its authority.

Accepted decisions are not functional qualification. Source and focused controls are in progress;
the coordinator owns integrated reconstruction, selected ordinary acceptance, strict keep-going
Clippy and CE99 handback. Prior receipts keep their exact scope; old plans await later reconciliation.
No benchmark, measured speed claim, operator adoption, embedding inference, real-library pilot or
submitted MCP runtime qualification follows from this decision.
