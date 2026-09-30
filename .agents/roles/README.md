# Agent roles and coordination

Use subagents for review, planning and execution. Use concurrency as you see fit; strive for parallel
execution. The coordinator chooses the decomposition and remains responsible for design, integration
and acceptance. The roles are reusable capabilities; a task need not use every role.

## Roles and runtime routing

Read the relevant shared contract when assigning work. Workers load [the common contract](worker.md)
and their role; native definitions carry model and tool settings.

| Responsibility | Shared contract | Codex model / effort | Claude agent / model |
|---|---|---|---|
| Repository evidence and dependency mapping | [code-mapper](code-mapper.md) | `gpt-6-luna` / high | `code-mapper` / haiku |
| Library capabilities, contracts and alternatives | [library-research](library-research.md) | `gpt-6-luna` / high | `library-research` / haiku |
| Independent architecture and domain assessment | [design-reviewer](design-reviewer.md) | `gpt-6.1-sol` / high | `design-reviewer` / opus |
| Bounded implementation with local discretion | [executor](executor.md) | `gpt-6.1-sol` / high | `implementer` / sonnet |
| Correctness and regression review | [implementation-reviewer](implementation-reviewer.md) | `gpt-6.1-sol` / medium | `implementation-reviewer` / sonnet |
| Functional verification and failure diagnosis | [test-agent](test-agent.md) | `gpt-6.1-sol` / medium | `test-agent` / sonnet |

Codex definitions live in [`.codex/agents/`](../../.codex/agents/); Claude definitions live in
[`.claude/agents/`](../../.claude/agents/). Claude's `implementer` is the executor adapter, not an
additional role. The Codex coordinator defaults to Astra/high; use greater effort when the actual
reasoning warrants it. These are starting allocations to calibrate from results, not measured
cost or quality claims. Preserve a user's explicit runtime choice.

Prefer the named native role. If the available delegation tool has no role selector, supply the
shared contract paths and choose the table's model and effort explicitly. With Codex's collaboration
tool, use a focused brief with `fork_turns="none"` when selecting a different model; a full-history
fork inherits the parent. If the runtime cannot select a model, disclose the fallback. Do not assume
editing configuration changes agents already running. Native sandbox defaults supplement the task
contract; live parent permission overrides can take precedence.

## Coordinate the work

Give each assignment an outcome, bounded scope, relevant authorities and baseline, permitted edits,
dependencies and useful completion evidence. Provide enough context to act independently without
copying the whole conversation. Reuse a worker for coherent follow-up; use fresh context for an
independent review, giving requirements and source evidence without steering its verdict.

Parallelize work whose inputs are ready. Resolve shared contracts before dependent edits and name
one owner for shared model declarations, manifests, generated surfaces and integration. Use separate
worktrees only for genuinely concurrent production editing, as AGENTS.md requires. Review or test a
stable revision or identified tree; re-evaluate affected evidence after integration changes it.
Keep a single writer for the final plan, shared status and finding disposition unless ownership is
explicitly partitioned. Existing plans own scheduled work; do not create a second task ledger.

Read decisive evidence and reconcile results rather than accepting a worker's completion message
as verification. Resolve architectural contradictions through the existing decision and review
process; retain unresolved findings and their effects on acceptance. Executors choose local details
within their brief. Delegate additional evidence gathering or increase reasoning effort when a
material ambiguity needs it, without sending every small decision back to the user.

## Workflow integration

| Workflow | Typical contribution |
|---|---|
| Review planning and design review | Mappers and library researchers establish evidence; a fresh design reviewer forms the architectural judgment |
| Plan preparation and creation | Evidence workers inform options and the integral foundation assessment; focused design advice addresses consequential questions; one author assembles the document |
| Execution planning and execution | The coordinator orders ready packages; executors implement; implementation reviewers and test agents assess stable work; design review follows the binding's cadence |

The [process skills](../../.claude/skills/README.md) own each workflow. Planning companions produce
an adaptable conversational approach and can use the available planning interface. Skills do not
switch the runtime into Plan mode. No extra planning artifact or hook is required.

Configuration references, consulted 2026-09-30:
[Codex custom agents](https://learn.chatgpt.com/docs/agent-configuration/subagents) and
[Claude custom subagents](https://code.claude.com/docs/en/sub-agents).
