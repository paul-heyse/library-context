---
id: ADR-0045
title: Behavioral model semantics and proof basis
status: accepted
date: 2026-09-25
supersedes: [ADR-0022, ADR-0027, ADR-0029, ADR-0030, ADR-0031, ADR-0032, ADR-0033, ADR-0034, ADR-0035, ADR-0037, ADR-0039]
superseded-by: null
design: [§B5, §B6, §B10, §3.2, §3.4.1, §3.9, §4.0, §9, §9.9]
evidence: Tested
revisit: A registered question needs a condition the closed language cannot express; a ty upgrade breaks two-way parity or decides a test at index time differently from runtime; a pinned model assertion, handler or finalizer rule meets a runtime counterexample; or a finite summary needs a proof that an ordered witness sequence cannot represent.
---

## Context

ADR-0021 makes an evidence-carrying behavioral model of a pinned library's whole public surface
the product. Its answers must be exhaustive where the analysis is complete and a named `unknown`
where it stopped, so the model needs a stated semantics that no provider can silently redefine.
Four forces shape it:
- **Providers carry checker views.** Pyrefly drops statically decided branches (§B1) and ty
  decides `TYPE_CHECKING` true at index time; relabelling either as runtime flow breaks §B5.
- **Negative answers need premises.** Without a verdict lattice and explicit completeness
  premises, silence reads as "none".
- **Compatibility is satisfiability**, and §B10 excludes a general constraint solver.
- **Absence and containment are not execution.** A transfer rule, a matching class, a syntactic
  `except` or `finally`, or a true return region does not by itself prove that a call returned, a
  handler caught, or a frame completed.

This record restates, by responsibility, the decisions in force from ADR-0022 (the surviving
clauses), ADR-0027, ADR-0029, ADR-0030, ADR-0031, ADR-0032, ADR-0033, ADR-0034, ADR-0035, ADR-0037
(which replaced ADR-0036) and ADR-0039 (which replaced ADR-0038). It adds no new decision. The
clause ADR-0027 replaced (a raise guard from source-text handler names, a hard-coded builtin
exception tree and a `suppress(` substring) is not restated. Items ADR-0022 itself marked
Proposed stay Proposed here. Architecture: [§3.9](../design/sections/behavior-model.md) and
[§9.9](../design/sections/behavioral-analysis.md).

## Options

1. **Extend syntactic recognizers, with no flow IR.** Rejected: on the pilot (2026-09-24), 15,364
   of 32,075 mapped arguments were computed expressions and 79 of 516 raising `if`s were accepted
   guards; recognizers cannot follow values through assignments, fields or exceptions.
2. **Adopt a provider's IR (Pyrefly's binding graph or ty's use-def map) as the model.** Rejected
   as the model; allowed as a provider. Both are built for type inference.
3. **Fill gaps by inference from silence or shape:** treat a transfer rule plus no modeled
   exception as normal return, a positive class match as a catch, MRO non-membership as a
   nonmatch, any pass-free finalizer as harmless, text names as class identity, a display DNF as
   the condition, or a raw return fact as a summary's key. Rejected throughout: each promotes an
   unknown into a claim or merges distinct evidence.
4. **A stated runtime abstraction, closed conditions, five verdicts, models as pinned typed data,
   and positive results only from explicit, cited, reconstructible witnesses.** Chosen.

## Decision

### Stated runtime model and flow provider

- The flow IR is our stated **runtime** abstraction (§B5): reaching definitions over places with
  their conditions, statement reachability and value sources. `ty_python_core` 0.0.14 builds its
  raw material inside `cpg-flow` only; byte ranges, place text and our condition data cross that
  boundary, and no ruff 0.0.14 or ty type leaves it (ADR-0012). Pyrefly's binding graph is a parity
  oracle only.
- ty's output is normalized (loop-header definitions to `loop_carried` body bindings; import alias
  targets to the bound name; augmented targets are use and definition) and checked by two-way
  parity with `references` and `bindings`, with a declared residue. ty's exception model is part
  of the stated model; ambient exceptions are outside it. Regions are relative to their scope's
  entry. Panics abort extraction.
- **Runtime view:** `TYPE_CHECKING` is false. A same-length sentinel rename stops ty deciding by
  spelling (a module already using the sentinel is refused); our evaluator decides only names our
  lexical resolution binds to `typing`/`typing_extensions.TYPE_CHECKING` or the imported `typing`
  module. `sys.version_info` (full tuple ordering, prefixes of at most three fields),
  `sys.platform` and `os.name` follow the analysis context, only through resolved stdlib roots.
  The CPG layers keep the checker view; a behavior takes the runtime view at its site, a pruned
  but runtime-reached site is `outside_provider_model`, and a runtime-unreachable seed
  declaration is `runtime_unreachable`.
- **Places** have two layers: the spelled place (scope-relative text, a label) and the resolved
  join key (`Parameter`, `Local`, `Field[C.f]`, `Global[module.name]`, then at most two segments),
  whose written form is §9.9's access-path grammar. A module's own global shadows a same-named
  submodule; that import order is assumed. ContextVar and function-object-attribute places, and
  Pyrefly-typed receivers for dynamic access, are **Proposed**.
- **Materialized, one implementation.** Predicates, summaries and memberships are computed by the
  pinned Rust implementation in the compile, before publication. The server never re-implements a
  semantic decision; a serve-time semantic selection or condition filter needs its own decision
  (ADR-0025 proposes the bounded native executor).

### Conditions and their persisted authority

- Conditions are data in a closed language, with no Python twin. Atoms (`condition_atom`,
  append-only) keep the Python operator (`is` vs `==`, membership, truthiness, `isinstance`),
  literals are `None`/`True`/`False`, decimal integers and JSON strings, and any other test is
  `opaque` with its normalized source text. An opaque condition is still stated (`conditional`).
- **An atom is an evaluation.** Each source test is identified by its evaluation site (module key
  over path and content, byte span) or provider predicate; text is a label. Atoms at different
  sites stay independent until an effect-stability proof exists; no reaching-definition set,
  type or spelling substitutes for it.
- **Lowering:** follow only true/false edges; ty's ambiguous terminal reads as `true` because
  verdicts state may-behavior, and the row records `approximated`; evaluate the runtime view
  first; simplify by contradiction removal, self-subsuming resolution and absorption. Calls are
  assumed to return; suppression, non-empty-iterable and finally-path predicates are opaque.
  Fates are stated on the normal path (guards factored out by `given`).
- **Raise escape.** `raise_sites.escapes` is set only for an explicit raise outside any `try` or
  `with` body of its function; `false` means unknown, never caught. Resolved L2 fates may later
  admit narrower escapes with cited class, frame and exit evidence.
- **Persisted authority.** Recomposed analysis conditions are persisted as their own validated
  structural catalog (`analysis_conditions`, `analysis_condition_nodes`), separate from provider
  `conditions`, with references naming their catalog. Publication and native load reconstruct and
  hydrate every catalog through one shared structural validator. Display DNF is never parsed to
  answer a condition question. A missing or over-budget diagram is unknown, never false.
  Approximation stays row-local because Boolean identity does not encode it. A condition created
  by a later pass needs its own validated extension.
- **Scope of this record.** Stage 2 accepted a DNF form (16 conjunctions of 8 literals) with a
  syntactic id; the implementation now identifies conditions by structural diagram roots and keeps
  the DNF as a bounded display. The general allowance behind that change (§B10's bounded
  decision-diagram kernel, its operations and budgets, and the typed primitive theory) is
  ADR-0024, which remains proposed; this record accepts only the persisted catalog above and the
  predecessor screen below.

### Verdicts and negative premises

- Five verdicts, never a null (`verdict`, append-only): `established`, `conditional`,
  `refuted_under_model`, `unknown` (with a named `boundary_reason`, `budget_reached` included) and
  `not_analyzed` (out of scope or not requested only).
- **Approximation direction.** A positive verdict states may-behavior: the model admits it
  without crossing a boundary; it is not a witness that a concrete execution exists. A negative
  verdict needs a complete may-analysis and an explicit premise. Execution checks admission: an
  observed execution the model does not admit is a counterexample; a finite pass is evidence.
- **Premises** live in `negative_premises` per place kind (parameter/local; field by name across
  the release; module global; forward chain), each also requiring flow IR for the modules read,
  runtime reachability, a non-abstract body and no release override. External readers are outside
  the model. Rules reject a refutation without its premise, on an overridden method, or on a
  field or setting whose name some attribute load spells.
- **Boundaries:** dynamic access (the getattr family with a non-literal name, `vars`, `__dict__`,
  `importlib`, `exec`/`eval`) reaches places by the stated receiver rules; `override_dispatch` for
  candidate arcs; `call_transfer` for a value reached only through a call (a transfer, never a
  flow); `abstract_body`; `runtime_unreachable`. Verdict, modality and evidence status are three
  vocabularies.
- **Identity:** the flow provider is part of the extractor's producer revision, and a change to
  its output bumps `EXTRACTOR_OUTPUT_VERSION`. Registry concept ids are `H("concept", key)`; labels
  and definitions are digested data.

### Models as pinned typed data

- Effects, transfers, callbacks, resources and exceptions of stdlib, dependency and framework
  callables are committed data (`crates/cpg-schema/models/`), append-only ids, origin
  `synthetic_model`, digested into `compiler_digest` and the extractor's producer identity.
  Authored from pinned source and official docs, never from the gold under `.claude/skills/`.
- A model binds only to an exactly pinned context definition; an unreferenced model is dormant; a
  referenced unresolved target, formal or class fails before publication. Each application to a
  source call is candidate-local and keeps target and model modality and open dispatch.
- **Object-method formals.** A positional formal shifts past the receiver only when the Pysa
  target says `true_with_object_receiver` **and** the Ruff callee is an attribute expression; every
  pinned signature must select the same explicit argument, with no unpacking. Class receivers,
  unsupported callees and disagreement stay unknown.

### Exception classes, MRO and handlers

- The extractor retains class definitions named by committed exception rules in context modules
  it already describes; the compiler binds every source and conversion class to a unique pinned
  class, never by name. A class in an unvisited module is a compile failure, not a text match.
- Class relationships come from Pyrefly's pinned MRO per retained class (`context_class_mro`,
  with an empty/cyclic marker). A handler is a `pinned_ancestor` candidate only with a cited MRO
  row; a missing, unbound or cyclic relation is `class_relation_unknown`; MRO non-membership is not
  a nonmatch.
- A modeled raise composes with a handler only on a **bounded direct path**: a complete ancestry
  walk, the first provable matching clause of a direct function-body `try` with no possible
  earlier clause, a sole `return None` action, no inner `try`/`with`, no `finally`. The result is
  a candidate-local conditional path, not an operation-level catch or return; absence is not
  evidence of escape.

### Finalizers

- A pending return is admitted through `finally` frames only when every pending frame's entire
  direct `finalbody` is one literal `pass`. The pass facts are reconstructed in inner-to-outer
  execution order and cited as ordered proof steps of every admitted summary; the single-frame
  status fields keep their one-frame meaning. `with`, any other finalizer and capped ancestry stay
  unknown; widening needs an ordered exit witness per frame. This is a local normal-exit proof,
  not proof that the return expression or an earlier call completes.

### Normal completion

- A model may carry an optional `normal_return` assertion (default false = unknown): the target
  itself returns normally after its arguments are evaluated, under the pinned model. It is
  accepted only for a function target with complete exception coverage and no exception rule,
  carried on the validated `model_targets` row, and never inferred from a transfer rule or from
  absent exceptions. The assertions in force are `typing.cast` and `typing.assert_type`, whose
  pinned CPython 3.14.7 bodies directly return their argument. A source call must also prove
  exact endpoints, target selection, modality, path condition and enclosing exit before a
  positive flow uses it; it never turns an unknown into a negative.

### Proof identity

- A finite summary has a canonical content-derived `summary_id` over callable, input/output
  paths, transfer kind, condition, exit and its ordered typed proof `(step kind, evidence id,
  step condition)`; `summary_flow_steps` stores the steps. A changed sequence changes the id; the
  id is never a graph index or a display label. A step kind is append-only and needs a checked
  source relation, a deterministic encoding, a publisher and shared reconstruction before it can
  create positives. `summary_boundaries` names raw paths without a completed proof. A true
  multi-parent proof requires a new decision.

### Predecessor screening

- The direct finite producer ignores an earlier same-function source call only when the call's
  narrowest ty statement region and the return-value condition are both present, the call region
  is not approximated, and their bounded conjunction is false. Every other earlier call withholds
  the proof, leaving a `summary_boundaries` row. This is path incompatibility, not completion, and
  an interim screen that a cited predecessor normal-outcome relation replaces rather than
  accumulates beside.

## Consequences

- Negative answers are possible but narrow: any dynamic access, open override, unresolved frame,
  missing witness or cap yields `unknown`. Positive paths are few and cited, and each widening
  (handler shape, finalizer kind, model family, proof step) needs its own witness and shared
  reconstruction.
- More to maintain: the `flow` family and its parity rules, several codebooks, the model catalog,
  condition catalogs and the proof-step relation; catalog edits move producer identity.
- Implementation evidence is focused: each clause is **Tested** on positive and withholding
  fixtures with shared publication equality (2026-09-24/25); the Stage 2 parts passed the
  integrated gate at Stage 2's exit. Integrated Stage 3 acceptance and a fresh pilot are
  `not_run`.
- Open defects and choices are owned by the
  [forward plan §6](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition):
  native proof-kind admission rejects the finalizer step (W1); qualified/aliased builtin access
  yields false no-read premises (W4); summary inputs and typed refusal causes (W5); kernel
  decisions, support and aggregate hydration budgets (W6); cyclic reach (W7); membership and
  integer equality (W11); the recursion engine (W12) and SCC routine (W13). Accepting this record
  closes none of them.
