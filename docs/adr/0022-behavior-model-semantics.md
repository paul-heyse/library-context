---
id: ADR-0022
title: The behavior model: a stated runtime flow abstraction, closed conditions, five verdicts, and models as data
status: proposed
date: 2026-09-24
supersedes: []
superseded-by: null
design: [§B5, §B10, §3.2, §3.9, §9, §9.9]
evidence: Proposed
revisit: The Stage 2.1 spike decides the flow-IR provider (record it here as an amendment), or the structured evaluation finds a question class that the closed condition language cannot express.
---

## Context

ADR-0021 makes the behavioral model the product. The deep review of the pivot found four gaps its
semantics must close:
- **F3:** no verdict lattice, so negative answers have no completeness premise.
- **F5:** every ready-made flow provider carries a type checker's view that would be relabelled
  as runtime flow. Pyrefly drops statically decided branches (§B1); `ty_python_core` 0.0.14
  decides `TYPE_CHECKING` as true (review probe).
- **F6:** the pivot's "compatible behavior context" is, in general, satisfiability, and §B10
  excludes a constraint solver.
- **F4:** the channels that carry FastMCP's behavior (`self` fields, the settings singleton,
  ContextVars, function-object attributes, framework registries) are absent from the pivot's
  relation list.

## Options

1. **The simpler alternative: extend Pass B's recognizers** (more predicate shapes, more value
   classes), with no IR. Rejected. On the pilot, 15,364 of 32,075 mapped arguments are computed
   expressions and only 79 of 516 raising `if`s are accepted guards. Recognizers over syntax
   cannot follow values through assignments, fields or exceptions.
2. **Adopt a provider's IR as the model** (Pyrefly's binding graph, or ty's use-def map).
   Rejected as the *model*, and allowed as a *provider*. Both are built for type inference;
   relabelling them breaks §B5.
3. **Our own stated runtime abstraction, whichever provider builds it, with closed conditions, five
   verdicts and models as data.** Chosen.

## Decision

- **The flow IR is our stated runtime abstraction (§B5).**
  - A statement-level control flow with exceptional exits.
  - Reaching definitions over **places**. A place is a local name; `self.f` or `self.f.g`
    (k ≤ 2); a module global, settings singletons included; a ContextVar object; or an attribute
    of a function object.
  - **The runtime view of static branches:** `TYPE_CHECKING` is false; `sys.version_info` and
    `sys.platform` tests follow the analyzed context.
  - **The provider** is decided by the Stage 2.1 spike: `ty_python_core` 0.0.14 behind a range
    parity rule, or our own builder. It is recorded here by amendment. Pyrefly's binding graph is
    a parity oracle only.
- **Conditions are data in a closed language.**
  - **Atoms over places:** `is None`, `is not None`, `== literal`, `in {literals}`, truthiness,
    `isinstance(C)`. Anything else is an **opaque** atom.
  - **Normal form:** sorted conjuncts of normalized atoms. Two conditions are equal when their
    normal forms are equal. That is syntactic, not semantic, equivalence (DM-15).
  - **Compatibility of two conditions** is decided by a finite-domain evaluator as
    **compatible**, **incompatible** or **unknown**. It is unknown whenever an opaque atom
    decides.
  - This is not a constraint solver (§B10 unchanged). The Rust evaluator and its Python twin are
    held to one shared known-answer corpus.
- **Five verdicts, never a null.** Codebook `verdict`, append-only:

  | Verdict | When |
  |---|---|
  | `established` | No boundary in the region |
  | `conditional` | Established under a stated condition |
  | `refuted_under_model` | **Only** in a `complete_under_stated_model` region with no named boundary; a rule rejects any other |
  | `unknown` | A `boundary_reason` is named |
  | `not_analyzed` | Out of scope, not requested, or cut by a budget |

  Discovery results (FCA, communities, vectors) carry no verdict. They are `statistically_derived`
  nominations and never write membership.
- **Meaning comes from models, propagation from summaries.**
  - Effects and roles of stdlib, dependency and framework callables are committed data
    (`crates/cpg-schema/models/*.toml`, origin `synthetic_model`, append-only ids), digested into
    `compiler_digest`.
  - Transfer summaries carry them bottom-up over the call graph's SCCs, with argument
    substitution, conditions and exception handling. A call alone never propagates a capability.
- **Materialized at compile time.** Every predicate, summary and concept membership is computed in
  the compile, in DataFusion or in `lctx-analytics` kernels, before publication. The server never
  re-implements predicate semantics; only the condition evaluator has a twin, held to the shared
  corpus.

## Consequences

- **Negative answers become possible, but narrower:** "never read" is `unknown` wherever a dynamic
  access (`getattr` by string, `importlib`) could reach the value.
- **More to maintain:** a flow family, a verdict codebook, an effect codebook, models files, and
  two fixtures (`flow_shapes`, `behavior_shapes`).
- **If D-2 chooses `ty`:** a second ruff line (0.0.14) enters the lockfile under a declared family
  exception (the ADR-0002 amendment), with salsa pinned exactly at 0.28.2.
- **Revisit:** at the spike's outcome, or when the structured evaluation needs a condition the
  closed language cannot express.
