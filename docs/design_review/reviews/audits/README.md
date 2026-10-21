# Design-review recommendation audits

Companion documents here explain the net changes made by a
[design-review audit](../../../../.claude/skills/design-review-audit/SKILL.md). The integrated
review at its original path remains the clean input for planning. The
[binding](../../design_principles/binding/library-context.md#design-review-recommendation-audit)
owns the publication and disposition boundary.

For `../design_review_<subject>_<date>.md`, use
`design_review_<subject>_<date>_audit.md` here. The date in this name identifies the source
review; record the audit's actual date inside. Maintain one current companion per principal
review, updating it in place for another audit; Git retains earlier comparisons.

Use moderately structured Markdown suited to the breadth of the recommendations. Make the
input principal review's exact Git revision recoverable before replacing it. Identify that
revision, the inspected source state, applicable standard, date and coverage limits. A repeat
audit compares the then-current principal revision, rather than silently treating an earlier
comparison as its input.

Account for every input recommendation: **Retain**, **Improve**, **Correct or adjust**, **Omit**,
or **Unresolved**, with reasons where consequential. Include additional recommendations and
their basis, correspondence to integrated output, and effects on the combined design or decision.
A compact index can route to richer explanations. Recommendation references need not be finding
IDs; preserve the mapping where remedies split, combine or change while the diagnosis survives.

The integrated review explains the resulting design and carries a brief dated attribution and
link back here. Keep the critique of the first round in the companion. Preserve finding and
rule-impact identities and distinguish original evidence from newly established claims.
Neither document approves rule changes or updates accepted implementation scope automatically.

These dispositions assess recommendations; they do not form another implementation ledger.
Plans remain the sole owners of scheduled finding progress. Plan-completion audits update those
plans in place and have no output folder here. The broad diagnostic account lives separately at
[`docs/design_review/audit/`](../../audit/README.md).

No recommendation audit has been performed merely by creating this folder. Retain a companion
while it explains a current review's revision or a live downstream decision; retire it with the
review when its last current consumer is gone, carrying surviving obligations to their owners.
