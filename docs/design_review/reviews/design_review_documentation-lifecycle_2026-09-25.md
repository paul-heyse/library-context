# Design review: documentation lifecycle (ADR-0042) and its ADR tooling

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Proposed `docs/adr/0042-documentation-lifecycle.md` (untracked) and the uncommitted diff on HEAD `94f4eb3`: `scripts/adr.py`, `tests/scripts/test_adr.py`, DESIGN §2 lifecycle paragraph, `docs/README.md` *Historical recovery*, `.claude/skills/adr/SKILL.md`, `.claude/skills/handoff/SKILL.md`, `AGENTS.md`, binding §4. The tree is dirty; the review reads the working copy. |
| Standard | Core 3.0 (repository-owned), profile code-intelligence 1.1 (n.a.: documentation tooling, no fact semantics), binding `library-context` |
| Tier · purpose | Design · target, proportionate (compact effort) |
| Reviewer · date | `design-reviewer` subagent, 2026-09-25 |
| Maturity and outcome | Migration packet M2 of `docs/plans/legacy-documentation-migration_2026-09-25.md`. The change should let M3/M4 delete ADRs, reviews, plans and evidence without breaking governance or reusing ids. |
| Supported scope | ADR lint/index/numbering semantics; lifecycle ownership text; recovery route; process instructions. Whether to migrate is operator-approved (plan §1) and not re-judged. |
| Expected changes | S1 retire a group behind a consolidated replacement; S2 create the next ADR after deletions, including in a shallow clone; S3 a later reader resumes and recovers a retired file; S4 replace one accepted record by clauses. |
| Baseline | ADR-0041 publication and resolver; ADR-0040 disposition rules; the old lint required every `supersedes` target to exist and numbered from files present. |
| Method and coverage | Read the ADR, the full `adr.py`, the new tests, `design_sections.source_paths`, `check_gold.py`, the plan's §1–§2, M0, M2 and §5, binding *Reviews* and §4, the core template. Grepped ADR-id citations outside `docs/adr`. One probe (below). Not examined: `scripts/docs.py` publication of the retained corpus (M5). |

## 2. Responsibilities and semantic ownership

| Component | Responsibility | Consumer contract |
|---|---|---|
| ADR-0042 | Lifecycle policy: which surface owns current meaning; when a document may leave the tree | Skills, AGENTS, binding §4, DESIGN §2 |
| `adr.py lint` | Governance validity: section refs, governing citations in the architectural collection, retained supersession links, accepted-body immutability, index freshness | `just adr lint`, `just docs-check`, `just test-all` |
| `adr.py next_number` | Id allocation above tree and reachable history; refuse shallow history | `just adr new`, `just adr supersede` |
| `adr.py render_index` | Current reading surface grouped by status with section routes; retired predecessors as plain ids | readers, publisher |
| `docs/README.md` *Historical recovery* | Single deliberate route back to retired text | S3 readers |

The split is sound. Provenance (`supersedes`) and governance (`superseded-by`, DESIGN citations)
are now distinct in lint, and history is read only by record creation, so lint and publication
still work in a shallow clone. The dependency direction is unchanged: `adr.py` uses
`design_sections` and the `git` CLI, and nothing new depends on `adr.py`.

## 5. Change scenarios

| Scenario | Owner | Observed route | Result |
|---|---|---|---|
| S1 retire a group | replacement ADR, DESIGN citations, index | Predecessors may be missing from `supersedes`. The index renders them as plain ids plus one recovery link. A DESIGN/sections citation of a retired id still fails (`test_a_retired_predecessor_is_history_but_a_missing_governing_record_fails`). A retained superseded record with a missing successor fails. | Holds for an **accepted** replacement. For a *proposed* replacement it does not hold (F01). Plain-text citations outside the collection are unchecked (F03). |
| S2 next id after deletions | `next_number` | `git log --all --name-only -- docs/adr` includes deletions. Shallow detection raises with `git fetch --unshallow`. A non-Git fixture returns `[]`. Tests cover all three. | Holds. The shallow test's lint assertion is weak (F05). |
| S3 resume and recover | index, `docs/README.md` | Current records are grouped by status with section links, so no chain reading is needed for accepted records. The recovery route is pinned to `df52baf` and needs a path, but the index shows only an id. | Partly holds (F02). |
| S4 replace one accepted record | adr skill *Replace and retire* | The clause-by-clause transfer and "never accepts an unapproved target" are clear. Deleting predecessors while the replacement is still proposed is not ruled out, and tooling allows it (F01). | Unclear until F01 is fixed. |

**Probe (S1/S4, 2026-09-25).** I copied `docs/adr`, `docs/design` and `docs/README.md` to the
session scratchpad, rewrote the DESIGN citations `ADR-0010`→`ADR-0025`, deleted
`0010-agent-interface-and-retrieval.md` and ran
`python3 scripts/adr.py --root <copy> index && python3 scripts/adr.py --root <copy> lint --no-git`.
The result was `adr lint: ok (41 records)`. The index lists `ADR-0010` as a retired id under
**Proposed: open choices, not accepted decisions** (ADR-0025) and states "Their surviving meaning
is in the records above". This is the state the repository is in today: ADR-0010 is
`superseded` by the *proposed* ADR-0025, and the plan names ADR-0010 for M3 consolidation.

## 6. Gates

| Gate | Verdict | Evidence / action |
|---|---|---|
| G1 Authority | fail (narrow) | F01: in-force rationale can end up owned only by a proposed record, and tooling accepts it. |
| G2 Semantic fidelity | pass | Frontmatter shape and immutability are unchanged. Reciprocity still holds when both records are retained. |
| G3 Validity | pass | Governing citations and `superseded-by` still resolve (tests). |
| G4 Hidden behavior | pass | Shallow or unreadable history fails loudly. Isolated non-Git behavior is explicit. |
| G5 Consistency and recovery | unresolved | F02: the recovery route cannot locate post-migration retirements. |
| G6 Transformation and reuse | n.a. | No derived data. |
| G7 Truthful claims | fail (narrow) | F01: the generated index sentence is false in the probed state. F04: the DESIGN label is ambiguous. |
| G8 Library leverage | pass | Uses the `git` CLI, with no counter file or catalog. Nothing lighter exists. |
| CI profile gates | n.a. | No code-intelligence facts are involved. |

## 7. Findings

| ID | Finding and consequence | Principles · gate | Evidence | Correction and owner | Closure evidence |
|---|---|---|---|---|---|
| <a id="F01"></a>F01 | Predecessors can be deleted while their replacement is still `proposed` (or `rejected`). Lint passes, and the index then files an in-force decision under "open choices, not accepted decisions" while claiming its surviving meaning is in the records above. The old "supersedes target must exist" error guarded this state; the change removed it without a narrower replacement. | FP-04, DP-01 · A2, G1, G7 | `adr.py` lint loop `for old in a.refs("supersedes")`. Probe above. ADR-0010/ADR-0025 today. | **`adr.py` lint:** `if old not in by_id and a.status in ("proposed", "rejected"): err(a, f"supersedes retired {old} but is {a.status}; accept it before retiring {old}")`. Add a test next to the retired-predecessor test. **ADR-0042 Retirement bullet:** add "Predecessors are deleted only in or after the commit that accepts their replacement." **Skill *Replace and retire*:** change "then delete them" to "then, once the replacement is accepted, delete them". | New test passes. The probe above then fails lint. |
| <a id="F02"></a>F02 | The recovery route handles only material retired before `df52baf`, and it needs a path that the index does not show. A reader who meets `ADR-0010` under *Replaces* after a later retirement cannot follow `git show df52baf:<path>`. Records retired after M4 are not at that revision at all. `df52baf` also predates the `94f4eb3` edits to three reviews, which contradicts plan M0 step 4 (record the pre-removal revision when M4 lands). Separately, `git show` of an LFS path prints the pointer even after `git lfs fetch`. | FP-06 · G5 | `docs/README.md` *Historical recovery*; `git log df52baf..HEAD` shows `94f4eb3` touching reviews | **`docs/README.md`:** lead with a revision-independent pair: find the deleting commit and path with `git log --diff-filter=D --name-only --format=%h -- 'docs/adr/NNNN-*'` (or `-- <path>`), then `git show <commit>^:<path>`. Keep one pre-migration revision only as a convenience, and set it to the parent of the M4 deletion commit when M4 lands. For raw evidence, say `git lfs fetch origin <rev> --include=<path>` then `git show <rev>:<path> \| git lfs smudge`. | A reader recovers a record deleted after M4 by id alone, using only the README commands. |
| <a id="F03"></a>F03 | "Governing references resolve" is enforced only for the architectural collection, `superseded-by` and (separately) the gold freeze. Plain ADR ids that direct current work elsewhere are unchecked, e.g. AGENTS.md cites ADR-0022 as the reason for the flow-facts split, and the plan names ADR-0022 for consolidation. After retirement, agents would be sent to a missing record with no lint signal. | FP-04 · A1 | `rg -o 'ADR-[0-9]{4}'` finds citations in AGENTS.md, binding, pin-check skill and STATUS. `check_gold.py` reads `freeze['adr']` (ADR-0021), which fails loudly but with "status None". | No new lint (operator preference). **ADR-0042 Governing references bullet:** add "and every current instruction surface (AGENTS.md, binding, skills, STATUS, active plan, the gold freeze) that relies on it; a provenance mention is allowed". **Skill *Replace and retire*:** add the step "`rg -n 'ADR-NNNN'` outside `docs/adr` and repoint or remove each governing mention before deleting". | Skill text present. M4 retirement commits show the sweep. |
| <a id="F04"></a>F04 | DESIGN §2 writes "(ADR-0042, **Proposed**, 2026-09-25)". A reader cannot tell whether this is the ADR status or the §D claim label. `CLAIM_RE` is case-sensitive and does not match the bold capitalized form, so the line will not be flagged once ADR-0042 is accepted. Binding §4's "cited in the closing commit" is also ambiguous: is it the commit that closed the finding or the one that removes the row? | DP-22 · G7 | `DESIGN.md` §2 diff; `CLAIM_RE` in `adr.py`; binding §4 diff | **DESIGN §2:** write "(ADR-0042 proposed, 2026-09-25; tooling Tested by `tests/scripts/test_adr.py`, reading-cost benefit **Proposed**)", and change it on acceptance. **Binding §4:** write "once the commit that removes the row cites its closure evidence". | Text inspection. |
| <a id="F05"></a>F05 | `test_new_refuses_a_shallow_history` checks lint by comparing two failing results, because both trees still cite the deleted ADR-0001. It does not show that lint *passes* in a shallow clone with Git checks on, which is the stated property ("lint and publication do not" need history). | DP-23 · G3 | the test's last line | Keep a second record committed in the fixture, re-index and assert `adr.lint(clone, check_git=True) == []`. | The test passes. |

Observations (no action): the index resolves section owners once per record (0.16 s total, not
material). The core template's "Preserve original findings … rather than rewriting history"
does not conflict with the policy, because deletion rewrites nothing. `just adr supersede` still
marks a predecessor `superseded` as soon as a *proposed* draft exists, which is how ADR-0010 and
ADR-0025 reached their state. With F01 in place this is harmless, and M3 already has to resolve
that pair.

Foundations: FP-01 separation is satisfied (provenance vs governance; history is read only at
creation). FP-02 stable contracts are satisfied (frontmatter unchanged). FP-04 authority holds
subject to F01 and F03. FP-06 local reasoning is satisfied for accepted records, subject to F02.

## 8–9. Library fit and alternatives

The `git` CLI through `subprocess` is the right mechanism. A counter file or retired-id catalog
would be extra machinery the plan explicitly rejects, and a pygit2/dulwich dependency buys nothing.
The ADR's options (keep-and-label, archive tree, working set plus Git) are the real ones. The
simplest viable design is the chosen one plus the F01 guard.

## 10. Verification

| Check | Outcome | Command |
|---|---|---|
| ADR tooling tests | passed (17), 2026-09-25 | `uv run --no-project --offline --no-python-downloads --with pytest pytest -q tests/scripts/test_adr.py` |
| ADR lint on the working tree | passed (42 records), 2026-09-25 | `just adr lint` |
| F01 probe | reproduced (lint ok when it should fail), 2026-09-25 | the scratchpad copy command in §5 |
| Docs build/publication of retained corpus | not_run (M5 scope) | `just docs-test`, `just docs-check` |
| Integrated gate | not_run (not end of integrated scope) | `just test-all` |

## 11–12. Dispositions, judgment and decision

| Judgment | Verdict | Evidence |
|---|---|---|
| A1 Localize change | satisfied, with F03 | Retirement touches the replacement, collection citations (enforced) and the regenerated index. Plain-text references elsewhere need the F03 sweep. |
| A2 Encode meaning structurally | violated (narrow), F01 | The provenance-vs-governance distinction is encoded. "Only an accepted replacement can own in-force meaning" is not. |
| A3 Extend through composition | satisfied | Numbering, lint and index each changed in their own function. No new command, register or hook. |

**Bounded change decision:** Accept with changes. Under the template this is a narrow Revise.
The policy and tooling direction are sound, and applying F01–F03 (F04 and F05 are small) makes
them sufficient; no redesign is needed. ADR-0042 can move to `accepted` in the commit that lands
F01–F03.

**Enclosing architecture:** the documentation-lifecycle architecture is accepted for S1–S4 once
F01–F03 land. This is at Proposed/Tested strength for the tooling only. Publication of the retained
corpus (M5) and actual consolidation quality (M3) are not assessed here.

| Priority | Change and component | Findings | Closure |
|---|---|---|---|
| 1 | Lint guard, test, ADR and skill sentence (`adr.py`, ADR-0042, adr skill) | F01 | New test passes; probe fails lint |
| 2 | Revision-independent recovery commands (`docs/README.md`) | F02 | Recovery by id works after a post-M4 deletion |
| 3 | Governing-reference scope and `rg` sweep step (ADR-0042, adr skill) | F03 | Text present; M4 commits show the sweep |
| 4 | Label and wording (DESIGN §2, binding §4); shallow-lint test | F04, F05 | Inspection; test passes |

Disposition owner: the migration plan's M2 packet (`docs/plans/legacy-documentation-migration_2026-09-25.md`).
