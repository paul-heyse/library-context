---
name: design-review
description: Review architecture or a bounded implementation through expected change scenarios, component ownership, contracts, composition and local reasoning. Apply the repository's layered standard and domain constraints, compare library fit and total complexity, and produce an evidence-grounded review. Use for architecture decisions, design reviews and architectural code audits.
allowed-tools: Read, Glob, Grep, Bash, Write, Edit, Agent
user-invocable: true
model-baseline: claude-5 (2026-08)
---

# Design review

Review a proposed design, implementation or both. Establish how the system accommodates realistic
change, then judge correctness and domain fidelity independently. The six foundations organize
the analysis: separation of concerns, stable contracts, composition, authoritative representations,
explicit structure and local reasoning.

## Load the standard

Find `standard.toml` at the location named by repository instructions. Read the declared core
principles and template, each profile and its companion skill, then the repository binding.
The manifest owns versions and paths; the binding owns local cadence, authority and disposition
routes. If the manifest is absent, use an available core and state the limitation. If the core is
missing, report the missing prerequisite.

Profiles add or tighten constraints; they do not replace architectural assessment or waive a
core MUST. State which authority governs a known conflict and whether it affects the decision.
Historical reviews use their recorded version and are not retroactively certified.

## Scope

- **target:** document, code scope or both; infer from the request when clear.
- **tier:** `change` within accepted boundaries, or `design` for architectural choices and assembled
  scope. Depth follows impact and uncertainty; compact/standard/deep are legacy effort descriptions.
- **purpose:** `conformance` to the accepted architecture or `target` for the best architecture
  serving the functional outcome. Use the binding's default.
- **scenarios/focus:** expected changes, foundations or domain concerns to emphasize. Focus never
  hides an encountered in-scope defect.
- **slug:** infer from the subject when omitted.

For a design review, include the affected owners and adjacent consumers. A slice review examines
its changed boundary and identifies what remains unresolved in the enclosing architecture.
Ask only if ambiguity would materially change the judgment; otherwise state the bounded scope.

## What to establish

Use judgment about investigation order and depth. These are outcomes, not a mandatory tool script.

1. **Responsibilities and dependencies.** Reconstruct coherent owners, hidden decisions, public
   contracts and dependency direction. Use source and accepted design, not just module names.
2. **Realistic change scenarios.** Select changes from the next capabilities or known variation
   axes. Trace trigger → owner → contract change → affected consumers → verification. Explain
   why change propagates, which decisions repeat and what context/test setup is required.
3. **Authority, constraints and composition.** Trace semantic definitions into derived forms and
   workflows. Identify domain rules embedded in orchestration and private mechanics exposed to consumers.
4. **Alternatives and library fit.** Compare a suitable library mechanism and the simplest viable
   design where relevant. Qualify pinned semantics and total integration burden. Functions or
   modules may be sufficient; a wrapper or new crate must improve a real boundary.
5. **Independent judgments.** Settle A1–A3 from scenario evidence, G1–G8 and profile gates from
   their own evidence, and applicable foundation/supporting-rule verdicts. Unresolved stays unresolved.
6. **Actionable findings and disposition.** Group by structural cause. Name a concrete semantic
   failure or architectural consequence, owner, correction and closure evidence. Link the single
   location owning current status. Follow the template's decision rules.

A demonstrated architectural violation can require revision despite correct output or a SHOULD
supporting rule. Scoped acceptance states the excluded scenario and revisit trigger, and cannot
certify the enclosing architecture. No positive architectural result offsets a failed fidelity gate.

## Evidence and calibration

Read every citation at the grain used. Proposals remain Proposed; interface inspection is
Interface-checked; code existence is Implemented. Tested/Measured claims name commands, cases,
conditions and dates. Attribute historical receipts rather than reporting them as current runs.
Run a probe only where reasoning leaves material uncertainty; use repository acceptance timing.

Library-first means considering established implementations of the required capability. Both
adoption and bespoke code can add excessive coupling, lifecycle or configuration. A planned
consumer can justify a seam; catalog availability alone cannot. Shared library types can be an
intentional contract. Do not invent a provider framework to demonstrate hypothetical replaceability.

The [reference](REFERENCE.md) contains finding calibration and investigative lenses. Use relevant
parts, especially for architectural consequences and false positives. Independent tests must
challenge production semantics, even when mechanical validators derive from one authority.

## Output

For a requested review, write one document at the binding's location using the template's slots
and scoped profile additions. A request to discuss or revise this process does not itself require
an additional review artifact. Close with scope, A1–A3, gates, material findings, bounded decision,
enclosing architectural status and path. Distinguish review acceptance from release qualification.

## Failure modes

- Starting and ending with individual rule correctness while leaving ownership and change unexamined.
- Requiring a wrong output before reporting concrete architectural damage.
- Accepting an enclosing architecture because successive narrow slices passed.
- Treating file count, traits, crates, declarative vocabulary or library adoption as proof of quality.
- Deriving all tests from production logic, then treating agreement as independent evidence.
- Calling a proposed benefit measured, or an accepted ADR an implemented correction.
- Repeating a finding's current status across reviews, plans and handoff prose.
