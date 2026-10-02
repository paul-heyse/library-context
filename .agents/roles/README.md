# Agent roles and coordination

Use subagents when independent coverage, context isolation, distinct capabilities or independent
judgment justify their handoff and integration cost. Parallelize ready, independent work; keep small
or tightly coupled tasks with the coordinator when that is simpler. The coordinator remains
responsible for design, integration and acceptance. No role sequence or minimum agent count applies.

## Roles and runtime routing

Read the relevant shared contract when assigning work. Workers load [the common contract](worker.md)
and their role; native definitions carry model and effort settings only.

| Responsibility | Shared contract | Codex model / effort | Claude agent / model / effort |
|---|---|---|---|
| Repository evidence and dependency mapping | [code-mapper](code-mapper.md) | `gpt-6-luna` / high | `code-mapper` / sonnet / medium |
| Library capabilities, contracts and alternatives | [library-research](library-research.md) | `gpt-6-luna` / high | `library-research` / sonnet / medium |
| Independent architecture and domain assessment | [design-reviewer](design-reviewer.md) | `gpt-6.1-sol` / high | `design-reviewer` / opus / high |
| Bounded implementation with local discretion | [executor](executor.md) | `gpt-6.1-sol` / high | `implementer` / sonnet / high |
| Correctness and regression review | [implementation-reviewer](implementation-reviewer.md) | `gpt-6.1-sol` / high | `implementation-reviewer` / sonnet / high |
| Functional verification and failure diagnosis | [test-agent](test-agent.md) | `gpt-6.1-sol` / medium | `test-agent` / sonnet / high |

Codex definitions live in [`.codex/agents/`](../../.codex/agents/); Claude definitions live in
[`.claude/agents/`](../../.claude/agents/). Claude's `implementer` is the executor adapter, not an
additional role. The Codex coordinator defaults to Astra/high; use greater effort when the actual
reasoning warrants it. These are policy defaults, not measured cost or quality claims. Revisit them for concrete
failures or model/runtime availability changes; no calibration exercise is required. Preserve a user's explicit runtime choice.

Prefer the named native role. If the available delegation tool has no role selector, supply the
shared contract paths and choose the table's model and effort explicitly. With Codex's collaboration
tool, use a focused brief with `fork_turns="none"` when selecting a different model; a full-history
fork inherits the parent. If the runtime cannot select a model, disclose the fallback. Do not assume
editing configuration changes agents already running. Native role files carry no permission
restrictions: Codex roles inherit the session's sandbox and approval settings (Codex ignores
sandbox keys in role files), and Claude roles inherit the session's tools (no `tools` lists). The
shared contracts alone define each role's permitted effects; live parent permission overrides take
precedence.

## Coordinate the work

Give each assignment an outcome, bounded scope, relevant authorities, settled decisions and exact
baseline (including relevant uncommitted changes), permitted files/effects, sibling ownership,
dependencies and useful completion evidence. Provide enough context to act independently without
copying the whole conversation. Reuse a worker for coherent follow-up; use fresh context for an
independent review, giving requirements and source evidence without steering its verdict.

Parallelize work whose inputs are ready. Resolve shared contracts before dependent edits and name
one owner for shared model declarations, manifests, generated surfaces and integration. Use separate
worktrees only for genuinely concurrent production editing, as AGENTS.md requires. Give reviewers concrete acceptance criteria, affected consumers and relevant failure cases. Review or test a
stable revision or identified tree; re-evaluate affected evidence after integration changes it.
Keep a single writer for the final plan, shared status and finding disposition unless ownership is
explicitly partitioned. Existing plans own scheduled work; do not create a second task ledger.

Read decisive evidence and reconcile results rather than accepting a worker's completion message
as verification. Resolve architectural contradictions through the existing decision and review
process; retain unresolved findings and their effects on acceptance. Executors choose local details
within their brief. Surface consequential absence claims, conflicting evidence, unsupported cross-version transfers,
unresolved ownership or contract decisions, and a second failed attempt at the same repair. The
coordinator chooses whether to narrow the question, inspect sources, run a focused functional check,
handle it directly or route stronger work. These are reassessment triggers, not automatic reruns.

### Stronger-worker routing

Codex custom roles pin their model and effort, overriding spawn-time choices. For stronger evidence
work, use the built-in `default` role with `gpt-6.1-sol` / `high`, `fork_turns="none"`, and a focused
brief supplying the common and relevant evidence-role contracts, sources and the evidence role's
permitted effects.
Use the built-in `worker` only for authorized edits with explicit ownership. Do not create duplicate
higher-tier roles or assume an override changes a named custom role.

Claude permits a per-invocation model override: selecting Opus for an evidence worker retains that
role's configured medium effort. There is no per-invocation effort override; use the existing
Opus/high design-reviewer when the question calls for design judgment. Otherwise the coordinator
can resolve the uncertainty directly. A configuration edit does not change an already-running agent.

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
