# Subagent Orchestration Best Practices for library-context

Sep 30, 2026 · @Paul

## Executive summary

The library-context setup is close to best practice on topology, shared role contracts and brief content. Its real gaps are four: delegation is a disposition rather than a rule, Claude's evidence roles run on the weakest tier, workers re-read context they already have, and the escalation path it describes doesn't work as written in either runtime.

The broader research confirms most of the earlier advice and changes five points:

- **Writes should stay single-threaded by default.** Cognition's 2026 production report and Google Research's 180-configuration study agree: parallel readers pay off, parallel writers rarely do, and sequential work gets worse.
- **Clean-context review deserves more weight, not less.** Keep an independent review at every package boundary; drop only reflexive "double-check your work" subagents.
- **Claude effort settings need recalibrating for Opus 5.5.** Its default `medium` matches Opus 5 at `high`, so `xhigh` should be reserved for measured gains.
- **Cheap workers can't judge when to escalate.** Escalation triggers must be written as rules, not left to the worker.
- **Choosing a subagent's model at launch is free in cache terms.** Changing the root's model or effort mid-session is not.

The three changes with the largest expected payoff are: replace "strive for parallel execution" with criteria, sizing and hard caps; move Claude's code-mapper and library-research off Haiku; and stop workers from reading the root's session-start list and re-reading `AGENTS.md`. "Actionable recommendations" lists all eleven in priority order, and "Proposed text and configuration changes" has the text to paste.

## Scope, method and evidence quality

This review covers how a root agent in Codex and Claude Code should decide when to delegate, how to split work, what to hand workers, which model and effort each role gets, and how results are verified. It audits the recommendations given earlier in this conversation and checks them against a broader body of evidence.

The configuration reviewed is the `main` branch of [paul-heyse/library-context](https://github.com/paul-heyse/library-context) as fetched on 2026-09-30: `AGENTS.md`, `CLAUDE.md`, `.agents/roles/*`, `.codex/config.toml`, `.codex/agents/*`, `.claude/agents/*`, `.claude/settings.json` and the plan-execution skills. The local checkout at `/home/paul/library-context` was not reachable, so uncommitted changes are not reflected. The external review (`external_review_agent_workflow_design.md`) was read in full.

Sources are weighted in three tiers:

| Tier | What it is | How it is used |
| --- | --- | --- |
| 1 | Vendor documentation and vendor engineering posts (OpenAI Codex and model guides; Anthropic Claude Code docs, prompting guides and engineering posts) | Treated as authoritative for runtime mechanics and for each vendor's own models |
| 2 | Peer-reviewed or preprint research on multi-agent systems, LLM evaluation and model routing | Used for general principles; not assumed to transfer exactly to today's models |
| 3 | Practitioner reports and third-party guides | Used only as corroboration, or flagged as anecdotal |

Two limits apply throughout. Vendor guidance changes with every model release, so claims are dated. And no claim here has been measured on this repository; the calibration plan near the end says how to check the ones that matter.

## Audit of the previous recommendations

Of 16 earlier recommendations, 4 hold as stated, 6 are strengthened by the broader evidence, 3 are refined, and 3 are revised. None is withdrawn; the revisions concern effort levels and cross-model review.

| # | Earlier recommendation | Verdict | What the broader evidence says |
| --- | --- | --- | --- |
| 1 | Replace "strive for parallel execution" with delegation criteria and sizing | Strengthened | Centralized multi-agent setups gained 80.9% on parallelizable tasks and lost 39–70% on sequential ones ([Google Research](https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/)); Anthropic needed explicit scaling rules in its research system |
| 2 | Calibrate delegation per runtime: Astra under-delegates, Opus over-delegates | Refined | Holds for Astra ([OpenAI](https://developers.openai.com/api/docs/guides/latest-model/gpt-6-astra.md)). The over-delegation warning is documented for Opus 5; the Opus 5.5 guide keeps Opus 5 patterns as a starting point and stresses its long parallel-subagent runs. Keep the damper light and verify it |
| 3 | Hard concurrency caps in both runtimes | Holds | Anthropic recommends deterministic caps; the repo's single build directory and PostgreSQL set a physical ceiling |
| 4 | Don't delegate planning; avoid a role pipeline | Strengthened | Anthropic's context-centric guidance, Google's sequential penalty and Cognition's "writes stay single-threaded" all point the same way |
| 5 | Implementation review only at boundaries, not as a routine stage | Refined | Cognition's clean-context review loop finds about 2 bugs per PR, 58% severe. Review every completed package; remove only self-verification instructions |
| 6 | Move Claude's evidence roles off Haiku | Strengthened | Weaker models are poor at knowing when to escalate ([Cognition](https://cognition.com/blog/multi-agents-working)); misroutes trigger costly correction chains ([Kwartler et al.](https://arxiv.org/pdf/2609.28919)) |
| 7 | Reviewer at high effort on Sol; try Opus at medium on Claude | Holds | Opus 5.5 at `medium` matches Opus 5 at `high` on coding evaluations |
| 8 | Try Astra at medium for Codex execution sessions | Revised | OpenAI suggests starting low for explicitly set Astra subagents, but ARC Prize found higher Astra effort cheaper per solved long-horizon task. Keep `high` for the root and measure cost per completed task |
| 9 | `xhigh` for Claude planning sessions and design review | Revised | Opus 5.5 thinks more per turn than Opus 5 at the same level; Anthropic says to reserve `xhigh`/`max` for measured gains |
| 10 | Named Codex agents ignore spawn-time model overrides; document a real escalation route | Strengthened | Confirmed in the Codex docs. Add rule-based escalation triggers, because weaker workers can't reliably decide to escalate |
| 11 | Use cross-runtime review for decorrelation | Revised | Same-model clean-context review works well in production (Cognition). Self-preference bias is real but was measured on summaries. Keep cross-family review as an option for formal design reviews only |
| 12 | Scope session-start reading to the root; stop re-reading `AGENTS.md` | Strengthened | Context files raised cost over 20% without improving success ([Gloaguen et al., ICLR 2026](https://arxiv.org/abs/2602.11988v1)); OpenAI flags "read X before every edit" as a context burn |
| 13 | Return skeletons for evidence roles | Strengthened | Specification problems are the largest failure family in multi-agent traces, 41.8% ([MAST](https://arxiv.org/abs/2503.13657)) |
| 14 | Fix the dangling "Execution rhythm" reference | Holds | Astra pauses on unclear or conflicting instruction files |
| 15 | One-off bakeoff to calibrate routing | Refined | Measure cost per verified task and correction count, not tokens per turn |
| 16 | Claude Code dynamic workflows for mechanical fan-out | Holds | Still valid; the feature is a research preview |

One new finding has no earlier counterpart: Codex caps project instructions at 32 KiB *cumulatively* across global and nested `AGENTS.md` files. At 21.5 KB, the root file leaves little room for a global file or nested ones, and overflow is dropped silently.

## When to delegate, and how much

Delegate only to protect the root's context, to cover independent ground in parallel, or to get an independent judgment. Write the sizing down as rules, and calibrate them to the root model, because the two vendors' frontier models lean in opposite directions.

**Three reasons, and only three.** Anthropic found multi-agent setups beat a single agent consistently in three situations: context pollution, parallelizable work, and specialization. Outside those, coordination costs usually exceed the benefit, and multi-agent runs typically use 3–10x the tokens ([Anthropic, 2026](https://claude.com/blog/building-multi-agent-systems-when-and-how-to-use-them)).

**The task's shape decides.** Google Research tested 180 configurations across three model families. An orchestrator with workers improved parallelizable tasks by 80.9%, but every multi-agent variant degraded sequential planning tasks by 39–70% ([Google Research, 2026](https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/)). A layered cutover plan is mostly sequential; its evidence-gathering is mostly parallel.

**Readers parallelize; writers don't.** Cognition's 2026 production report says multi-agent systems work best when writes stay single-threaded and extra agents contribute intelligence rather than actions ([Cognition, 2026](https://cognition.com/blog/multi-agents-working)). Parallel writers make conflicting implicit choices about style, edge cases and patterns.

**Write sizing rules; models misjudge effort.** Anthropic's research system only behaved after it embedded rules such as one agent for simple lookups and 2–4 subagents for comparisons ([Anthropic engineering](https://www.engineering.fyi/article/how-we-built-our-multi-agent-research-system)). A disposition like "strive for parallel execution" gives the root nothing to apply.

**Calibrate per root model.**

- *GPT-6 Astra* may delegate less often than desired and responds well to explicit guidance on when and how much to delegate ([OpenAI](https://developers.openai.com/api/docs/guides/latest-model/gpt-6-astra.md)). Codex spawns subagents only when asked directly or when `AGENTS.md` or a skill requests it ([Codex docs](https://developers.openai.com/codex/concepts/subagents.md)).
- *Claude Opus 5* delegates more readily than earlier models; Anthropic recommends naming which scenarios warrant delegation and setting deterministic caps ([Anthropic](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5)). The Opus 5.5 guide keeps those patterns as a starting point ([Anthropic](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5-5)).

**Cap mechanically, not only in prose.** Claude Code allows 20 concurrent subagents by default and nests three layers deep; `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` and `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH` change that ([Claude Code docs](https://code.claude.com/docs/en/sub-agents.md)). Codex uses `agents.max_concurrent_threads_per_session`.

A root can apply these as a short checklist, in order:

1. Can I finish this in a handful of tool calls? Do it myself.
2. Will it flood my context with output I'd only summarize (search, logs, library docs, test runs)? Delegate to a read-only worker.
3. Are there several independent questions? One worker per question, usually 2–4 at once.
4. Does it write production code? One writer per package. Parallel writers only with disjoint files, settled contracts and worktrees.
5. Do I need a judgment that my own reasoning would bias? A fresh-context reviewer, given requirements and the diff but not my conclusions.

## Decomposition and agent topology

Split work along context boundaries, not job titles. Keep one hub that owns decisions and one writer per package, and put verification where it needs little context.

**Context-centric, not problem-centric.** Splitting by type of work (one agent plans, one implements, one tests, one reviews) forces a lossy handoff at every step. In Anthropic's experiment with exactly those four roles, the subagents spent more tokens coordinating than working ([Anthropic, 2026](https://claude.com/blog/building-multi-agent-systems-when-and-how-to-use-them)). Good boundaries are independent research paths, components behind a settled interface, and blackbox verification; bad ones are sequential phases of the same work, tightly coupled components, and shared state.

**A hub contains errors only if it checks.** Independent parallel agents amplified errors 17.2x in Google's study; a central orchestrator held that to 4.4x by acting as a validation bottleneck ([Google Research](https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/)). That benefit depends on the root reading decisive evidence rather than accepting completion messages.

**Map, reduce, manage.** Cognition found unstructured swarms a distraction; what works is a manager that splits work, children that execute, and a manager that synthesizes. Two failure modes recur: managers without deep codebase context write overly prescriptive briefs, and agents assume their children share their state when they don't ([Cognition, 2026](https://cognition.com/blog/multi-agents-working)).

**Permanent roles only for durable differences.** OpenAI's guidance is that custom agents should be narrow and opinionated, with a tool surface that matches the job ([Codex docs](https://developers.openai.com/codex/concepts/subagents.md)). A different tool surface, permission level or evaluation standard justifies a role; a different subsystem belongs in the brief. The external review got this right.

**Fresh subagent or fork.** A Claude Code fork inherits the whole conversation and shares the parent's prompt cache, while a named subagent starts clean from its definition and brief ([Claude Code docs](https://code.claude.com/docs/en/sub-agents.md)). Use forks for side tasks that need the root's full context, such as a second opinion on a decision in progress. Use named roles for isolated evidence work and for any review that must not inherit the root's reasoning.

**Move mechanical fan-out into code.** For many near-identical edits, Claude Code's dynamic workflows (research preview) hold the plan in a script, so the root's context holds only the result ([Claude Code docs](https://code.claude.com/docs/en/workflows)).

For library-context, this gives three lanes under the root. A read lane runs evidence questions in parallel. A write lane runs one executor per package, in layer order, with parallel writers only on disjoint crates in separate worktrees. A verification lane reviews and tests each stable package from a fresh context.

&#91;embedded content: recommended topology · 3 lanes, 7 roles\]

Every brief goes down from the root and every result comes back to it; workers never hand work to each other directly, and the read lane escalates instead of guessing.

## Briefs, return contracts and worker context

Most multi-agent failures start at the handoff, so the brief and the return format deserve more care than the role definitions. Give each worker exactly the context its task needs, and nothing it would have to wade through.

**Where failures come from.** In the MAST study of over 200 multi-agent traces, specification problems caused 41.8% of failures, inter-agent misalignment 36.9%, and verification gaps 21.3% ([Cemri et al.](https://arxiv.org/abs/2503.13657); shares from the paper's first version). Better briefs and returns address the first two directly.

**What a brief must contain.** Anthropic's research system needed four things per subagent: an objective, an output format, guidance on tools and sources, and explicit task boundaries ([summary of Anthropic's post](https://ai.plainenglish.io/how-we-built-our-multi-agent-research-system-5f5e10b2a8d6?gi=a610b8f62c64)). OpenAI adds that the prompt should say how work divides, whether to wait for all agents, and what to return ([Codex docs](https://developers.openai.com/codex/concepts/subagents.md)).

**State the shared state.** Agents assume their children know what they know ([Cognition, 2026](https://cognition.com/blog/multi-agents-working)). A brief should name the decisions already made, the baseline revision, and what sibling workers own, so two workers don't make conflicting implicit choices.

**Return distillations, with a shape.** A worker may spend tens of thousands of tokens exploring but should return a condensed summary, often 1,000–2,000 tokens ([Anthropic](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)). A fixed skeleton matters most for cheaper models: answer first, then evidence with locations, then what is observed versus inferred, then open questions.

**Load only what the task needs.**

- Claude Code subagents already load the full CLAUDE.md hierarchy, including imported `AGENTS.md` files; `omitClaudeMd: true` turns that off for workers that take everything from the brief ([Claude Code docs](https://code.claude.com/docs/en/sub-agents.md)). A public issue reports subagents missing CLAUDE.md in some setups, so verify with a probe rather than trust either.
- Context files are followed, which is exactly the cost: in an ICLR 2026 study they made agents explore and test more, raised inference cost over 20%, and did not improve success; repository overviews did not help agents find files faster ([Gloaguen et al.](https://arxiv.org/abs/2602.11988v1)). Keep non-standard rules; cut the tour.
- OpenAI's Astra guidance makes the same point: point to docs by when they apply, not "read these before every edit" ([OpenAI, 2026](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra)).
- Codex stops loading instruction files once the global and project `AGENTS.md` chain reaches 32 KiB combined, and drops the rest without a warning ([Codex issue #36371](https://github.com/openai/codex/issues/36371)).
- Agent descriptions load on every turn. Claude Code warns once they pass 15,000 tokens combined; detail belongs in the agent body, which loads only when it runs.

## Allocating model intelligence and effort

Match each role's tier to the cost of an error nobody downstream will catch, not to how simple the task looks. Choose model and effort when the worker launches, and make escalation a rule rather than the worker's call.

**Price errors by where they land.** An executor's mistake usually fails a compile or a test. A mapper's or researcher's mistake flows silently into the root's decision, and a reviewer's miss ships. In Kwartler et al.'s emulation of real Claude Code sessions, correction turns made up over a third of model spend, so their router moves work to a cheaper model only when it is at least 80% confident and never moves a correction below the tier it corrects ([Kwartler et al., 2026](https://arxiv.org/pdf/2609.28919)).

**Launch is the free routing point.** A subagent starts a fresh conversation, so choosing its model costs nothing in prompt-cache terms. Switching a running conversation's model, or changing its top-level effort, rebuilds the cache ([Kwartler et al.](https://arxiv.org/pdf/2609.28919); [Anthropic](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5-5)). Set the root once per session and route at the workers.

**What each vendor says its tiers are for.**

- OpenAI: Sol for ambiguous multi-step work that needs planning and validation; Luna for fast, narrowly scoped, clear or high-volume work; `high` effort for agents that trace complex logic, including reviewers. For explicitly configured subagents, start Luna at `high` and Astra at `low` ([Codex docs](https://developers.openai.com/codex/concepts/subagents.md)).
- Anthropic: Opus 5.5 at its default `medium` matched or beat Opus 5 at `high` on coding, and `xhigh`/`max` should be reserved for measured gains ([Anthropic](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5-5)). Sonnet 5.5 should start at `medium` for well-specified agentic work and `high` for harder or longer work ([Anthropic](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-sonnet-5-5)).

**Effort is not monotonic in cost.** On bounded work, Anthropic's runs found `medium` matched the default's accuracy at 70–87% of its cost. On long-horizon work the direction can reverse: ARC Prize found GPT-6 Astra cheaper and stronger at `max` than at `medium` because it needed fewer actions (both as reported in [Kwartler et al.](https://arxiv.org/pdf/2609.28919)). Judge effort by cost per completed task, not tokens per turn.

**Cheap workers can't be trusted to escalate.** Cognition's "smart friend" setup failed with a weaker primary because the gap showed exactly in knowing when to escalate and what to ask; it worked between frontier models ([Cognition, 2026](https://cognition.com/blog/multi-agents-working)). Anthropic's advisor tool more than doubled Haiku's score with an Opus advisor, yet it still trailed Sonnet alone by 29% ([Anthropic](https://claude.com/blog/the-advisor-strategy)). So write the escalation triggers down:

- any claim that a capability or API does not exist;
- any claim carried across versions (for example ruff crates 0.0.13 indexed, 0.0.11 linked);
- any finding that would change a dependency, an `lctx-model` contract or a §B decision;
- conflicting sources, or a worker that reports low confidence;
- a second failed fix of the same problem, which goes up a tier, never down.

**Runtime mechanics that decide what is possible.**

- Codex: a custom agent file's `model` and `model_reasoning_effort` override any value passed at spawn time ([Codex docs](https://developers.openai.com/codex/concepts/subagents.md)). To escalate, spawn the built-in `worker` or `default` agent with an explicit model and the role contract path, or keep a second agent file for the stronger tier.
- Claude Code: a per-invocation `model` parameter beats the agent's frontmatter, but there is no documented per-call effort ([Claude Code docs](https://code.claude.com/docs/en/sub-agents.md)). Escalate by passing `model`, or keep a second agent definition with a higher `effort`.

Recommended allocation for library-context, as starting points to calibrate:

| Role | Codex model / effort | Claude Code model / effort | Reasoning |
| --- | --- | --- | --- |
| Root | Astra / high; plan mode high | Opus 5.5 / medium; high for plan authoring | Owns every decision; measure before moving |
| Code mapper | Luna / high | Sonnet / medium | Lookups with citations; errors feed decisions |
| Library research | Luna / high, escalate to Sol / high | Sonnet / medium, escalate to Opus / medium | Version and absence claims shape adoption |
| Design reviewer | Sol / high (formal reviews may justify xhigh after measurement) | Opus / high; medium for focused advice | Independent architectural judgment |
| Executor | Sol / high | Sonnet / high; medium for mechanical packages | Compile checks and tests catch most errors |
| Implementation reviewer | Sol / high | Opus / medium | Last check before acceptance |
| Test agent | Sol / medium | Sonnet / medium | Running is cheap; diagnosis needs reasoning |

## Verification and independent review

Verify independently, at package boundaries, from a fresh context and against concrete criteria. Don't ask a model to double-check itself, and don't let a reviewer's output become a work queue without the root filtering it.

**Fresh context is the active ingredient.** Cognition's coding and review agents run the same model, yet the reviewer catches about 2 bugs per PR, roughly 58% of them severe, and works best when it shares no context with the coder ([Cognition, 2026](https://cognition.com/blog/multi-agents-working)). The reviewer reasons backward from the diff and avoids the long, rotting context the coder built up.

**Self-verification instructions backfire on Opus.** Anthropic says Opus 5 verifies its own work unprompted, and instructions like "use a subagent to verify" cause over-verification with no quality gain ([Anthropic](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5)). Its Fable 5 guide adds that separate fresh-context verifiers outperform self-critique ([Anthropic](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-fable-5)). The line is between reflexive re-checking (remove it) and an independent review of a stable result (keep it).

**Give verifiers concrete criteria.** The common verifier failure is declaring success after one or two checks; the fix is explicit criteria, negative tests and an instruction to run the complete relevant suite ([Anthropic, 2026](https://claude.com/blog/building-multi-agent-systems-when-and-how-to-use-them)). Verification also needs little context, which is why it splits off cleanly.

**Let the reviewer report everything; let the root filter.** Opus 5 follows "only report high-severity issues" literally and reports less, so Anthropic suggests asking for everything and filtering in a separate pass. Cognition likewise found the key was the coder filtering findings against the user's intent and scope. Your contract's "don't manufacture findings or propose style churn" is compatible with that: report all real findings with severity, and the root decides.

**Cross-family review is a targeted option.** LLM evaluators recognize and favor their own outputs ([Panickssery et al., NeurIPS 2024](https://arxiv.org/abs/2404.13076v1)), though that was measured on summaries, not code. With both runtimes configured, a Sol review of a Claude-authored design, or the reverse, is cheap insurance for formal design reviews, not a routine step.

**Deterministic checks belong in hooks.** Claude Code's guidance treats CLAUDE.md as advisory and hooks as guaranteed ([Claude Code docs](https://code.claude.com/docs/en/best-practices)). Your end-of-turn hook already moves formatting, lint and dependency checks out of the agents' hands, which is best practice.

## Parallel execution, isolation and shared resources

Parallel writers need real isolation, disjoint ownership and enough machine to run them. On one workstation with one Cargo build directory and one PostgreSQL cluster, the practical ceiling for concurrent writers is about two.

**Isolation differs by runtime.**

- Codex subagents run in the parent's workspace and inherit its sandbox; live permission overrides set during the session are reapplied to children even where an agent file says otherwise ([Codex docs](https://developers.openai.com/codex/concepts/subagents.md)). Concurrent writers need separate worktrees or `codex exec` runs.
- Claude Code subagents can set `isolation: worktree`, but the worktree branches from the default branch, not the parent session's `HEAD` ([Claude Code docs](https://code.claude.com/docs/en/sub-agents.md)). An isolated executor therefore cannot see the root's uncommitted work; commit first, or give it the files in the brief.

**Contention is the hidden cost.** Parallel executors in this workspace contend for Cargo's build-directory locks, 16 build jobs, sccache and disposable PostgreSQL databases. Two executors compiling the same crates mostly wait on each other, so parallel writes pay only when they touch different crates and different test databases.

**One owner per shared artifact.** Shared model declarations, manifests, generated code, plan files and status need a single writer, which the roles README already requires. The Cargo lockfile, `.sqlx` query data and insta snapshots belong on that list too, since parallel edits to them conflict.

**Background runs change what workers can do.** In interactive Claude Code, fork mode puts spawned subagents in the background by default, and background subagents get a reduced built-in tool set; permission prompts surface in the main session ([Claude Code docs](https://code.claude.com/docs/en/sub-agents.md)). In the Codex CLI, approval requests can arrive from inactive agent threads ([Codex docs](https://developers.openai.com/codex/concepts/subagents.md)).

**Time budgets speed up parallel work.** Opus 5.5 paces itself against a stated time budget; in Anthropic's tests, small agent teams given a budget finished sooner with comparable quality ([Anthropic](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5-5)). This is optional and needs harness support for an elapsed-time line.

## Instruction hygiene for frontier models

Frontier models follow instruction files closely, so every stale, conflicting or over-specified line now costs behavior, not just tokens. Keep files short and contextual, write prescriptive detail for the cheaper tiers that need it, and audit after every model change.

**Higher instruction sensitivity cuts both ways.** OpenAI says GPT-6 Astra is more sensitive to instructions in skills and `AGENTS.md`, that unclear or conflicting guidance may make it pause and block early, and strongly recommends auditing those files ([OpenAI](https://developers.openai.com/api/docs/guides/latest-model/gpt-6-astra.md)). It suggests asking the model to quote the instruction that made it pause, which is a cheap diagnostic.

**Recipes now hinder the strongest models.** Overly specific itineraries can hurt Astra where they once helped, and guidance that helps Sol or Luna may overconstrain Astra ([OpenAI, 2026](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra)). Put concrete procedure in the worker contracts that cheaper models read, and keep the root's guidance at the level of criteria.

**Boundaries and completion.** Strong "ask first" language written for earlier models may make Astra stop where you'd want it to continue; define completion up front instead ([OpenAI, 2026](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra)). Opus 5.5 can end an unattended turn with a progress report, so long runs need a checklist and an explicit completion condition ([Anthropic](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5-5)).

**Descriptions route; bodies instruct.** Skill and agent descriptions should be short and say when they apply, not everything they touch; Codex shortens descriptions when there are too many ([OpenAI, 2026](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra)). Claude Code routes delegation on the description field the same way ([Claude Code docs](https://code.claude.com/docs/en/sub-agents.md)).

**Minimal context files.** The ICLR 2026 study's conclusion was that human-written context files should describe only minimal requirements ([Gloaguen et al.](https://arxiv.org/abs/2602.11988v1)). Non-standard rules earn their place; repository tours and command catalogs mostly don't.

**Re-audit on model changes.** Both vendors changed delegation, verification and effort behavior between consecutive releases this year. A short audit after each root-model change is cheaper than debugging drift: ask the new root to list instructions it finds unclear, conflicting or unnecessary.

## Assessment of the current configuration

Of 14 areas, 4 are aligned with best practice, 5 are partial and 5 are gaps. The strengths are structural (topology, contracts, ownership, deterministic hygiene); the gaps are calibration and context loading, which are cheap to fix.

| Area | Current state | Rating | What best practice adds |
| --- | --- | --- | --- |
| Topology and roles | Six reusable roles; root owns design, integration and acceptance | Aligned | Matches the hub-and-spoke pattern that contains errors best |
| Shared contracts and adapters | One contract per role in `.agents/roles/`; thin Codex and Claude adapters | Aligned | Good single source; keep runtime-specific calibration in the adapters |
| Root reads decisive evidence | Roles README forbids accepting completion messages as verification | Aligned | Exactly the validation-bottleneck behavior Google measured |
| Delegation trigger | “Use subagents for review, planning and execution … strive for parallel execution” in AGENTS.md, the README and two skills | Gap | Criteria, sizing and a damper per root model; no planning delegation |
| Mechanical caps | None set in either runtime | Gap | Concurrency and depth caps sized to one workstation |
| Worker context loading | Workers told to read AGENTS.md, whose session-start list then sends them to STATUS and two plans | Gap | Root-only session start; workers work from the brief |
| AGENTS.md size and content | 21.5 KB, including a repository tour and a long command catalog | Partial | Keep non-standard rules; move the tour; stay well under the 32 KiB Codex chain budget |
| Brief contract | Outcome, scope, authorities, baseline, permitted edits, dependencies, evidence | Partial | Add decisions already made, sibling ownership and a return shape |
| Claude model allocation | Haiku for mapper and research; no explicit effort on four agents; design review at xhigh | Gap | Sonnet at medium for evidence; explicit effort everywhere; high for design review |
| Escalation | “Increase reasoning effort when warranted” | Gap | Rule-based triggers and a route each runtime actually honors |
| Deterministic hygiene | End-of-turn hook runs and repairs formatting, lint and dependency checks | Aligned | Hooks over advisory text is the documented best practice |
| Codex model allocation | Luna for evidence, Sol for the rest, Astra root | Partial | Raise implementation review to high; add a Sol escalation route |
| Parallel writes | Worktrees only for truly concurrent production edits | Partial | Name the shared-resource owners and a writer limit; note the worktree base branch |
| Calibration | Allocations labeled “starting points to calibrate” | Partial | A small, one-off measurement with a decision rule |

One small defect: `.claude/agents/implementer.md` points to an “Execution rhythm” section of AGENTS.md that does not exist; the content lives under “Testing rules”.

## Actionable recommendations

Eleven changes, in priority order. The first six are text and configuration edits of under an hour each; seven through ten are small refinements; eleven is the measurement that tells you whether the routing is right.

| # | Change | Where | Why | Done when | Status |
| --- | --- | --- | --- | --- | --- |
| 1 | Replace “strive for parallel execution” with delegation criteria, sizing and a writer limit; stop delegating planning | AGENTS.md “Agent coordination”; `.agents/roles/README.md`; `plan-execution` and `execute-plan` skills | Dispositions don't calibrate; sequential work degrades under multi-agent setups | No “strive for parallel” remains; the root can state which criterion justified each spawn |  |
| 2 | Add a per-runtime delegation calibration: encouragement for Astra in AGENTS.md, a damper for Opus in CLAUDE.md | AGENTS.md; CLAUDE.md | The two root models lean in opposite directions | Claude sessions stop spawning for small tasks; Codex sessions delegate qualifying evidence work unprompted |  |
| 3 | Set hard caps: 4 concurrent subagents, no nesting | `.claude/settings.json` env; `.codex/config.toml` `[agents]` | One build directory and one database; workers shouldn't delegate | Settings committed; a five-way fan-out request queues instead of running five |  |
| 4 | Move Claude code-mapper and library-research to Sonnet at medium; set explicit effort on every Claude agent; design review at high | `.claude/agents/*.md` frontmatter; roles README table | Evidence errors propagate into decisions; Opus 5.5 recalibrated effort | Frontmatter and the routing table agree |  |
| 5 | Make session start root-only; drop “Read AGENTS.md” from the worker contract and adapters; verify what workers receive | AGENTS.md “Start of session”; `worker.md`; `.codex/agents/*.toml`; `.claude/agents/*.md` | Workers already load it; the list sends them through plans they don't need | A probe worker quotes AGENTS.md without reading it, and opens no plan files on a lookup |  |
| 6 | Add rule-based escalation triggers and a route each runtime honors | `.agents/roles/README.md`; `library-research.md`; `code-mapper.md` | Cheap workers can't judge when to escalate; named Codex agents ignore spawn overrides | Triggers listed; README names the exact escalation mechanism per runtime |  |
| 7 | Add return skeletons to evidence and test roles | `code-mapper.md`; `library-research.md`; `test-agent.md` | Specification gaps are the largest failure family; cheaper tiers need shape | Returns lead with the answer and separate observed from inferred |  |
| 8 | Add shared state to the brief: decisions made, baseline revision, sibling ownership | `.agents/roles/README.md` “Coordinate the work” | Agents assume children share their state | Two parallel briefs never claim the same file |  |
| 9 | Raise Codex implementation review to high; review every completed package; keep reviewers reporting all real findings with severity | `.codex/agents/implementation-reviewer.toml`; README | Clean-context review is the highest-yield multi-agent pattern in coding | Each package closes with a review against a named revision |  |
| 10 | Slim AGENTS.md: move the repository tour and build-environment detail to docs; fix the “Execution rhythm” reference; lint section references | AGENTS.md; `docs/`; `implementer.md`; `scripts/check_agents.py` | Context files cost >20% with no gain beyond non-standard rules; Codex's 32 KiB chain budget | AGENTS.md under about 12 KB; Codex reports the full file loaded |  |
| 11 | Run the calibration bakeoff and adopt its decision rule | See “Calibration plan” | Every allocation above is a starting point | Allocations updated from measured results, with the date recorded |  |

Two options are worth keeping in reserve rather than adopting now. Cross-family review of formal design reviews (Sol reviewing Claude-authored designs, or the reverse) is cheap insurance when a decision is expensive to reverse. Claude Code dynamic workflows suit the mechanical consumer migrations in cutover phases 3–5, once the feature leaves research preview.

## Proposed text and configuration changes

These drafts implement recommendations 1–9. They are written to fit the repository's existing voice and to replace text rather than add to it.

**AGENTS.md: replace the “Agent coordination” section (recommendations 1 and 2).**

```text
## Agent coordination

The root agent owns design, integration and acceptance, and reads decisive evidence itself.
Delegate when it buys one of three things:
- context protection: searches, library research, logs or test output you would only summarize;
- parallel breadth: independent questions, or packages with disjoint files and settled contracts;
- independent judgment: a fresh-context design or implementation review.
When one of these holds, delegate rather than doing the work inline. Otherwise do it yourself,
including anything you can finish in a handful of tool calls.

Sizing: one evidence worker per independent question, usually 2-4 at once. One executor per
package, in plan order; at most two concurrent executors, only on disjoint crates in separate
worktrees. Review and test each completed package from a fresh context. The root writes plans;
workers supply evidence for them.

Follow the roles and coordination contract (.agents/roles/README.md) for briefs, escalation
and model routing.
```

**AGENTS.md: scope the session-start list (recommendation 5).** Rename the heading and add one line under it:

```text
## Start of session (root session only)

Subagents work from their brief and the files it names, and skip this list.
```

**CLAUDE.md: add under “Claude Code specifics” (recommendation 2).**

```text
- **Delegation on Opus:** delegate only under AGENTS.md's criteria. Don't spawn subagents to
  re-check your own work, and don't delegate what you can finish in a handful of tool calls.
```

**.claude/settings.json: add an `env` block (recommendation 3).** Depth 1 means subagents cannot spawn their own.

```json
"env": {
  "CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS": "4",
  "CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH": "1"
}
```

**.codex/config.toml: cap concurrency (recommendation 3).**

```toml
[agents]
enabled = true
default_subagent_model = "gpt-6.1-sol"
default_subagent_reasoning_effort = "high"
max_concurrent_threads_per_session = 4
```

**.agents/roles/worker.md: replace the first paragraph and add a return shape (recommendations 5 and 7).**

```text
AGENTS.md is already in your context; don't re-read it, and skip its session-start list.
Read your role contract and only the files, skills and plans your brief names.

Return, in this order: the answer in two to five sentences; the evidence (path:line or symbol,
and the revision examined); what you observed versus what you infer; open questions and
anything unfinished. Stay under about 400 words unless the brief asks for a map or a full
review text.
```

**.agents/roles/README.md: extend the brief and add an escalation section (recommendations 6 and 8).**

```text
Each brief also states the decisions already made that the worker must not revisit, the
baseline revision, and what sibling workers own.

## Escalation

Workers return to the coordinator instead of concluding when a finding claims a capability or
API is absent, carries a claim across versions, would change a dependency, an lctx-model
contract or a section-B decision, rests on conflicting sources, or follows a second failed fix.
The coordinator reruns that question at a stronger tier:
- Codex: spawn the built-in worker with model gpt-6.1-sol, effort high and the role contract
  path. Named agents ignore spawn-time model overrides.
- Claude Code: pass model opus on the call. There is no per-call effort setting.
Never rerun a failed or corrected task at a lower tier.
```

**Agent frontmatter and TOML (recommendations 4 and 9).**

| File | Change |
| --- | --- |
| `.claude/agents/code-mapper.md` | `model: sonnet`, add `effort: medium`; optionally `omitClaudeMd: true` |
| `.claude/agents/library-research.md` | `model: sonnet`, add `effort: medium` |
| `.claude/agents/design-reviewer.md` | `effort: high` (from `xhigh`) |
| `.claude/agents/implementation-reviewer.md` | `model: opus`, add `effort: medium` |
| `.claude/agents/test-agent.md` | add `effort: medium` |
| `.claude/agents/implementer.md` | keep `sonnet` / `high`; change “Execution rhythm” to “Testing rules” |
| `.codex/agents/implementation-reviewer.toml` | `model_reasoning_effort = "high"` |
| `.agents/roles/README.md` routing table | mirror every change above |

Update the routing table in the same commit as the agent files, so the two sources of truth never disagree.

## Calibration plan

Run three small comparisons once, with the decision rule written before the run, and repeat them only when a root or worker model changes. The measure is cost per verified task and the corrections it needed, not tokens per turn ([Kwartler et al.](https://arxiv.org/pdf/2609.28919)).

Keep the record in one evidence folder, `docs/design_review/evidence/YYYY-MM-DD_agent-routing/`, following the repository's existing convention. Nothing here needs a new hook, register or document type.

| Comparison | Inputs | Configurations | Scored on | Decision rule |
| --- | --- | --- | --- | --- |
| Evidence tier | 12 questions from the cutover plan with independently verified answers: 6 owner or consumer questions, 6 library questions including at least 2 absence claims and 2 version-transfer claims | Claude: Haiku vs Sonnet / medium. Codex: Luna / high vs Sol / medium | Decisive claim correct; citations resolve; escalated when a trigger applied; cost; wall time | Cheapest configuration with zero wrong decisive claims and every trigger honored |
| Review tier | 2–3 recent packages on a scratch branch, each with 3 planted defects (an invariant break, an error-path bug, a missed consumer) | Codex: Sol / medium vs Sol / high. Claude: Sonnet / high vs Opus / medium | Planted defects found; false findings; cost | Highest recall; cost decides only between equal recall |
| Codex root effort | 2 comparable execution packages | Astra / high vs Astra / medium, fixed for the whole session | Cost per completed package; corrections; rework | Keep high unless medium finishes with no extra corrections or rework |

For one week after adopting the changes, have the `handoff` skill note how many subagents each session spawned and any worker output that turned out wrong. That is enough to see whether the delegation criteria and the Opus damper are calibrated. Drop the note after the week.

## Sources

All accessed 2026-09-30. “Opened” means the full page was read; “excerpt” means only search-result text was used, for a claim that matched other sources.

| Source | Tier | Read |
| --- | --- | --- |
| [OpenAI: Codex subagents (concepts and custom agents)](https://developers.openai.com/codex/concepts/subagents.md) | 1 | Opened |
| [OpenAI: Using GPT-6 (Astra prompting guide)](https://developers.openai.com/api/docs/guides/latest-model/gpt-6-astra.md) | 1 | Opened |
| [OpenAI: Rethinking skills and prompts for GPT-6 Astra](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra) | 1 | Opened |
| [OpenAI Codex issue #36371: project\_doc\_max\_bytes is cumulative](https://github.com/openai/codex/issues/36371) | 1 | Excerpt |
| [Anthropic: Create custom subagents (Claude Code)](https://code.claude.com/docs/en/sub-agents.md) | 1 | Opened |
| [Anthropic: Best practices for Claude Code](https://code.claude.com/docs/en/best-practices) | 1 | Opened |
| [Anthropic: Dynamic workflows (Claude Code)](https://code.claude.com/docs/en/workflows) | 1 | Excerpt |
| [Anthropic: Prompting Claude Opus 5](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5) | 1 | Opened |
| [Anthropic: Prompting Claude Opus 5.5](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5-5) | 1 | Opened |
| [Anthropic: Prompting Claude Sonnet 5.5](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-sonnet-5-5) | 1 | Excerpt |
| [Anthropic: Prompting Claude Fable 5](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-fable-5) | 1 | Excerpt |
| [Anthropic: When to use multi-agent systems (and when not to)](https://claude.com/blog/building-multi-agent-systems-when-and-how-to-use-them) | 1 | Opened |
| [Anthropic: The advisor strategy](https://claude.com/blog/the-advisor-strategy) | 1 | Excerpt |
| [Anthropic: Effective context engineering for AI agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) | 1 | Excerpt |
| [Summary of Anthropic's multi-agent research system post](https://ai.plainenglish.io/how-we-built-our-multi-agent-research-system-5f5e10b2a8d6?gi=a610b8f62c64) | 3 | Excerpt |
| [Google Research: Towards a science of scaling agent systems](https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/) | 2 | Opened |
| [Cemri et al.: Why Do Multi-Agent LLM Systems Fail? (MAST)](https://arxiv.org/abs/2503.13657) | 2 | Excerpt |
| [Gloaguen et al.: Evaluating AGENTS.md (ICLR 2026)](https://arxiv.org/abs/2602.11988v1) | 2 | Excerpt |
| [Panickssery et al.: LLM Evaluators Recognize and Favor Their Own Generations (NeurIPS 2024)](https://arxiv.org/abs/2404.13076v1) | 2 | Excerpt |
| [Kwartler et al.: Harness Tokenomics (2026 preprint)](https://arxiv.org/pdf/2609.28919) | 2 | Opened |
| [Cognition: Multi-Agents: What's Actually Working (April 2026)](https://cognition.com/blog/multi-agents-working) | 3 | Opened |
| [Cognition: Don't Build Multi-Agents (2025)](https://cognition.com/blog/dont-build-multi-agents) | 3 | Excerpt |
| [paul-heyse/library-context](https://github.com/paul-heyse/library-context), `main` branch | Subject | Opened |

Kwartler et al. and Cognition are counted below vendor documentation because one is an emulation-based preprint and the other a vendor's own production report. Their findings are used where they agree with tier 1 sources or explain a mechanism, not as measured results for this repository.
