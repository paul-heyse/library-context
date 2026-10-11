---
name: execute-test-review
description: Conduct a requested causal and structural test review using results, code and contract inspection, probes and research; write an integrated review with design-aligned recommendations. Use for test reviews, not ordinary suite execution or remediation.
---

# Execute a test review

Investigate what test observations reveal about the system, its tests and their execution
conditions. Develop a coherent causal assessment and recommendations aligned with the functional
target and design principles. Build on the agreed approach, adapting it as discoveries change
the premises. Use [plan-test-review](../plan-test-review/SKILL.md) when consequential questions
about the inquiry remain; direct invocation need not start another planning round.

Review work includes code and contract inspection, creating and executing revealing probes or
focused tests, library research, comparisons and other useful investigation. Test outputs are
starting signals, not the review's evidence boundary. Choose methods for the questions they
resolve rather than following a fixed sequence. Production and existing-test remediation remain
separately authorized work.

## Ground the inquiry in meaning and actual execution

Discover the applicable design principles, heuristic companion, profiles and repository binding
through local instructions and `standard.toml`. Judge corrections against that standard and the
functional target. Current ADRs, plans and conventions explain the existing design; they can be
challenged rather than treated as constraints on a better recommendation. Preserve the binding's
rule-impact and disposition routes.

Establish enough context to interpret the observations: intended scope, actual invocation,
relevant source/configuration, prerequisites and the available execution/capture information.
Use existing evidence readers for selective investigation where useful; choose original outputs,
native replay, code or new experiments when they better answer the question. Missing fields and
unexamined scope stay unknown. Relevant historical results retain their original conditions.

Keep native outcomes, phases and attempts distinct from command failure, capture health,
interruption, postprocessing and cleanup. Supplemental events are not additional test executions.
A partial run is not evidence that its remaining work passed or never ran. Comparing runs requires
attention to answer-affecting context; matching revisions or messages do not establish equivalence.

## Follow causes across the relevant boundaries

Reconstruct the expectation at risk and inspect its semantic owner, realization and consumers.
Follow the mechanism far enough to distinguish symptoms, triggering conditions and structural
causes. Similar messages may have independent causes; different failures may share a prerequisite
or ownership defect. Group manifestations without losing distinct obligations or contrary cases.

Consider alternative explanations and seek comparisons that could overturn the working diagnosis.
A timeout alone does not identify the slow operation, a retry pass does not establish correction,
and repeated failures do not by themselves prove nondeterminism. A revealing experiment should
distinguish material explanations rather than merely repeat expensive execution. Static inspection
may settle a contract or structural defect without reproduction; quantitative claims still need
appropriate measurements.

Expand to similar patterns where the mechanism warrants it. Consequential issues can concern
semantic authority, construction and validity, cache dependencies, shared active state, isolation,
publication, recovery, repeated preparation or physical amplification. Apply relevant principles
and efficient-architecture heuristics qualitatively. An avoidable structural problem need not be
the measured bottleneck to merit correction. Do not force all findings into one root cause or
whole-system redesign.

## Assess tests and execution conditions independently

These are possibilities to examine, not required classifications or a checklist:

- **Stale expectations and legacy fixtures.** Establish which guarantee or mechanism an assertion
  represents and what actually changed. Age or disagreement with current code is insufficient.
  An accepted target is not proof that its replacement is implemented. Distinguish an obsolete
  assertion from a valid test exposing incomplete migration or a regression.
- **Implementation-specific testing.** Consider whether private mechanics can be replaced by
  independent checks of the enduring guarantee. If recommending retirement, explain which
  obligation is obsolete and what meaningful protection should survive. Updating expected output
  from the same questionable producer can conceal a defect.
- **Environment, fixtures and harness.** Examine relevant readiness, configuration, service and
  logical scope, retained inputs, ordering and cleanup. Separate an incidental host condition
  from a recurring contract weakness. Fixture convenience can introduce unintended coupling,
  while a missing prerequisite can prevent any conclusion about production behavior.
- **Concurrency, retries and reuse.** Assess shared mutable state, identity, invalidation,
  synchronization and effect ownership where relevant. Serialization, fresh state, larger limits
  or retries may distinguish explanations without establishing a structural correction.
- **Incomplete or conflicting observations.** Capture defects and unsupported producer details
  can coexist with real failures. Preserve what is established and what remains uncertain;
  a later successful run does not reinterpret or erase the earlier observation.

Keep normal test/compiler parallelism and the repository's preparation, fixture, preservation and
storage rules. Use the existing owners for new probe outputs and retained citations. No mandatory
evidence folder, inventory, probe campaign or separate packet follows from this review kind.

## Develop corrections that fit together

Explain the causal mechanism, consequence and responsible boundary before recommending a remedy.
Describe the corrected operation and what consumers can stop deciding or reconstructing. Preserve
required guarantees and independent semantic checks; make intentional behavior changes clear.
Consider suitable built-in/library capabilities and the simplest viable alternative, qualifying
material API claims through the repository's library documentation routes.

Challenge consequential remedies with legitimate cases they might reject, flatten or mishandle.
Compare total ownership, integration, execution and maintenance burden, not just local code size.
Distinguish an established diagnosis from a proposed remedy and from demonstrated correction.
A recommendation can remain useful while a material design question needs further investigation.

Reconcile related findings: shared causes, prerequisite contracts, independent defects, alternative
remedies and conflicting recommendations. Separate consequence-based priority from dependency
order. Where delegation helps, use the [shared roles](../../../.agents/roles/README.md) and keep
one principal reviewer responsible for the integrated argument. Preserve independent judgments
and resolve their disagreements through decisive investigation or explicit uncertainty.

## Write the integrated review

Use the [suggested report structure](references/test-review-structure.md) to develop the argument
at a depth suited to the subject. Choose headings, examples, references, comparisons and supporting
material for what they help the reader understand. There is no required evidence section, exhaustive
failure table or investigation diary. The report should communicate the judgment and its material
limits without making readers reconstruct the reasoning from raw outputs.

Use stable finding IDs and the existing finding/disposition conventions. Apply the standard's
claim vocabulary to the diagnosis and remedy separately. Identify consequential rule impacts
through the existing route; the review does not itself change those rules. Follow the binding's
test-review location. Subsequent plan creation consumes these findings without another ledger.

A completed review does not certify a suite, remedy or enclosing architecture. A scoped test
review can identify architectural defects without claiming general architectural acceptance.
If the assignment includes that broader judgment, use [design-review](../design-review/SKILL.md)
for its applicable assessment obligations; one integrated document may serve both purposes when
it satisfies them. Conclude with the assessment, material recommendations and unresolved questions,
the report path, and the assurance actually established.
