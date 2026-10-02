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

Carry out delegated assignments under the [shared coordination contract](../../../.agents/roles/README.md).
Keep affected workers informed when shared assumptions or dependencies change. The coordinator
retains responsibility for design decisions, integration and acceptance; delegation remains
discretionary. Independent review assesses an identified baseline.

Resolve local implementation choices within the agreed behavior and contracts. When evidence
contradicts a shared assumption or affects a contract, scope or acceptance, reconcile the specific
issue with its owner and affected work. Update the remaining approach and its owning records
where needed, and continue independent authorized scope whose assumptions still hold. Do not
silently change the intended behavior or weaken acceptance to finish a local assignment. Seek
user input only when a material scope or authorization decision requires it.

Implement prerequisites before work that requires them, and integrate contributions with
their adjacent consumers. Complete the plan's applicable migration, preservation and retirement
obligations as part of delivering the target. Follow the repository's library-skill and
documentation routes when library behavior matters. A completed assignment does not by itself
establish accepted integrated scope.

Follow the binding's
[review cadence](../../../docs/design_review/design_principles/binding/library-context.md#reviews-in-this-repository).
Routine execution does not repeat the plan's foundation assessment or initiate recurring
architecture or domain-model reviews. Route warranted architectural decision changes through
[adr](../adr/SKILL.md) and the architectural owner.

Follow `AGENTS.md` for verification scope and timing, generation, formatting and hook ownership.
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

Follow repository commit policy within the task's authorization. Use [handoff](../handoff/SKILL.md)
when the work changes what is true.
Keep durable status in its existing owner without a second execution plan or parallel ledger.
Clean up any remaining worktrees that are fully merged. Report the resulting behavior,
verification and any remaining authorized work or blockers; a partial checkpoint is not
completion of the requested scope.
