# Agent workflow design
Yes. With **GPT-6 Astra / high as the root**, I would also slightly simplify the research side: rather than keeping a generic `docs_researcher` alongside the new role, I would make **`library_research` the broad external technical-research agent**. It can use your library-reference skills, Context7, GitHub, upstream docs, registries, release notes, issue trackers, and web sources as appropriate.

That keeps the stable topology to seven agents. OpenAI's current guidance is to add distinct agents when they have genuinely different ownership, instructions, tool surfaces, or approval policies, rather than creating lots of narrowly named roles. :chatgpt-content-reference{index="1"}

## Updated topology

```text
┌─────────────────────────────────────────────────────────────────────┐
│                      ROOT / COORDINATOR                             │
│                       GPT-6 ASTRA / xHIGH                            │
│                                                                     │
│   User intent • decomposition • synthesis • architectural choice   │
│   delegation • coordination • integration • final acceptance       │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
             evidence / independent reasoning as needed
                                │
        ┌───────────────────────┼─────────────────────────┐
        │                       │                         │
        ▼                       ▼                         ▼
┌─────────────────┐   ┌──────────────────────┐   ┌───────────────────┐
│   CODE MAPPER   │   │   LIBRARY RESEARCH   │   │   DESIGN REVIEW   │
│ GPT-6 Luna/HIGH │   │   GPT-6 Luna/HIGH    │   │ GPT-6.1 Sol/HIGH  │
│    read-only    │   │      read-only       │   │     read-only     │
│                 │   │                      │   │                   │
│ Internal repo   │   │ External ecosystem   │   │ Architecture      │
│ evidence        │   │ knowledge            │   │ & domain model    │
└────────┬────────┘   └──────────┬───────────┘   └─────────┬─────────┘
         │                       │                         │
         └───────────────────────┼─────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      ROOT / GPT-6 ASTRA xHIGH                        │
│                                                                     │
│        Reconcile evidence • choose direction • structure work       │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                 one or more execution assignments
                                │
               ┌────────────────┼────────────────┐
               ▼                ▼                ▼
       ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
       │   EXECUTOR   │  │   EXECUTOR   │  │   EXECUTOR   │
       │ Sol 6.1/HIGH │  │ Sol 6.1/HIGH │  │ Sol 6.1/HIGH │
       │              │  │              │  │              │
       │ Task-specific│  │ Task-specific│  │ Task-specific│
       │ specialization│ │ specialization│ │ specialization│
       └───────┬──────┘  └───────┬──────┘  └───────┬──────┘
               │                 │                 │
               └─────────────────┼─────────────────┘
                                 │
                                 ▼
                        completed implementation
                                 │
                  ┌──────────────┴──────────────┐
                  ▼                             ▼
         ┌─────────────────┐          ┌─────────────────┐
         │    REVIEWER     │          │    TEST AGENT   │
         │ Sol 6.1/MEDIUM  │          │ Sol 6.1/MEDIUM  │
         │    read-only    │          │ test / inspect  │
         │                 │          │                 │
         │ Semantic/static │          │ Empirical       │
         │ verification    │          │ verification    │
         └────────┬────────┘          └────────┬────────┘
                  │                            │
                  └──────────────┬─────────────┘
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      ROOT / GPT-6 ASTRA xHIGH                        │
│                                                                     │
│      Adjudicate findings • further delegation • final acceptance    │
└─────────────────────────────────────────────────────────────────────┘
```

GPT-6 Astra is currently OpenAI's highest-intelligence model and supports `high`, `xhigh`, and `max` reasoning in addition to lower settings, so `gpt-6-astra` / `high` is a valid root configuration. OpenAI positions GPT-6.1 Sol as the balanced near-Astra option and Luna as the fast/high-volume model, which fits this division well. :chatgpt-content-reference{index="2"}

## Agent roster

| Agent | Model | Effort | Fundamental responsibility |
|---|---|---:|---|
| **Root** | GPT-6 Astra | **High** | Global reasoning, synthesis, orchestration and acceptance |
| **Code Mapper** | GPT-6 Luna | **High** | Obtain precise evidence from the current codebase |
| **Library Research** | GPT-6 Luna | **High** | Obtain authoritative external technical/library knowledge |
| **Design Review** | GPT-6.1 Sol | **High** | Independent architecture/domain-model reasoning |
| **Executor** | GPT-6.1 Sol | **High** | Production implementation |
| **Reviewer** | GPT-6.1 Sol | **Medium** | Independent semantic/correctness review |
| **Test Agent** | GPT-6.1 Sol | **Medium** | Empirical validation, diagnostics and failure analysis |

The central division is:

```text
                ASTRA
          understand + decide
                 │
     ┌───────────┼───────────┐
     │                       │
    Luna                    Sol
 acquire/compress      reason/build/verify
 information
```

That is the architecture I would optimize around.

---

# 1. Root / coordinator

**Model:** `gpt-6-astra`  
**Effort:** `high`

This should remain the owner of the overall problem.

Its responsibility is not merely delegation. It should retain:

- understanding the user's real objective;
- determining what is known versus uncertain;
- deciding whether investigation is needed;
- determining when architecture warrants independent review;
- choosing between direct work, single-agent delegation, sequential delegation, or parallel delegation;
- synthesizing evidence from different sources;
- making cross-cutting architectural decisions;
- deciding how implementation should be decomposed;
- reconciling contradictions between agents;
- interpreting reviewer/test findings;
- deciding whether further work is necessary;
- accepting the resulting system.

This is exactly where Astra's additional intelligence is most valuable. Current OpenAI guidance describes Astra as the model for the most demanding reasoning, coding and professional workflows, while explicitly recommending simpler prompts and less accumulated scaffolding for it. :chatgpt-content-reference{index="3"}

### What it should *not* do by default

The root should avoid spending substantial context on things other agents can compress effectively:

```text
large grep/search traversals
hundreds of compiler diagnostics
entire third-party repositories
long dependency documentation
release-history archaeology
raw benchmark output
```

It should still inspect evidence itself whenever the judgment depends on details that cannot safely be compressed.

### Conceptual instruction

```text
Own the problem globally.

Use subordinate agents when separate context, independent judgment,
specialized tools, or parallel work materially improves the outcome.

Retain responsibility for synthesis, architectural decisions,
coordination, integration, and final acceptance.

Delegate work, not ownership of the user's intent.
```

That is about as prescriptive as I would make it.

---

# 2. Code Mapper

**Model:** GPT-6 Luna  
**Effort:** High  
**Default mode:** Read-only

Its question is:

> **What does our system actually do?**

This is your internal evidence-acquisition specialist.

Typical assignments:

- find the canonical representation of a concept;
- trace a data flow;
- identify where an invariant is enforced;
- determine all consumers of an API;
- trace type conversion across layers;
- identify ownership of a subsystem;
- map control flow around a feature;
- locate relevant tests;
- establish how state is persisted/hydrated;
- determine whether two abstractions duplicate one another;
- understand how an existing feature is implemented.

It should use whatever repository-analysis capabilities are available: normal search, rust-analyzer, semantic search, compiler metadata, Git history, etc.

### It should return evidence, not prose

Ideally:

```text
Conclusion

Relevant symbols/types

Execution/data path

Files establishing this

Known invariants

Important dependencies

Remaining uncertainty
```

rather than twenty paragraphs describing its exploration process.

### Boundary

The mapper answers:

> **How does this repository work?**

It does **not** answer:

> How should we architect it?

That's root/design-review territory.

---

# 3. Library Research

**Model:** GPT-6 Luna  
**Effort:** High  
**Default mode:** Read-only

This is the significant new role.

Its question is:

> **What does the external software/library ecosystem actually provide, and how should we understand it accurately?**

I would make this significantly broader than a “documentation researcher.”

### Its source/tool hierarchy

Depending on the question, it should be able to use:

```text
Library/reference skills
        ↓
Context7
        ↓
official/upstream documentation
        ↓
GitHub source
        ↓
GitHub issues / PRs / releases
        ↓
crate/package registry metadata
        ↓
examples / benchmarks / changelogs
        ↓
broader web research where useful
```

Not as an inflexible ordering. The important principle is to choose the source that actually answers the question.

For example:

**API semantics**

```text
Context7 / official docs
        +
upstream source when ambiguity remains
```

**Undocumented behavior**

```text
GitHub source
        +
tests
        +
issues/PR discussion if relevant
```

**Library selection**

```text
reference skills
+ upstream repos
+ release/activity history
+ docs
+ benchmark evidence
+ relevant ecosystem alternatives
```

**Breaking changes**

```text
changelog/release
+ source diff
+ migration docs
+ PRs/issues
```

### This is also where your library-reference skills belong

OpenAI currently describes skills as exactly this kind of reusable workflow layer around tools: skills encode how and when information sources/tools should be combined, while MCP provides the actual live capabilities. :chatgpt-content-reference{index="4"}

So I would have:

```text
library_research agent
        │
        ├── library-reference skills
        ├── Context7 MCP
        ├── GitHub
        ├── documentation MCPs
        └── general search / upstream sources
```

rather than stuffing all library-research methodology into `AGENTS.md`.

### Typical assignments

Given the kinds of engineering questions you've been working on:

```text
Determine whether rustworkx-core exposes algorithm X
and whether it accepts petgraph-compatible representations.

Compare current petgraph graph serialization approaches
and identify practical rehydration options.

Determine the breaking API changes from Ruff 0.x used
by current Pyrefly to latest Ruff crates.

Investigate DataFusion's current optimizer interfaces
relevant to our expression architecture.

Find mature Rust libraries implementing algorithm X,
including maintenance state and integration constraints.
```

### Return contract

I'd have it return something like:

```text
Answer / recommendation-relevant findings

Candidate libraries or APIs

Exact capabilities and limitations

Current versions / maintenance state where relevant

Integration implications

Source references

Uncertainties or conflicting evidence
```

The **root** then decides what to do with this information.

---

# 4. Design Review

**Model:** GPT-6.1 Sol  
**Effort:** High  
**Default mode:** Read-only

Its question is:

> **Is this the right conceptual architecture?**

This is not a generic reviewer and not a planning agent.

Use it when the problem touches things such as:

- domain representation;
- semantic sources of truth;
- schemas/types/relationships;
- major ownership boundaries;
- architectural invariants;
- persistence models;
- public or foundational internal interfaces;
- control/data-flow architecture;
- concurrency models;
- extensibility mechanisms;
- major refactors;
- cross-cutting abstractions.

### It should reason independently

A good assignment would be:

```text
Here is the intended capability.
Here is the relevant code/evidence.
Here is the proposed direction.

Evaluate the architecture independently.
```

Not:

```text
Confirm that this design is correct.
```

Its purpose is partly **decorrelated reasoning**.

### Output

```text
Assessment of the conceptual model

Relevant architectural constraints

Invariants that must hold

Issues with the proposed design

Material alternatives

Consequences/tradeoffs

Recommended direction
```

The Astra root decides whether to adopt that recommendation.

---

# 5. Executor

**Model:** GPT-6.1 Sol  
**Effort:** High  
**Mode:** Write-enabled

Its question is:

> **How do I make this objective real in the codebase?**

The crucial thing from our previous exchange is that this role should remain deliberately **general**.

Do **not** encode:

```text
always split work by module
always split database from graph layer
always have a separate tests executor
...
```

The Astra root should determine the execution topology dynamically.

One executor may receive:

```text
implement this entire feature
```

or several executors might receive independent work partitioned by:

- subsystem;
- feature slice;
- domain concept;
- API boundary;
- implementation variant;
- algorithm;
- migration cohort;
- platform;
- independent technical concern;
- or some decomposition we did not anticipate.

Those are **examples**, not policy.

### Why Sol/high here

Implementation is itself a reasoning task.

The executor should be capable of discovering:

- the design cannot be represented cleanly as assumed;
- an existing abstraction already solves part of the problem;
- a new invariant appears during implementation;
- the task crosses a hidden boundary;
- a proposed representation generates downstream semantic problems;
- a locally obvious implementation would damage the domain model.

It should have permission to surface those discoveries.

### Boundary

The executor has **local implementation agency**, but not unilateral authority to redefine major architecture.

Conceptually:

```text
Follow the established intent and material constraints.

Make implementation decisions autonomously.

If implementation evidence materially challenges an architectural
assumption, surface the contradiction rather than hiding it behind
a workaround.
```

---

# 6. Reviewer

**Model:** GPT-6.1 Sol  
**Effort:** Medium  
**Default mode:** Read-only

Its question is:

> **What is wrong with the implementation we now have?**

This is independent post-implementation intellectual review.

Focus on:

- behavioral correctness;
- regressions;
- violated invariants;
- mismatch with intended design;
- incomplete implementation;
- incorrect edge cases;
- lifecycle problems;
- error semantics;
- ownership/coupling issues introduced accidentally;
- concurrency correctness where relevant;
- insufficient tests for significant behavior.

I would specifically instruct it **not to manufacture findings**.

Its return structure should be finding-driven:

```text
Finding
Severity/materiality
Evidence
Consequence
Suggested direction
```

If there are no meaningful findings, it should say so.

---

# 7. Test Agent

**Model:** GPT-6.1 Sol  
**Effort:** Medium  
**Mode:** Execute/test + inspect

Its question is:

> **What happens when we actually exercise the system?**

This distinction from the reviewer is important.

Reviewer:

```text
reason about the code
```

Test agent:

```text
interrogate the actual system
```

Typical responsibilities:

- targeted tests;
- workspace tests;
- compilation;
- Clippy/lints;
- doctests;
- feature-matrix checks;
- integration tests;
- regression tests;
- failure reproduction;
- failure clustering;
- identifying first causal failure;
- deciding whether a failure is related or incidental;
- examining logs/traces;
- benchmarking where validation requires it.

The test agent's real value is **context compression**.

Instead of feeding Astra:

```text
8,000 lines of cargo output
```

it returns:

```text
Three tests fail.

Two are downstream manifestations of the first.

Root cause:
FooId is persisted before canonicalization.

Relevant failure:
...

Likely affected implementation:
...
```

The root or executor gets the actionable semantic information.

---

# Why I would merge `docs_researcher` into Library Research

With the new agent, these would overlap heavily:

```text
docs_researcher
library_research
```

I'd use:

```text
library_research
    ├── API documentation
    ├── source code
    ├── Context7
    ├── GitHub
    ├── skills/reference material
    ├── releases
    ├── issues
    └── ecosystem comparison
```

A separate `docs_researcher` becomes warranted only if it later has a **materially distinct tool surface or workflow**, for example if you wanted one agent restricted to authoritative documentation for compliance-sensitive API questions.

OpenAI's present guidance is consistent with this: start with the smallest focused set of agents and split only when separate ownership, instructions, tools or controls actually justify it. :chatgpt-content-reference{index="5"}

---

# Optional agents, not part of the base topology

I would explicitly leave room for these without defining them today.

### Performance investigator

Potentially valuable if profiling/benchmark work becomes recurrent.

Distinctive behavior:

```text
measure before modifying
profile
form hypothesis
change
remeasure
report statistical evidence
```

That is enough of a different execution discipline to potentially deserve its own agent.

### Migration specialist

Could warrant a permanent role if schema/domain representation migrations become common:

```text
compatibility
backfills
dual representations
round-trip preservation
rollback
migration completeness
```

### Security reviewer

Potentially appropriate when work crosses actual trust/security boundaries. Its threat-model-driven evaluation criteria differ enough from generic code review to justify specialization.

### GPU / compute specialist

Relevant if your local CUDA/vLLM work becomes a recurring development domain and requires a dedicated tool/runtime profile.

### Browser / integration tester

Potentially worthwhile for applications with substantial UI or browser-visible behavior.

The rule I'd keep is:

> **Promote a task specialization into a permanent agent only when it repeatedly requires meaningfully different reasoning, tools, constraints, or evaluation criteria.**

Otherwise, make it a specialized invocation of `executor`, `reviewer`, `test_agent`, or `library_research`.

---

# Recommended project structure

```text
AGENTS.md

.codex/
├── config.toml
└── agents/
    ├── code-mapper.toml
    ├── library-research.toml
    ├── design-review.toml
    ├── executor.toml
    ├── reviewer.toml
    └── test-agent.toml
```

The root is the normal Codex session rather than another subagent definition.

Conceptually:

```toml
model = "gpt-6-astra"
model_reasoning_effort = "high"

[agents]
enabled = true
max_concurrent_threads_per_session = 6
```

The official managed multi-agent infrastructure currently defaults to six concurrent subagents, excluding the coordinator, so six is also a reasonable ceiling rather than an instruction to actually keep six workers occupied. :chatgpt-content-reference{index="6"}

Each named role then explicitly specifies its own model/effort rather than relying on fallback behavior.

---

## The overall information architecture becomes quite elegant

```text
                         ┌───────────────┐
                         │     USER      │
                         └───────┬───────┘
                                 ▼
                  ┌──────────────────────────┐
                  │    GPT-6 ASTRA / HIGH    │
                  │                          │
                  │ understand • decide      │
                  │ decompose • synthesize   │
                  │ coordinate • accept      │
                  └────────────┬─────────────┘
                               │
              ┌────────────────┼────────────────┐
              │                                 │
              ▼                                 ▼
       INTERNAL EVIDENCE                  EXTERNAL EVIDENCE
       Code Mapper                        Library Research
       Luna / high                        Luna / high
              │                                 │
              └────────────────┬────────────────┘
                               │
                        evidence substrate
                               │
                         ┌─────┴─────┐
                         ▼           ▼
                       ROOT       DESIGN REVIEW
                       Astra       Sol / high
                         │           │
                         └─────┬─────┘
                               ▼
                          decision
                               │
                               ▼
                          EXECUTOR(S)
                          Sol / high
                               │
                               ▼
                    ┌──────────┴──────────┐
                    ▼                     ▼
                 REVIEWER             TEST AGENT
                Sol / medium          Sol / medium
                    │                     │
                    └──────────┬──────────┘
                               ▼
                              ROOT
                               │
                         final judgment
```

The deepest distinction is therefore not really seven job titles. It is **four cognitive functions**:

```text
ACQUIRE
Code Mapper + Library Research

REASON
Astra Root + Design Review

ACT
Executor

VERIFY
Reviewer + Test Agent
```

And Astra sits above all four as the agent preserving the complete problem representation.

That strikes me as the right use of a much more expensive/high-intelligence root: **don't give Astra every token; give Astra every important decision.** :chatgpt-content-reference{index="7"}



# Execution decomposition

> Parallelize execution when the work admits useful independent progress with sufficiently clear boundaries and manageable coordination costs. Choose the decomposition appropriate to the task rather than applying a fixed partitioning scheme.

The earlier “module A / persistence / query layer” split should therefore be treated only as one illustrative possibility.

### Execution can be decomposed in many different ways

| Possible decomposition | Example | When it may fit |
|---|---|---|
| **Subsystem / component** | graph model vs persistence vs query engine | Components have reasonably independent ownership |
| **Vertical feature slice** | executor owns one feature end-to-end | Feature cohesion matters more than architectural layers |
| **Domain concept** | symbol identity vs call relationships vs type relations | Domain concepts have strong internal semantics |
| **Change type** | schema migration vs consumer adaptation | Changes involve distinct technical mechanisms |
| **Interface boundary** | producer side vs consumer side of a new API | Contract can be agreed first |
| **Platform / backend** | PostgreSQL implementation vs in-memory implementation | Multiple implementations share an abstraction |
| **Algorithm family** | graph construction vs community detection vs FCA | Algorithms are largely independent |
| **Migration cohort** | executor handles one set of existing callers | Large mechanical-but-semantic migrations |
| **Validation ownership** | implementation plus its directly associated tests | Tests are tightly coupled to the behavior being added |
| **Risk specialty** | concurrency-sensitive portion separated from ordinary changes | One part requires substantially different reasoning |
| **Toolchain specialty** | proc macros/build system/CUDA/SQL/etc. | Work requires unusual tooling or knowledge |
| **Independent solution exploration** | two executors prototype different viable implementations | Design uncertainty remains and comparison is valuable |

And sometimes the correct choice is simply:

```text
one executor owns the entire implementation
```

A complicated change does **not** automatically become better because it has been parallelized.

## I would give the root agent discretion

The execution section of `AGENTS.md` could be much less prescriptive:

```markdown
### Execution

Once the intended behavior and material architectural constraints are
sufficiently understood, use `executor` agents where delegation provides
meaningful benefit.

The primary agent should determine whether implementation is best handled by:
- a single executor,
- multiple parallel executors,
- sequential executors,
- or direct implementation by the primary agent.

When using multiple executors, choose boundaries appropriate to the structure
of the work. Possible decompositions include subsystem ownership, vertical
feature slices, domain concepts, interfaces, implementation variants,
migration cohorts, specialized technical concerns, or other naturally
independent workstreams.

These are examples, not prescribed decomposition strategies.

Provide each executor enough context to understand its objective, relevant
constraints, dependencies, and expected outcome. Avoid unnecessary
coordination overhead or overlapping work where it would undermine the value
of delegation.

The primary agent remains responsible for reconciling execution results and
for deciding when parallelism is appropriate.
```

That is much closer to what I think you want: **give Sol/high the operating principles, not the solution to every decomposition problem.**

---

## Specialist executors should have a fairly high bar

I also wouldn't start creating things like:

```text
database_executor
graph_executor
rust_executor
migration_executor
performance_executor
```

just because those categories exist in the codebase.

A dedicated execution specialist becomes worthwhile when several things line up:

1. **The work recurs.**  
   There is a recognizable class of tasks rather than a one-off problem.

2. **Its optimal instructions materially differ from the generic executor.**  
   If the only difference is “work on PostgreSQL,” the spawn prompt can handle that.

3. **It benefits from a distinct tool/environment configuration.**  
   For example, a browser-testing agent, GPU-performance agent, or database migration agent might need different tooling.

4. **It has specialized invariants or evaluation criteria.**  
   For example, a schema-migration specialist might consistently need to reason about backwards compatibility, dual reads/writes, rollback, and data preservation.

5. **Specialization reduces repeated context loading.**  
   If every task requires explaining the same unusual framework, conventions, or procedures, encoding those once may be worthwhile.

6. **The specialization improves routing.**  
   The root can clearly recognize when that role should and should not be invoked.

7. **It is sufficiently independent to justify its own context window.**

If those aren't true, I'd keep the generic `executor` and put the specialization in the **task prompt**.

That leads to an important distinction:

```text
Permanent agent role
    = durable difference in operating behavior

Spawn-time assignment
    = task-specific specialization
```

For example, you don't necessarily need a permanent `postgres_executor.toml`.

The root can simply spawn the normal executor with:

```text
Implement the PostgreSQL persistence portion of this change.

Pay particular attention to transactional semantics, round-trip identity,
schema compatibility, and hydration behavior...
```

The executor is temporarily acting as the PostgreSQL specialist, without adding another permanent agent type.

---

## Where permanent specialists make more sense

There are some stronger candidates.

A **performance executor**, for example, could be justified if it routinely receives benchmark tooling, profiling instructions, regression thresholds, and a rule against accepting unmeasured “optimizations.”

A **migration executor** could make sense if your codebase regularly undergoes large representation/schema migrations and you have a repeatable compatibility methodology.

A **GPU/CUDA executor** could warrant specialization if it has a substantially different toolchain and validation regime than normal Rust development.

Likewise, a **database migration specialist** could become valuable if database evolution becomes sufficiently central to your application that it repeatedly needs the same safety and validation discipline.

But these should emerge from actual recurring workload.

---

## I would therefore make the topology deliberately asymmetric

The support roles are relatively stable:

```text
code_mapper
docs_researcher
design_review
reviewer
test_agent
```

because those agents perform **fundamentally different kinds of work**.

Execution is more polymorphic:

```text
                          executor
                             │
             task-specific specialization
                             │
       ┌────────────┬────────┼───────────┬──────────┐
       │            │        │           │          │
     domain      subsystem  migration  algorithm  feature
      slice        slice     cohort      slice     slice
```

You can then introduce a permanent specialized executor only when repeated experience demonstrates that one branch deserves its own durable behavior.

That is preferable to trying to predict the complete execution-agent taxonomy upfront.

### So I would change the earlier rule

Rather than:

> Parallelize by coherent responsibility, not by mechanical file changes.

I'd use the less prescriptive:

> **Partition execution according to the natural independence structure of the task. Favor decompositions that permit meaningful autonomous progress without excessive shared-state coordination, but do not assume any particular decomposition strategy or that parallel execution is inherently preferable.**

And perhaps:

> **Agent specialization should follow durable differences in reasoning, tools, constraints, or evaluation criteria rather than merely mirroring codebase components.**

Those two principles give a capable root model considerably more room to choose the right execution strategy while still communicating what makes delegation useful.