# C4/C5 witnessed guard substitution and whole-call composition — bounded review

Retired source citations below are historical paths within this review's recorded baseline
and dated inspection scope, including any working-tree limitations. They do not point to
replacement owners. Recover committed source through [Git history](../../README.md#historical-recovery);
the findings and their original evidence strength remain unchanged.

## 1. Scope, outcome and coverage

**Change / conformance; Revise, 2026-09-29.** Fresh-context `design-reviewer` source review,
compressed template slots 1, 5 (the four requested scenarios), 6, 7, 8, 10–12;
[core 3.0, code-intelligence profile 1.1, template and binding](../design_principles/standard.toml).
Authority: [ADR-0085](../../adr/0085-typed-semantic-domain.md),
[ADR-0089](../../adr/0089-stage-contributions-and-input-closure.md),
[DESIGN §15.4–§15.9](../../design/sections/semantic-model.md#section-15-6) and the
[cutover plan §4.1.1 rows C4/C5](../../plans/semantic-model-cutover-plan_2026-09-29.md#411-detailed-remaining-execution-order).
Source findings under test: [core review C05, C06, C07](design_review_cutover-core_2026-09-29.md#C05).

| Field | Value |
|---|---|
| Subject | Commits `9705345` (C4) and `b58fc93` (C5) on `main`, clean tree: [`composition.rs`](../../../crates/lctx-model/src/domain/composition.rs), [`conditions/stability.rs`](../../../crates/lctx-model/src/domain/conditions/stability.rs), [`conditions/rebase.rs`](../../../crates/lctx-model/src/domain/conditions/rebase.rs), `Predicate::BoundGuard` in [`value.rs`](../../../crates/lctx-model/src/domain/value.rs); tests `domain_composition` (`crates/lctx-model/tests/domain_composition.rs`), [`domain_stability`](../../../crates/lctx-model/tests/domain_stability.rs), fixtures `crates/lctx-model/tests/fixtures/composition.rs`, [`stability.rs`](../../../crates/lctx-model/tests/fixtures/stability.rs), PG18 `domain_composition` (`crates/lctx-postgres/tests/domain_composition.rs`), `domain_stability` (`crates/lctx-postgres/tests/domain_stability.rs`) |
| Adjacent code read | `calls.rs` (binder, `SiteTargets`, policies), `place_composition.rs`, `transfer.rs`, `declarations.rs`, `flow.rs`, `obligation.rs` (verdict), `derivation.rs`, `charged.rs`, `occurrence_owner.rs`; legacy `cpg-flow/src/lib.rs` reaching emission; `ty_python_core` 0.0.14 `builder.rs` |
| Tier · purpose | Change · conformance (binding: bounded slice). Does not certify the enclosing §15 architecture |
| Supported scope | Pure kernels `compose_call`/`compose_site`/`substitute_call_guards`; stored invariants `witnessed_guard_substitution`, `call_composition_frames`, amended `invoked_guard_origins`; PG18 round trip. **Excluded:** P3 stored bindings, ownership and policy views; the P4 summary engine; the verdict function (C09) |
| Scenarios | (1) P4 SCC summary fixpoint; (2) P2 producers: ty_flow witnesses, Pyrefly/Pysa targets, signatures and links; (3) wider substitutable predicates and receiver-field guards; (4) budgets on a large library |
| Method | Source read at `b58fc93`, 2026-09-29. No check executed; receipts in §10 are historical |

**What holds.** The kernels are pure, bounded and store-free to test. Paths extend only through
identity (C05(a); derived caller, derived callee and identity facade are known answers). The whole
`BoundCall` is used. Return, field, global, raise/yield (`UnsupportedControlFlow`) and default
(`DefaultUnavailable`) outputs have explicit outcomes, and a non-owning caller is refused. The
condition is caller ∧ restated callee. Callee-local guards stay opaque `InvokedGuard`s. Formal
guards need a witness or refuse: `ConditionTransferUnsupported`, or `NotRequested` when no flow was
requested, never true. Kernel budget hits become obligations. Each (caller alternative × admitted
target × variant × callee alternative) composes separately. `Selection` is typed and connected to
its alternative (C07). `BoundGuard`/`GuardSubstitution`/`StabilityWitness`/`CallCompositionStep`
carry typed premises into the derivation index.

## 5. Change scenarios

| Scenario | Owner and route | Observed propagation | Evidence |
|---|---|---|---|
| 1 SCC fixpoint over `compose_site` | P4 engine over `compose_call`, sequential composition (absent), `merge` | The engine must supply `summary_admitted`, `unique_variant` and `site_owner` itself (F04). It must know an undeclared port precondition (F02) and reconstruct obligation subjects (F07). Recursive sites mint a fresh atom per iteration, so no fixpoint (F06). Composed alternatives never merge into one invocation | Implemented; engine Proposed |
| 2 ty_flow witnesses; Pyrefly/Pysa targets and links | P2 producers → stored checks | Cross-provider joins go through occurrence identity and declaration links, which is sound. The witness rule exists only inside the validator (F08). Nested-scope rebinding has no typed reach (F01). Links from several variants of one symbol are resolved first-match (F03) | Implemented; Interface-checked (ty 0.0.14) |
| 3 wider predicate set; receiver-field guards | `substitutable`, `StabilityBasis`, `GuardSubstitution`, `root_bindings` | `substitutable` is one owner, but it is independent of the basis (F08). Receiver guards need a second root map and a relation change: `argument: Id<CallArgument>` cannot name a receiver actual (F03, F08) | Implemented |
| 4 budget on a large library | kernel caps, `ResourceBudget` | Substitution work, lineage depth and BDD limits become obligations. Catalog inputs can be `ChargedMap`s through `Deref`. One uncharged catalog-sized copy is made per call (F09) | Implemented |

## 6. Correctness and fidelity gates

| Gate | Verdict | Evidence or scope reason |
|---|---|---|
| G1 Authority | pass | One typed relation per concept. `substitutable`, `compose_kinds`, `Modality::weakest` and `Approximation::join` are shared by the kernel and the stored check. The design-level duplication is judged under A2 |
| G2 Semantic fidelity | **fail** | F02: a formal's variable place is treated as the caller's entry value. F03: a bound receiver or variant silently yields nothing |
| G3 Validity | **fail** | F05(a): an erased callee condition validates. F05(b) is declared unenforceable until P3 stores bindings |
| G4 Hidden behavior | pass | Pure functions; no ambient reads |
| G5 Consistency and recovery | pass | Limits become obligations, never flows. A cyclic re-derivation fails the global acyclicity check observably |
| G6 Transformation | **fail** | F02. The path rule itself passes |
| G7 Truthful claims | pass | Plan receipts describe what the tests exercise. F01 notes that "witness" relies on an undeclared producer convention |
| G8 Library leverage | pass | §8 |
| CI-G1 Fidelity | **fail (contract level; nothing published)** | F02 spurious caller flows. F04: a potential or non-admitted target can yield a Candidate or Definite flow. F03 silent loss |
| CI-G2 / CI-G3 | n.a. / pass | No served claims. The fixtures are repository Python, not gold references |

## 7. Findings

Severity: **High** means claimed-behavior fidelity. **Medium** means a contract gap with a concrete
consequence in a planned scenario. **Low** means extension or cost. Nothing blocks the store or
P1 work. F01–F03 and F05(a) block closing C05/C06 at X0.

<a id="F01"></a>
**F01 · Medium · owner `domain::flow` vocabulary + `conditions::stability`; producer P2 ty_flow.**
Nested-scope rebinding has no typed reaching form, so the witness's soundness rests on an
undeclared producer convention.
- *Evidence.*
  - `StabilityCheck::witness` (stability.rs:81–109) requires exactly one reaching observation per (use, context), bound to a `Parameter` definition. `ReachingDefinition` is `Bound | Unbound` (flow.rs:41–45).
  - ty 0.0.14 synthesizes a `NestedBindings` definition for a `nonlocal` write in a nested scope. The definition keeps prior bindings and is never shadowed (builder.rs:1909–1972).
  - Legacy cpg-flow turns that definition into a definitionless reach (lib.rs:669, 792–799). The pinned ty-flow skill recipe says to drop these markers (`use-def-map.md:33`).
  - [Behavior model](../../design/sections/behavior-model.md) states that reaching sets do not prove stability, because "a nested call can rebind a `nonlocal`". The retired parameter-identity certificate withheld identity for nonlocal mutation ([behavioral analysis](../../design/sections/behavioral-analysis.md)).
- *Consequence.* Take `def select_timeout(timeout=None): def reset(): nonlocal timeout; timeout = None` followed by `reset(); if timeout is None: …`.
  - The P2 producer may drop the marker, or relabel it `Bound`/`Unbound`. `Unbound` is semantically false and only coincidentally refuses.
  - If it drops or relabels it `Bound`, the read has one parameter reach and the witness validates. The caller-side `t is None` then misstates the branch condition.
  - A typed theory evaluating a literal actual would refute a feasible flow.
- *Correction.*
  - Append a typed reach for nested and lazy bindings, for example `ReachingDefinition::Nested` (code 2).
  - State in the witness contract that any non-parameter reach refuses.
  - Declare frame write-through (`f_locals`, PEP 667) as `DynamicAccess`, outside the model.
- *Closure.* A stored fixture with a nested reach refuses the witness, and P2 ty_flow conformance on this shape emits it.

<a id="F02"></a>
**F02 · High · owner `composition.rs` + a §15.6 amendment.** Formal- and receiver-rooted callee
places are mapped to the caller's actual as if they denoted the entry value. Guards on the same
places require a witness.
- *Evidence.*
  - `map_output` maps every formal-rooted output through its binding (composition.rs:120–128). The input side does the same (:177–192). An element projection whose remaining path is empty maps to the whole actual (:223–230).
  - A formal's place is the variable shared by every definition and use (§15.4). The stability fixture roots the parameter definition and its read at the same Formal place.
  - The same "does this formal still hold the caller's value?" question needs a witness for guards (rebase.rs:129–137), but is assumed for transfers.
- *Consequence.* These compositions produce spurious caller-owned flows. They are Definite at a unique site and Established when the condition is true:
  - `def f(t, v): t = []; t.append(v)` at `f(a, x)` gives `x → a[*]`.
  - `def f(t, v): t = v` gives `x → a`.
  - `def f(**kw): kw['k'] = v` at `f(k=c)` gives `x → c`.
  - The `items = items or []; items.append(v)` idiom becomes unconditional.
- *Correction.*
  1. Whole-formal and whole-aggregate-element outputs are slot writes, not ports. They compose to `Disjoint`.
  2. A formal- or receiver-rooted end of a callee transfer composes only with entry-value evidence: the `StabilityWitness` basis per access, carried by the callee frame. Otherwise it yields a typed obligation. An alternative is an explicit entry-value root that only witnessed accesses may use.
  3. Record in §15.6 that callee transfers are port summaries.
  - A minimum scoped alternative: state the precondition in §15.6 and `CalleeFrame`, apply rule 1 now, and defer enforcement to P4 with trigger "first producer of callee transfers".
- *Closure.* Known answers for the four shapes: `Disjoint` or an obligation, never a flow. `update(t, k, v)` and `Config.__init__` keep their current answers.

<a id="F03"></a>
**F03 · Medium · owner `composition.rs`.** Root resolution is not total over the bound signature.
- *Evidence.*
  - `parameter()` returns the first link for an occurrence (:87–89). Links are one-to-one only *within* a signature (declarations.rs:104), so variants can share occurrences.
  - A receiver root maps only through `BindingKind::Receiver` (:109–110). `C.m(obj, v)` binds `obj` positionally (C2 `Receiver::None`).
  - An empty binding list makes `compose_call` return `[]`, not a transfer, obligation or `Disjoint` (:120–128, :216).
  - An unmatched actual gives `Disjoint`, the same outcome as a legitimately different actual (:182).
  - `root_bindings` (:133–150) is a second root map. It covers `CallArgument` only, excludes the receiver and lets the last link win.
  - The frame's `destination` is not checked against `target.destination` (:161–169). The unresolved-target test relies on this.
- *Consequence.* These flows vanish without a reason (CI-04):
  - `C.m(obj, v)` with `self.x = v` rooted at `Receiver` composes to nothing.
  - A two-variant callee with shared parameter occurrences composes to `Disjoint` through the wrong variant.
- *Correction.*
  - Build one total `map_root` over the links of `bound.signature()` (exactly one per root).
  - Map `Receiver` through the callable's first-parameter binding of any kind. Alternatively, §15.4 forbids `Receiver` roots for source callables.
  - Treat zero bindings for a declared root as an error.
  - The guard substituter consumes the same map, admitting a single whole actual.
  - Check `destination.id() == target.destination`.
- *Closure.* `C.m(obj, v)` gives `obj.x`. Shared-occurrence variants compose through the bound variant. A foreign link is refused. No empty result vector remains.

<a id="F04"></a>
**F04 · Medium · owner `composition.rs` with `calls::SiteTargets`, the binder and `OwnerTable`.**
Site admission, variant uniqueness and caller ownership enter as caller-supplied assertions. The
target's own qualification is ignored.
- *Evidence.* `CallFrame.summary_admitted`/`unique_variant` and `CallerFrame.site_owner` (:37–41), and modality/approximation (:207–209). `compose_site` passes frames through unchanged (:281–288). The stored check bounds modality only by caller and callee, and keeps only context and condition per qualification (:306, :342–343).
- *Consequence.* The claim strength of every composed flow is decided by booleans the P4 engine computes. Neither the kernel nor the store can detect these errors:
  - per-provider rather than per-site admission (the C04 concern);
  - a `Potential` target composed as `Candidate` (CI-02).
- *Correction.*
  - An admission token constructed only by `SiteTargets::admitted(Summary)`.
  - Uniqueness derived by `compose_site` from its frames per target, or one `SiteBinding` builder over all variants.
  - Ownership looked up in `OwnerTable`.
  - Target modality and approximation joined into the result.
  - The stored check adds target modality now, and policy and ownership at P3.
- *Closure.* Controls: a `Potential` target gives a `Potential` result, cross-provider disagreement gives `Candidate`, and admission cannot be forged.

<a id="F05"></a>
**F05 · Medium · owner `CompositionCheck`, `StabilityCheck`.** The stored checks enforce premise
frames and atom vocabulary, not the composition equation.
- *Evidence.*
  - `CompositionCheck::step` (:324–353) checks that composed atoms ⊆ caller atoms ∪ atoms evaluated at the site. It does not check the composed condition, the composed input/output roots or the `bindings` digest.
  - `StabilityCheck::substitution` (:110–126) accepts any argument of the site, not the one bound to the witnessed formal, and does not check that the site targets the guard's callable.
  - Rows are public structs, so store validation is the enforcement boundary.
- *Consequence.* A composed alternative with condition `true` validates and publishes. That is the callee guard erased, the core C06(i) failure. A substitution over the wrong argument also validates.
- *Correction.*
  - (a) Now: recompute caller ∧ restated(callee) from stored rows (the unique `InvokedGuard`/`BoundGuard{source}` atom at the site per callee atom) and compare condition IDs. Check the input root and path rule against the caller input, and the output root class. Add an erased-condition mutation.
  - (b) At P3, once bindings, ownership and policy views are stored: check argument ↔ formal ↔ target, the bindings digest, ownership and admission.

<a id="F06"></a>
**F06 · Medium (P4 barrier; Proposed) · owner §15.6/§15.7 decision, then `rebase.rs` and the P4 engine.**
Composition through a recursive site cannot reach a fixpoint.
- *Evidence.*
  - `substitute_call_guards` mints `InvokedGuard{source}` at the site for every callee atom, including atoms instantiated at the same site in the previous iteration (rebase.rs:164–166). This is by design ([guard-rebase review](design_review_semantic-guard-rebase_2026-09-29.md)).
  - The chain is bounded only by depth 32 and work 4096.
  - `Subsumed` catches only a result equal to a direct premise (:239). Global acyclicity (derivation.rs:43–64) refuses re-derivation through itself.
- *Consequence.*
  - Consider `def f(x): if flag(): return f(x)` then `return x`. Each iteration adds a deeper atom and a new alternative, and the process ends in `SummaryDepthLimit` (Unknown).
  - Two recursive sites double the alternatives per level until `SummaryPairWorkLimit`.
  - Every recursive function with a local guard ends by budget, not at a fixpoint. Without first-derivation dedup, the engine publishes cyclic premises.
- *Correction.* Decide SCC-internal instantiation before designing the P4 engine. For example: instantiate once per (site, root source atom), with a sound widening that eliminates deeper instances and conjoins a positive-only opaque recursion atom. The flow stays conditional without false contradictions. The engine records first derivations only.
- *Closure.* Known answers: one- and two-site recursion converge to conditional summaries without budget obligations, and the derivation index stays acyclic.

<a id="F07"></a>
**F07 · Low · owner `composition.rs`.** Results carry neither obligation subjects nor the implied
influence.
- *Evidence.* `CallComposition::Obligation(ObligationKind)` has no subject or domain (:60, §15.8). `compose_site` flattens results across triples (:284–286). No `ControlInfluence` is emitted for a `BoundGuard`; the fixture builds it by hand (fixtures/composition.rs:99–102).
- *Consequence.* The P4 engine cannot attribute or discharge obligations from `compose_site`'s output, and restates the operand→influence rule.
- *Correction.* Per-triple results carrying a typed obligation subject and domain (the refused atom, formal or root). Emit the influence (actual place → `BoundGuard` at the site) with the records.
- *Closure.* The kernel-emitted influence yields the fixture's `Selection`.

<a id="F08"></a>
**F08 · Low · owner `conditions::stability`.** Extension locality for scenarios 2 and 3.
- *Evidence.* The witness rule exists only in the private `StabilityCheck::witness`. `substitutable` (stability.rs:38) ignores `StabilityBasis`. `GuardSubstitution.argument: Id<CallArgument>` cannot name a receiver actual.
- *Consequence.* The ty_flow producer would restate the rule, or emit candidates that fail the whole generation. Editing `substitutable` alone to admit `Truthy`/`IsInstance` would accept mutation-sensitive predicates under a basis that proves only binding stability.
- *Correction.* A pure `StabilityWitness::derive` shared by the producer and the check. `StabilityBasis::admits(&Predicate)`, with the stored check using `row.basis`. The substitution references a stored binding at P3.

<a id="F09"></a>
**F09 · Low · owner `composition.rs`.** Every `compose_call` clones the whole segment and literal
catalog outside the budget (:188–191). This is O(catalog) per triple per SCC iteration.
*Correction:* an overlay lookup for the few projection rows.

**Observations** (not findings):
- **O1.** `verdict()` has no modality input (obligation.rs:192–214), so a Candidate flow with condition `true` maps to Established. Route to X0's C09 re-confirmation.
- **O2.** Witness coverage is not tied to the reaching observation's provider (stability.rs:102–106). This matters with a second Flow provider.
- **O3.** A closure formal of an enclosing function yields `NoSourceDeclaration` (:106), a misleading reason.
- **O4.** `reaching_count` spans providers, so agreeing duplicates would refuse. This is conservative.

**Assessment against core findings.** This review does not own their status; see plan §8.
- **C05(a):** met.
- **C05(b):** met for the complete binding set, return, field, global, raise/yield and default. Not met for formal ports (F02) or totality (F03).
- **C06:** kernel behavior met. The witness premise (F01) and the stored equation (F05) remain.
- **C07:** contract met. Influence derivation is unowned (F07).

## 8. Library fit and total complexity

| Capability | Choice | Reason |
|---|---|---|
| Condition AND, bounded substitution | Existing kernel over biodivine-lib-bdd | Already qualified (core review §8); no change |
| Call-frame and root composition, witness rule | Bespoke domain code | No library models Python binding, ports or stability. The code is small, bounded and owned |
| SCC schedule and fixpoint (P4) | petgraph SCC plus own driver, Proposed | A generic fixpoint engine would not converge while each iteration mints new atoms (F06). The widening is domain semantics |

No wrapper, crate or framework is warranted. F03 and F08 are function-level consolidations.

## 10. Verification and uncertainty

| Claim | Label, date | Outcome |
|---|---|---|
| C4 focused suites: `cargo test --release -p lctx-model` (incl. `domain_stability`, `domain_guard_rebase`); `-p lctx-postgres --test domain_stability --test domain_guard_rebase` | Tested, **historical author receipt** 2026-09-29 (plan §4.2) | passed as recorded; **not_run** here |
| C5 focused suites: `cargo test --release -p lctx-model` (incl. `domain_composition`); `-p lctx-postgres --test domain_composition --test domain_stability` | Tested, historical author receipt 2026-09-29 | passed as recorded; **not_run** here |
| F02, F03, F05, F09 | Implemented code read, 2026-09-29 | Direct from source; not executed |
| F01 | Interface-checked: ty 0.0.14 `builder.rs`, cpg-flow `lib.rs`, 2026-09-29 | The failing path depends on the future P2 mapping |
| F06 | Proposed: reasoning over an unbuilt engine | Settle with the recursion known answers |

Independent controls should be pre-written answers, like the existing twenty. They should not be
derived from the kernel.

## 11–12. Dispositions, judgments and decision

Disposition owner: the plan owner enters these in
[plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition). Until then, this review holds them:
- **Before X0:** F01 (vocabulary and control), F02, F03, F05(a).
- **Before P3/P4 producers:** F04.
- **Deferred:** F05(b) to P3 (stored bindings, ownership and policy views); F06 to the P4 engine design (trigger: before the SCC engine is designed); F07–F09 with P4.

| Judgment | Verdict | Scenario evidence |
|---|---|---|
| A1 Localize change | **unresolved** | Kernels are pure and testable without a store. Scenario 1 pushes admission, uniqueness, ownership (F04), port preconditions (F02) and obligation attribution (F07) into the P4 engine as hidden knowledge |
| A2 Encode meaning structurally | **violated (bounded)** | "Does a callee formal denote the caller's value?" is answered with a witness for guards and assumed for transfers (F02). Two root maps exist (F03). The site-modality decision arrives as booleans (F04). The witness rule has no producer-facing owner (F08). Typed premises and shared primitives are sound |
| A3 Extend through composition | **unresolved** | Per-alternative composition holds. Scenario 1 needs sequential composition and an SCC widening decision (F06). Scenario 3 needs basis-keyed eligibility and a receiver-capable substitution (F08, F03) |

**Bounded change decision: Revise.** G2, G6 and CI-G1 fail on claimed composition behavior (F02,
F03), and G3 fails (F05(a)). The corrections are local to `composition.rs`, `stability.rs`, the
`flow` vocabulary and a §15.6 port clause. The minimum scoped alternative for F02 is described in
F02.

**Enclosing architecture:** not certified. §15.6 needs the port contract (F02), SCC instantiation
semantics (F06) and obligation subjects (F07) before P4. X0 must reassess against P3/P4 scenarios.
Review acceptance is not release qualification.

**Next step.** The plan owner applies F02 rule 1, F03, F05(a) and F01's vocabulary with its
control, then records the F02 port clause in §15.6.

## Re-inspection of C5r (`9912598`), 2026-09-29

Scope: F01, F02, F03 and F05(a), and the target-modality part of F04, each judged against the
closure criteria above. Source was re-read at `9912598`: `composition.rs` (`Ports`, `map_output`,
`compose_call`, `compose_site`, `CompositionCheck`), `value.rs` `PlaceRoot::Entry`, `flow.rs`
`ReachingDefinition::Nested`, `stability.rs`, `rebase.rs` `local_root`, and the §15.4/§15.6
amendment and plan §8 rows.

**Reviewer-executed checks, 2026-09-29.** The tree was `9912598` plus concurrent uncommitted C6
work: additive `obligation.rs` budget/discharge code and deletions of legacy modules. The
composition and stability sources, tests and fixtures were unchanged from the commit.
- `python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test domain_composition --test domain_stability`: **passed**. 11 composition tests and 3 stability tests ran, including `rebound_formals_and_slot_writes_never_reach_the_caller`, `ports_are_total_over_the_bound_signature`, `the_target_modality_bounds_the_composition` and the erased-condition mutation.
- `… cargo test --release -p lctx-postgres --test domain_composition --test domain_stability` (real PG18): **passed**, 1 test each.

| Finding | Verdict | Reason |
|---|---|---|
| F01 | **closed (contract)** | `ReachingDefinition::Nested` (code 2, appended) makes nested writes a typed reach, and the witness refuses any non-`Bound` reach. The witness contract and §15.4 state the rule, with frame write-through classed as `DynamicAccess`. Stored controls refuse a nested reach beside the parameter reach and in place of it. The producer control (a `nonlocal` rebinding emits `Nested`) is plan A14's obligation |
| F02 | **closed (scoped)** | `PlaceRoot::Entry` (code 9) separates the entry-value port from the `Formal` variable. Variable reads and writes below the variable give `EntryValueUnknown` (49). Whole-slot writes (variable, entry, collected element) compose to nothing. A write below an entry value maps to the caller's object. §15.6 states that callee transfers are port summaries. Known answers cover all four shapes, and `update`/`Config.__init__` are unchanged. **Remaining, deferred:** nothing yet checks that a producer's `Entry` root is justified; plan §8 F02 owns this, with trigger "first producer of callee transfers" (P4) |
| F03 | **closed** | `Ports::build` is one total map. It admits only the bound signature's parameters, in order, with one-to-one links. A foreign or other-variant link is `Err`; a missing link is `NoSourceDeclaration`; zero bindings is `Err`. The receiver is the first parameter's whole value of any binding kind, so `C.m(obj, v)` gives `obj.x`. Guards read the same map. Destination and qualification are checked. The result is non-empty by construction |
| F04 (target-modality part) | **closed**; **F04 remains open** | Target modality and approximation are joined in both the kernel and the stored check, and a Potential target stays Potential. The admission token, derived uniqueness and owner lookup remain open (plan §8 F04, before the P3/P4 producers) |
| F05(a) | **closed** | The stored step re-derives caller ∧ restated callee from stored rows (a unique `InvokedGuard`/`BoundGuard{source}` per callee atom at the site) and compares condition IDs. It checks that the input root is the caller's with the identity path rule, and that the output root is caller-side. Erased-condition and unrestated-guard mutations are refused in memory and in PG18. F05(b) is deferred to P3 |

**Residuals introduced by or visible in C5r** (Low; none reopens a closed finding):
- **R1.** A root of another callable is now `Err`. The old behavior was an obligation, and a `Global` input still gives `Disjoint`. This includes closure cells of an enclosing function, such as a decorator wrapper reading the decorated `f`. Decide at P4, with port summaries, whether closure reads are excluded from summaries or compose as `Disjoint`/an obligation. Otherwise an ordinary wrapper aborts its site's composition.
- **R2.** `Receiver{callable}` and `Entry{first parameter}` are two encodings of one port. Callee-side transfer keys can diverge for the same value; §15.4 should pick one or declare them equal.
- **R3.** `compose_site`'s `delivers` sees only call arguments. A value delivered into the receiver expression of an *unresolved* method call yields no obligation (CI-04). Include the receiver or callee-object occurrence.
- **R4.** `CompositionCheck` requires one restatement per (site, source atom). Two signature variants that share a parameter occurrence but bind different actuals would produce two `BoundGuard`s and refuse a legitimate generation. Key the restatement by (site, source, operand), or accept any restatement that the composed condition uses.
- **R5 (tightening).** The stored output-root check accepts any `Occurrence`. It could be restricted to the site or one of its argument/receiver actuals, using the stored `CallArgument` rows.

**Decision update.** The F01–F03 and F05(a) corrections are accepted at bounded, contract level
(Implemented; reviewer-Tested 2026-09-29). With them, G2, G3 and G6 pass within this slice's scope.
CI-G1 is no longer failed by F02 or F03, but it stays **unresolved**: F04's admission, uniqueness and
ownership are still unenforced, and `Entry` justification is deferred. The bounded decision moves
from Revise to **Accept scoped**. Excluded: stored `Entry` justification (P4), the F04 remainder
(before the P3/P4 producers), and F05(b), F06–F09 on their plan §8 triggers. A1 and A3 stay
unresolved. A2 moves from violated to unresolved: two of its causes (F02, F03) are corrected,
but the F04 booleans and F08 remain. The enclosing architecture is still not certified; X0 reassesses it.
