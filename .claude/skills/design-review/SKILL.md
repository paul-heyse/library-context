---
name: design-review
description: Review architecture or a bounded implementation through explicit domain models, expected change scenarios, ownership, contracts, composition and local reasoning. Apply the layered standard, compare library fit and total complexity, and produce an evidence-grounded review. Use for architecture decisions, design reviews and architectural code audits.
allowed-tools: Read, Glob, Grep, Bash, Write, Edit, Agent
user-invocable: true
model-baseline: claude-5 (2026-08)
---

# Design review

Review a proposed design, implementation or both. Establish how the system accommodates realistic
change, then judge correctness and domain fidelity independently. The seven foundations organize
the analysis: separation of concerns, stable contracts, composition, explicit domain models and
scoped semantic authority, explicit structure, local reasoning and execution fit. The model must govern
implemented behavior wherever domain meaning is established or interpreted (core §1, FP-04, A2).

Apply this assessment during a requested design review or a review due at the repository's
declared cadence. Ordinary implementation work does not itself initiate a domain-model review.

## Load the standard

Find `standard.toml` at the location named by repository instructions. Read the declared core
principles, template and the declared heuristics companion, each profile and its companion skill,
then the repository binding.
The manifest owns versions and paths; the binding owns local cadence, authority and disposition
routes. If the manifest is absent, use an available core and state the limitation. If the core is
missing, report the missing prerequisite.

Profiles add or tighten constraints; they do not replace architectural assessment or waive a
core MUST. State which authority governs a known conflict and whether it affects the decision.
Historical reviews use their recorded version and are not retroactively certified.

Use the [Heuristics for Efficient Architecture](../../../docs/design_review/design_principles/core/efficient-architecture-heuristics.md)
with the principles to assess relevant execution choices before they become fixed. The companion
informs existing scenario/alternative judgments; it creates no extra verdict table or checklist.

## Grounding

Judge the subject only against the loaded standard (principles, companion heuristics, template and
declared profiles) and the system's functional target, located through the binding. Every other
repository rule describes the current design, not the criteria for judging it: accepted decisions
and ADRs, binding decisions and product rulings, binding policies, agent instructions and working
agreements, operator decisions recorded in status or memory, global working rules and dependency
pins. Read them to understand the subject. Do not judge the design by its agreement with them,
narrow a recommendation to fit them, or review them for their own sake. Where the target
statement names a chosen mechanism, such as a store or framework, that choice is a rule.

Rules for conducting and reporting the review still apply: its location, the reviewer's
independence, the evidence vocabulary and outcome reporting. Once the judgment is settled, the
review lists the rules its recommendations would change (see Identify rule impacts).

## Scope

- **target:** document, code scope or both; infer from the request when clear.
- **tier:** `change` within accepted boundaries, or `design` for architectural choices and assembled
  scope. Depth follows impact and uncertainty; compact/standard/deep are legacy effort descriptions.
- **purpose:** always `target`, the best architecture serving the functional outcome; the tier
  sets breadth only. The template's `conformance` purpose is not used: whether an implementation
  matches its accepted design belongs to implementation review.
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

For substantial reviews, use the [shared agent roles](../../../.agents/roles/README.md) to delegate
bounded code mapping and library research where helpful. Give a fresh design reviewer the target,
requirements and source evidence for an independent judgment. Delegation does not remove the
reviewer's obligation to inspect decisive evidence or the coordinator's responsibility to resolve
findings through the existing authority and disposition routes.

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
   design where relevant. Qualify semantics at the resolved version and total integration burden. Functions or
   modules may be sufficient; a wrapper or new crate must improve a real boundary. Compare the
   composed physical operation: access paths, optimizer/bulk capabilities, movement, preparation,
   working set, enforcement frequency, transaction/recovery scope and total operator/developer
   machinery. A module or semantic kind need not impose an execution stage or storage object.

5. **Independent judgments.** Settle A1–A4 from scenario evidence, G1–G8 and profile gates from
   their own evidence, and applicable foundation/supporting-rule verdicts. A2 requires both an
   adequate domain model and behavior governed by its authorities. A4 needs a qualitative assessment of a credible execution
   route for a workload premise drawn from functional intent: relevant operations, input size/skew,
   growth, concurrency/deployment and resource envelope. Use relevant growth/failure scenarios;
   safe refusal or bounded output alone does not establish fit. Unresolved stays unresolved.
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
Assess execution fit qualitatively from relevant operations and growth/failure scenarios.
Explain material tradeoffs in plain language. This design/review consideration does not require
numerical estimates, cost models, estimators, runtime cost accounting, execution-planning
machinery, instrumentation, formal cost proofs or additional proof artifacts. Introduce such
mechanisms only for a separate concrete functional or operational requirement. Quantitative
performance/capacity claims still require measurements; semantic correctness obligations remain.
Qualitative assessment of execution structure can establish avoidable amplification before measurement. No speed claim
is needed for an A4 defect; quantitative speed/capacity claims still require measurements. Do not
narrow the workload to convenient fixtures or turn these lenses into a standing audit/checklist.
Static review of documentation, source and types can be sufficient, including for library
features and fit. The reviewing agent decides whether complexity, criticality or unresolved
uncertainty warrants creating and running a probe. No review tier or kind requires probes or
an evidence folder merely because it is a review. When probes are useful, follow repository
storage conventions and acceptance timing. Implementation acceptance checks and the evidence
needed for Tested/Measured claims still apply.

When probes produce raw captures or derived evidence, follow
[workspace storage](../../../docs/workspace-storage.md): establish a run/profile owner and named
consumer, keep raw replay and cited reports in independently managed components, and hold storage
admission through process cleanup. Small retained evidence uses its existing Git/LFS owner;
environments and build outputs stay outside it. Release a consumer only when its evidence/replay
need ends. Archive publication must qualify the required replay operations; an unavailable
destination or reader leaves the original protected. Evidence age and file mtime do not authorize
deletion.

Library-first means considering established implementations of the required capability. Both
adoption and bespoke code can add excessive coupling, lifecycle or configuration. A planned
consumer can justify a seam; catalog availability alone cannot. Shared library types can be an
intentional contract. Do not invent a provider framework to demonstrate hypothetical replaceability.

The [reference](REFERENCE.md) contains finding calibration and investigative lenses. Use relevant
parts, especially for architectural consequences and false positives. Independent tests must
challenge production semantics, even when mechanical validators derive from one authority.

## Synthesize the assessment

Make the review a coherent argument about the system and the decision it should inform. Explain
the functional intent, the consequential responsibilities and design choices, what supports the
intended capabilities, and where the design falls short. Connect evidence to causes, consequences
and recommendations so the reader can understand the judgment without reconstructing it across
tables. Several independent concerns may remain; do not force them into one root cause or redesign.

Where findings interact, distinguish shared causes, prerequisite contracts, independent defects,
alternative remedies and conflicting recommendations. Group manifestations of one cause while
preserving distinct obligations needed for closure. Sharing a principle ID or component does not
make two findings the same defect. Assess whether proposed corrections fit together: individually
reasonable remedies can create duplicate owners, incompatible meanings or new consumer burdens.
Explain the relevant relationships in prose or a small diagram when useful; no finding graph or
whole-system remedy assessment is required for a bounded review.

In delegated work, the coordinator reconciles shared assumptions, overlapping findings, boundary
gaps and incompatible recommendations. Preserve the source and scope of each judgment; resolve
disagreements through decisive evidence or retain their effect on the decision. Local positive
assessments do not establish the adequacy of their interactions. Investigation assignments need
not become the published document structure.

## Make recommendations useful to subsequent design

For consequential remedies, describe the intended behavior and responsibility boundary: what
the corrected operation consumes, decides and produces, and what its consumers can stop
interpreting independently. State preserved guarantees, intentional behavior changes and support
limits where they constrain the correction. Inspected strengths can be preservation constraints,
such as a useful lifecycle separation, an independent check or an existing semantic distinction.
A narrow defect may need only a sentence; conceptual products do not mandate new types or services.

Distinguish the evidence establishing a diagnosis from the maturity of its remedy. A sound
finding remains reportable when the best replacement needs further design. Explain the proposed
direction, viable alternatives and material assumptions, and what decision or evidence would
settle or reopen the recommendation. A revised remedy need not invalidate the original diagnosis.
Apply the standard's evidence vocabulary to these distinct claims rather than adding confidence
scores or a second status system.

Challenge a consequential remedy with a revealing legitimate case: could it reject valid work,
erase a distinction, weaken a guarantee or transfer policy to the wrong owner? Reasoning may
settle the question; probes remain discretionary. Relate claimed benefits to the mechanism and
conditions that would produce them, including a qualitative assessment of material integration and ownership burdens.

Separate consequence-based priority from prerequisite order. A lower-priority contract correction
can enable a more urgent consumer fix. Explain the dependency through the capability or meaning
required, not just finding numbers. Consider preservation, external consumers and intermediate
states when transition feasibility affects a recommendation. Detailed work packages, staffing,
schedules and migration procedures belong to subsequent planning unless explicitly requested.

Connect uncertainty to the decision it affects. Distinguish unexamined breadth from a missing
premise that could change the verdict or selected remedy; name suitable settling evidence where
material. An unexamined area is not itself a defect. Keep the next consequential decision and
follow-up obligation clear without inventing implementation work merely to make the review actionable.

## Identify rule impacts

After the judgment and recommendations are settled, compare the recommended design with the
current rules named under Grounding. List each rule it would change, replace or retire, including
a rule it makes unnecessary, in the template's authority-changes slot. Give each item a stable
identifier (`review#RCnn`), the rule and where it is recorded, the proposed change, the findings
or recommendations that depend on it, and what the recommendation becomes if the rule is kept.
State "none" when nothing is affected. The review does not edit, supersede or route these rules:
plan creation presents them to the operator, and a change takes effect only once confirmed there.

## Organize the output for its readers

The investigation structure, system decomposition and published argument need not coincide.
Group detailed discussion by coherent responsibility, question or architectural cause. Tables
support comparison and navigation; use prose for explanations that lose meaning in wide cells.
A compact finding index can link to richer arguments with stable finding IDs.

Start with enough synthesis to explain the conclusion and its scope, then develop the supporting
arguments at the depth needed. For a substantial topic, supporting documents can hold bounded
analysis or evidence. Split when the reasoning is useful to consult independently; keep tightly
coupled explanations together when separation would make readers repeatedly reconstruct them.
Length, one file per component or one document per reviewer is not sufficient reason to split.

The principal review owns the combined scope, architectural explanation, material relationships,
overall judgment and evidence limits. Supporting documents identify their narrower role, baseline
and consumed context, with a clear route back to that review. They may carry bounded judgments
but do not create competing overall verdicts or disposition ledgers. Small reviews can do this
in one compact document.

Use the selected template's slots as stable content references, with relevant profile additions
and proportional detail. Group or distribute the explanations where this improves understanding,
keeping applicable foundation, gate and decision judgments discoverable. Preserve the selected
standard's assessment obligations and acceptance rules; flexible presentation does not waive them.
The template guides placement and the [reference](REFERENCE.md#synthesis-and-corrective-reasoning)
offers calibrated examples. These are authoring considerations, not additional mandatory review stages.

## Deliver the review

For a requested review, write the principal document at the binding's location using the template's
content references and scoped profile additions. A read-only delegated reviewer returns the complete review text
and intended path; the coordinator publishes it while preserving the reviewer's judgment.
Focused design advice during plan creation can instead be incorporated in the plan; it does not
replace a formal review due under the binding. A request to discuss or revise this process does not itself require
an additional review artifact. Close with scope, A1–A4, gates, material findings, rule impacts, bounded decision,
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
- Fitting a recommendation to an existing ADR, ruling or policy instead of the standard and the
  functional target.
- Auditing the rule corpus instead of the design, or treating a rule impact as applied before the
  operator confirms it.
