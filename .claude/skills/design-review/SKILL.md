---
name: design-review
description: Review architecture or a bounded implementation through explicit domain models, expected change scenarios, ownership, contracts, composition and local reasoning. Apply the layered standard, compare library fit and total complexity, and produce an evidence-grounded review. Use for architecture decisions, design reviews and architectural code audits.
allowed-tools: Read, Glob, Grep, Bash, Write, Edit, Agent
user-invocable: true
model-baseline: claude-5 (2026-08)
---

# Design review

Review a proposed design, implementation or both. Establish how the system accommodates realistic
change, then judge correctness and domain fidelity independently. The six foundations organize
the analysis: separation of concerns, stable contracts, composition, explicit domain models and
scoped semantic authority, explicit structure and local reasoning. The model must govern
implemented behavior wherever domain meaning is established or interpreted (core §1, FP-04, A2).

Apply this assessment during a requested design review or a review due at the repository's
declared cadence. Ordinary implementation work does not itself initiate a domain-model review.

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

Start from task routes and the relevant architectural section owners, expanding on a concrete
dependency or contradiction. Section IDs span DESIGN and the declared focused collection. Website
search is navigation; mechanical publication checks establish links and ownership, not architectural
quality. Source prose changes when meaning, boundaries or workflows change.

For a design review, include the affected owners and adjacent consumers. A slice review examines
its changed boundary and identifies what remains unresolved in the enclosing architecture.
Ask only if ambiguity would materially change the judgment; otherwise state the bounded scope.

## What to establish

Use judgment about investigation order and depth. These are outcomes, not a mandatory tool script.
Use the least investigation sufficient for the scoped judgment. Follow a flow only when a concrete
question about ownership, behavior or change remains unresolved; no complete flow trace is required.

1. **Domain model, responsibilities and dependencies.** Identify the phenomena, consequential
   distinctions and domain operations, then reconstruct their owners, contracts and dependency
   direction within the review scope. Assess whether the model is adequate and governs behavior
   using relevant source and accepted design, not just module or type names.
2. **Realistic change scenarios.** Select changes from the next capabilities or known variation
   axes. Distinguish instances, bindings, compositions, policies, domain concepts and mechanisms.
   Assess where a change belongs, which consumers it affects and why it propagates. Identify
   repeated decisions and material context or test setup requirements. For a
   substantial architecture review, examine a domain extension and mechanism substitution where
   credible; state a scope reason when either does not apply. A bounded review keeps its one
   relevant scenario.
3. **Authority, constraints and composition.** Assess whether derived forms and workflows use
   their semantic owners. Establish how domain operations realize their contracts and who owns their
   invariants; distinguish repeated enforcement from independent definitions. Identify domain
   rules embedded in orchestration and private mechanics exposed to consumers. Ordinary domain
   functions can suffice; no registry, universal model or conversion of algorithms to data is required.
4. **Alternatives and library fit.** Compare a suitable library mechanism and the simplest viable
   design where relevant. Qualify pinned semantics and total integration burden. Functions or
   modules may be sufficient; a wrapper or new crate must improve a real boundary.

   **Optional context: existing library use.** A focused look at relevant entries in
   docs/library-utilization.jsonl, where available, can be especially helpful when considering
   implementation alternatives. The catalog highlights library capabilities and integration
   patterns already found useful in the codebase, and may reveal opportunities to reuse
   established mechanisms instead of introducing ad hoc equivalents. It remains useful as a
   source of ideas even when some entries lag the implementation. Consultation is optional;
   catalog entries inform the alternatives, while alignment with the design principles governs
   the judgment.

5. **Independent judgments.** Settle A1–A3 from scenario evidence, G1–G8 and profile gates from
   their own evidence, and applicable foundation/supporting-rule verdicts. A2 requires both an
   adequate domain model and behavior governed by its authorities. Unresolved stays unresolved.
6. **Actionable findings and disposition.** Group by structural cause. Name a concrete semantic
   failure or architectural consequence, owner, correction and closure evidence. Link the single
   location owning current status. Follow the template's decision rules.

A demonstrated architectural violation can require revision despite correct output or a SHOULD
supporting rule. Scoped acceptance states the excluded scenario and revisit trigger, and cannot
certify the enclosing architecture. No positive architectural result offsets a failed fidelity gate.
The domain-model MUST cannot be waived for supported behavior: an in-scope A2 gap requires
revision even when outputs are currently correct. A deferral alone is not conformance.

## Evidence and calibration

Read every citation at the grain used. Proposals remain Proposed; interface inspection is
Interface-checked; code existence is Implemented. Tested/Measured claims name commands, cases,
conditions and dates. Attribute historical receipts rather than reporting them as current runs.
Static review of documentation, source and types can be sufficient, including for library
features and fit. The reviewing agent decides whether complexity, criticality or unresolved
uncertainty warrants creating and running a probe. No review tier or kind requires probes or
an evidence folder merely because it is a review. When probes are useful, follow repository
storage conventions and acceptance timing. Implementation acceptance checks and the evidence
needed for Tested/Measured claims still apply.

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
- Accepting output records as a domain model without assessing their governing operations, or
  accepting one centralized definition that omits consequential distinctions.
- Deriving all tests from production logic, then treating agreement as independent evidence.
- Calling a proposed benefit measured, or an accepted ADR an implemented correction.
- Repeating a finding's current status across reviews, plans and handoff prose.
