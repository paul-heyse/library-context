---
name: execute-plan
description: Execute authorized scope in an existing implementation plan through integration, functional verification and handoff, adapting to current evidence.
---

# Execute a plan

Carry the authorized scope through implementation, integration and acceptance using the existing
plan and established execution approach. Use [plan-execution](../plan-execution/SKILL.md) to
resolve consequential gaps when needed, including on direct invocation, without requiring
another planning round or approval. Reuse settled reasoning and assignments while adapting
to concrete evidence.

Refresh the current baseline where it affects the next work, especially concurrent edits and
prerequisite availability. Use the current plan, relevant `STATUS.md` checkpoint,
repository instructions and actual tree to establish what remains.
Delegated workers use their brief and relevant owners and may follow discovered dependencies.
Preserve concurrent work; distinguish accepted decisions, implemented prerequisites and proposals.
Prior receipts retain their original date and scope.

The refreshed design principle audit belongs to a new design-review engagement, not routine plan authoring or
execution. Use the plan's source findings and relevant architectural owners; loading review
criteria for a consequential question does not start a broad principle audit. Preserve audit/source-review
identities and the one scheduled-disposition owner without copying the audit's account into
another mutable ledger.

Carry out delegated assignments under the [shared coordination contract](../../../.agents/roles/README.md).
Keep affected workers informed when shared assumptions or dependencies change. The coordinator
retains responsibility for design decisions, integration and acceptance; delegation remains
discretionary. Independent review assesses an identified baseline.

Use the design principles and [Heuristics for Efficient Architecture](../../../docs/design_review/design_principles/core/efficient-architecture-heuristics.md)
for consequential choices left open by the plan or mismatches exposed during implementation.
Address relevant patterns before committing to the mechanism; reuse settled reasoning without
restarting reviews or introducing a whole-list assessment.

Resolve local implementation choices within the agreed behavior and contracts. When evidence
contradicts a shared assumption or affects a contract, scope or acceptance, reconcile the specific
issue with its owner and affected work. Update the remaining approach and its owning records
where needed, and continue independent authorized scope whose assumptions still hold. Do not
silently change the intended behavior or weaken acceptance to finish a local assignment. Seek
user input only when a material scope or authorization decision requires it.

Implement prerequisites before work that requires them, and integrate contributions with
their adjacent consumers. Complete the plan's applicable migration, preservation and retirement
obligations as part of delivering the target. Follow the repository's overview, Context7 and
primary-source research routes when library behavior matters. A completed assignment does not by itself
establish accepted integrated scope.

Start library work from the plan's recorded mechanisms and precise references, preserving their
intended composition, guarantees and advantages. Reassess details when the actual version or
integration premises differ. A consequential substitution needs a reasoned account of how it
preserves the target or changes it, with the owning plan and affected assignments reconciled
through existing decision and authorization routes. Routine API adjustments remain implementation
judgment; avoid silently replacing an enabling capability with weaker behavior or bespoke machinery.

Follow the binding's
[review cadence](../../../docs/design_review/design_principles/binding/library-context.md#reviews-in-this-repository).
Routine execution does not repeat the plan's foundation assessment or initiate recurring
architecture or domain-model reviews. Route warranted architectural decision changes through
[adr](../adr/SKILL.md) and the architectural owner.

Follow `AGENTS.md` for verification scope and timing, generation, formatting and hook ownership.
For generated outputs, diagnostic captures and temporary environments, follow the
[workspace storage contract](../../../docs/workspace-storage.md): identify the actual producer,
consumer capabilities and authoritative cleanup/release event, and acquire storage admission before
creation or use. Keep named immediate-use/replay/evidence holds; owner cleanup starts temporary
retention. Do not treat a changed pin, absent checkout, old mtime or directory size as release.
Protect legacy/unintegrated outputs instead of inventing deadlines. Preserve native cache policy
and available concurrency; no broad clean/purge or resource cap substitutes for lifecycle wiring.
Validate new behavior with relevant compile checks and targeted functional tests or probes;
perform integrated qualification at the repository's prescribed boundary. Repair failures and
rerun affected checks, broadening verification when a material change warrants it. Reassess
whether earlier evidence still applies when integration or subsequent edits change what it
exercised. A bounded package pass does not establish an enclosing stage exit.

Report actual commands and outcomes (`passed`, `failed`, `blocked` with its prerequisite, or
`not_run`). Record evidence in the existing receipt owner.
Keep proposed, implemented, tested and measured claims distinct, date verified claims, and
retain composite or partial qualification boundaries. Close findings only with evidence for
their stated obligation; preserve source-review IDs and their single disposition owner.

For a requested or explicitly planned dedicated assessment of scope claimed complete, use
[plan-completion-audit](../plan-completion-audit/SKILL.md). Reconstruct the accepted obligations
across packages and consumers, and reconcile the same plan's completion claims and next work.
This complements ordinary implementation review without recurring per-slice audits. Audit
authorization alone does not authorize repairs; when repairs are already within the execution
assignment, continue that authorized scope and update the plan against the resulting evidence.

Follow repository commit policy within the task's authorization. Use [handoff](../handoff/SKILL.md)
when the work changes what is true.
Keep durable status in its existing owner without a second execution plan or parallel ledger.
Clean up any remaining worktrees that are fully merged. Report the resulting behavior,
verification and any remaining authorized work or blockers; a partial checkpoint is not
completion of the requested scope.
