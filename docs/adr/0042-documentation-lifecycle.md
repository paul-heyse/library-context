---
id: ADR-0042
title: Keep a current documentation working set; Git holds retired records
status: accepted
date: 2026-09-25
supersedes: []
superseded-by: null
design: [§2]
evidence: Tested
revisit: A current task needs a retired document that Git recovery cannot supply in reasonable time, or current readers again need supersession chains, amendment logs or historical plans to interpret a contract.
---

## Context

ADR-0041 published a focused architectural collection, but the checkout still carried every
earlier plan, 105 dated reviews, the research input, retired review standards and 41 ADRs whose
current meaning was often spread across amendments and partial supersessions. The publisher put
all of it in navigation and in **Everything** search, and ADR-0022 and ADR-0010 could only be read
correctly by extracting the clauses that survived later records. Agents rebuilt project history
before they could act. The operator directed a migration (2026-09-25): keep only material that
explains what the system is, why its current design was chosen, what is uncertain and what to do
next; Git holds the rest.

Two tooling rules made retirement unsafe: ADR lint required every `supersedes` target to remain in
the tree, and new records were numbered from the files present, so deleting a record could reuse
its id.

## Options

1. **Keep everything and label it** (the ADR-0041 status quo): no loss, but the reading context
   and contradictions stay, and History pages keep appearing in navigation and search.
2. **An archive tree or historical book**: preserves browsing, but recreates a second corpus with
   its own links, search scope and maintenance, and still invites chain-reading.
3. **A current working set; Git for recovery** (chosen): one short recovery route, current owners
   carry surviving meaning, and tooling treats retired records as provenance.

## Decision

- **Ownership.** Architecture pages own current contracts and accepted or proposed targets; current
  ADRs own reasons and meaningful alternatives; the active plan owns execution and the current
  disposition of scheduled findings; STATUS owns the checkpoint. A current reader never needs a
  supersession chain, an amendment log or a retired plan to interpret any of them.
- **Consolidation.** A mixed or obsolete ADR whose rationale is still useful is replaced by a
  concise record organized by responsibility; one replacement may summarize several predecessors
  without their history. A self-contained, still-accurate ADR stays regardless of age. Records are
  not reissued mechanically, and implementation increments do not each need a record.
- **Retirement.** An ADR is removed from the tree only after each surviving clause has a current
  owner or is explicitly rejected or out of scope; partial supersession is resolved clause by
  clause. A consolidation restates decisions already in force: it never accepts an unapproved
  target or silently chooses an open alternative. Predecessors are deleted only in or after the
  commit that accepts their replacement; a proposed record can never be the only holder of a
  retired decision. A proposed record stays only while it carries a live choice.
- **Immutability is unchanged** for records that remain: accepted bodies change only through
  replacement, apart from status metadata and dated factual amendments. Former acceptance is not a
  reason to keep a record in the checkout.
- **Governing references resolve; history does not have to.** Every ADR cited by the architectural
  collection, and every `superseded-by` of a retained record, must exist (lint enforces both).
  Current instruction surfaces (AGENTS.md, the binding, skills, STATUS, the active plan and the
  gold freeze) cite only retained records as authority; before deleting a record, search for its
  id outside `docs/adr/` and repoint or remove each governing mention. A mention kept only as
  history is allowed. A `supersedes` entry
  naming a retired record is provenance. New ids are allocated above every number present in the
  tree **or** in reachable Git history; creation refuses a shallow history rather than reuse an id.
  There is no retirement register or id catalog.
- **Other documents.** Closed reviews, completed plans and evidence without a current consumer are
  removed once their obligations and useful rationale are carried forward. A review stays while it
  supplies an open finding or needed decision evidence; the plan owns its disposition. Finding and
  evidence ids stay intelligible; closed rows need not be kept forever.
- **Sections.** A moved **live** section keeps its stable id and a relocation pointer (ADR-0041).
  A retired document or obsolete section gets no permanent stub. Section ids are never reused.
- **Recovery.** `docs/README.md` gives Git commands that find any deleted path and its last
  content, and names the migration's pre-removal revision for convenience. It is a recovery aid, not a reading prerequisite. Publication includes only the
  retained corpus; a History search scope exists only if historical pages are published.

This narrows ADR-0041's historical retention and ADR-0040's "preserve closed/superseded rows"
disposition rule. Their other decisions remain in force.

## Consequences

- Current reading is bounded by owners and live records; a decision's surviving reason must be
  written where readers look, which costs a consolidation when a record is replaced.
- Recovering an earlier rationale takes a deliberate Git command; ordinary lint, navigation and a
  fresh site build never depend on retired files or on fetching LFS evidence.
- Record creation needs non-shallow history; lint and publication do not.
- Nothing here changes Arrow schemas, provider semantics, product dependencies or the review
  cadence. The tooling is **Tested** by `tests/scripts/test_adr.py` (retired predecessors, a
  proposed replacement, missing governing records, history-aware numbering, shallow clones); the
  reading-cost benefit remains **Proposed**. The 2026-09-25
  design/target review (`design_review_documentation-lifecycle_2026-09-25.md`) returned Accept
  with changes; its F01–F05 were applied before acceptance.
