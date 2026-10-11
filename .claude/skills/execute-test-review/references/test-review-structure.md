# Suggested test-review structure

Use this guidance to explain the review's conclusions and causal reasoning. It is a flexible
starting point, not a required set of headings, tables, fields or investigation stages. Combine,
rename, expand or omit parts according to the argument. A small review may need only a few
paragraphs and one finding; a substantial review may develop several interacting explanations.

Test observations motivate the inquiry. Source and contract inspection, probes, experiments,
library research and comparisons can all contribute to it. Choose which references, examples
and results help substantiate and communicate the judgment. There is no required evidence
inventory, exhaustive failure membership table, separate packet or account of every investigative
step. Existing claim-calibration and reporting conventions continue to govern claims actually made.

| Possible part | What it helps the reader understand |
|---|---|
| Integrated assessment | What the observations reveal, the consequential causes or uncertainties, and what should change |
| Subject and context | The intended capabilities and relevant execution/source conditions; the scope and limits of the assessment |
| Causal explanation | How expectations, contracts, dependencies and executing mechanisms produce the observed behavior; competing explanations and contrary cases |
| Test and environment validity | Where assertion meaning, fixture design, migration state, readiness, configuration or harness behavior affects the diagnosis |
| Structural findings and recommendations | Concrete defects, responsible boundaries, proposed corrections, preserved guarantees, alternatives and meaningful closure conditions |
| Synthesis and further investigation | How corrections interact, which decisions depend on others, what remains uncertain and what would settle it |

## Findings that subsequent work can use

Keep stable `F01`, `F02`, … identifiers within the source review, with explicit anchors for
resolvable `review#F01` links. Develop each finding as a causal argument rather than forcing its
explanation into a wide table. A compact index is useful when it improves navigation.

Explain what is wrong, why the diagnosis follows, what consequence matters, and which owner
should change. Relate design principles to the mechanism rather than merely attaching labels.
For a consequential correction, describe the intended behavior, viable alternatives, guarantees
to preserve and questions still affecting the remedy. State what would establish correction;
inspection or a revealing semantic control may suffice. Diagnosis and remedy can have different
maturity under the standard's existing vocabulary.

For example, a test may assert that each operation creates a database while the implemented
contract provides logical attempt isolation on shared storage. Assess both the assertion and the
actual isolation guarantee: replacing the database-count assertion is justified only if its
mechanical expectation is obsolete, and does not settle whether cross-attempt reads are prevented.
The report can recommend a test update and a production correction together when both are warranted.

Group concrete manifestations of one cause without merging distinct closure obligations. Related
findings may instead be independent, prerequisite-dependent or alternative explanations. Keep
those relationships clear where they affect the recommendation. Preserve uncertainty rather than
inventing a common cause, numerical confidence score or implementation work to fill every slot.

## Scope, maturity and disposition

Give enough context to understand the conclusion and its limits. Historical results, new
experiments, static conclusions and proposed benefits establish different claims. Use the
existing design standard's evidence vocabulary; a completed review is not a test pass or an
implemented remedy. An incomplete run cannot establish success for its remaining scope.

Use the repository's rule-impact route when a recommendation would change current rules,
retaining stable `RCnn` references where applicable. Findings feed the existing plan-creation
workflow. The source review owns its dated assessment; current scheduled disposition belongs
to one active plan, with links from later reviews and STATUS. Unscheduled deferrals keep their
trigger under the existing binding. No second failure or remediation register is introduced.

The test-review format does not require a general architectural verdict. If that judgment is
part of the assignment, keep the applicable design-review assessments discoverable and preserve
their acceptance obligations. Neither a local diagnosis nor passing checks certify unexamined
interactions. Organize supporting material only when it is independently useful; reviewer
assignments and document length need not determine document boundaries.
