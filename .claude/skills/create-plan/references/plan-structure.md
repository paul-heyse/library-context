# Suggested plan structure

Use this outline to make the plan useful to an implementing agent. Adapt it to the subject and
the repository's conventions; it is not a required set of headings or a completion checklist.
Technical design may need several subject-specific sections, while a small plan can combine
most of the outline into a few paragraphs and a work table.

Identify the reference review or parent plan, inspected baseline and date, and the plan's
evidence status near the beginning. Explain which document owns architecture, sequencing or
finding disposition where those responsibilities are split. An approved design remains distinct
from implemented or verified behavior.

| Part | What it establishes |
|---|---|
| Purpose, scope and basis | Intended outcome, completion boundary, reference review or parent plan, and governing architecture |
| Current baseline and affected foundations | What exists, relevant evidence limits, reusable components, and conclusions from the focused dependency assessment |
| Target design and contracts | Responsibilities, semantic owners, interfaces, invariants and interactions; divided into subject-specific sections |
| Choices and alternatives | Material design decisions, library fit, tradeoffs and unresolved questions |
| Execution sequence | Work packages, dependencies, affected owners and completion evidence |
| Migration and retirement | Changes to existing consumers, preservation obligations, replacement and deletion work where relevant |
| Verification and acceptance | Focused controls, assembled acceptance, review points and separately scoped measurements |
| Finding disposition and current state | Traceability to source findings, open decisions, deferred work and the next executable step |

## Work packages

For each package, establish what changes, who owns it, what it depends on and what establishes
completion. A table often makes this easy to scan. Use stable package identifiers when other
parts of the plan need to reference them. A dependency diagram can clarify branches and joins
when order alone is insufficient.

Place preparatory improvements before their dependent work and identify useful independent
branches where they exist. Detail file changes, APIs or algorithms when they resolve material
uncertainty. Package boundaries should follow coherent changes and their verification needs.

## Evidence and current state

Separate future acceptance requirements from existing receipts. State unresolved design choices
and the work they constrain, rather than presenting assumptions as settled decisions. Reference
the existing disposition owner for source findings; accepting the plan does not close their
implementation obligations.

As execution proceeds, keep current state and the next action at their declared owner. Link to
that owner from related documents so the plan remains usable without competing status records.
