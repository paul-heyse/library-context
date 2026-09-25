# Execution plan: current architecture and legacy documentation migration

**Date:** 2026-09-25. **Status:** Proposed; ready for implementation sequencing.
**Scope:** documentation, process instructions and their existing tooling. Product fixes belong to
the [behavioral forward plan](behavioral-model-forward-plan_2026-09-24.md).
**Claim strength:** repository observations below are **Interface-checked, 2026-09-25**;
the target and migration are **Proposed**. Writing this plan does not execute the migration.

## 1. Outcome and operator direction

Leave a small, coherent working set that explains **what the system is, why its current design
was chosen, what remains uncertain, and what to do next**. A developer or agent should reach
that context through the architecture map and the relevant owner, without reconstructing the
project's decision history.

The operator establishes these migration rules:

- `behavioral-model-forward-plan_2026-09-24.md` is the sole current **product execution plan**.
  Revise it against the two reasoning reviews below; keep its path stable.
- Earlier product plans are inactive: complete, superseded, or abandoned as applicable. Missing
  retirement labels do not make them active. **Retiring a plan is not proof that its work passed
  or was implemented.** Preserve only obligations that still belong in the current scope.
- Current rationale matters more than the sequence of early decisions. Preserve useful reasons
  and constraints; remove obsolete alternatives, amendment chains and implementation diaries
  from ordinary reading context.
- Git holds retired material. Do not create another archive tree, historical book, migration
  database, permanent disposition spreadsheet, or documentation-to-code proof system.
- Preserve known defects and incomplete qualification. Cleanup must not turn a target into an
  implementation claim, an accepted ADR into a test receipt, or an old plan into evidence of closure.

This temporary plan owns the documentation migration only. It neither replaces the product
forward plan nor schedules production changes itself. Its eventual deletion is part of completion.

## 2. Inputs and precedence

### 2.1 Required execution inputs

| Input | Role in this migration |
|---|---|
| [Behavioral forward plan](behavioral-model-forward-plan_2026-09-24.md) | Current product scope, remaining Stage 3 work, Stage 4/5 intent, deferred triggers and ARC-01–03 disposition |
| `docs/design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md` | Original F01–F16; open findings and candidate mechanisms |
| `docs/design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md` | Qualifications to the original findings in §7.1, eight additional findings, and revised priorities in §12 |
| [Architecture map](../design/README.md), [DESIGN](../design/DESIGN.md), [ADR index](../adr/README.md) | Present authority, including accepted targets and inconsistencies to resolve |
| [Documentation implementation plan](architectural-documentation-and-search_2026-09-25.md), [ADR-0041](../adr/0041-architectural-documentation.md), [publishing operations](../publishing.md) | Already implemented publication/section infrastructure; retain its useful mechanisms |
| [ADR-0040](../adr/0040-architecture-change-scenarios.md), [binding](../design_review/design_principles/binding/library-context.md) | Architecture principles, proportional review, evidence labels and one current disposition owner |
| [STATUS](../../STATUS.md) | Actual checkpoint and bounded qualification receipts; preserve concurrent edits |

The operator confirms that the reasoning reviews examined current production code. Inspection
found no `crates/` or `python/` changes between production checkpoint `acbcee5` and documentation
checkpoint `05b1993`. Both reviews therefore remain relevant inputs; do not rerun the entire
review merely because documentation moved. Refresh a finding only if its affected code changes,
its evidence conflicts, or a planning choice needs a narrower answer.

Use the follow-up's qualifications when the reviews disagree. Reviews establish dated evidence
and recommendations; they do not silently amend accepted contracts. Current code establishes
implemented behavior, including defects. Accepted design establishes intended contracts, including
unimplemented ones. Show that distinction where they differ. Resolve a genuine contract choice
through the existing ADR/review route; do not normalize an implementation bug into the target.

### 2.2 Observed migration gaps

The inspected tree contains 41 numbered ADRs, four plan files, 107 review/evaluation/deviation
Markdown files, and 22 evidence Markdown files. These are a dated inventory, not size limits.

- The publisher discovers all reviews, evidence READMEs, initial research and retired standards.
  Putting them in **History** still leaves them in navigation and **Everything** search.
- Accepted ADRs automatically enter **Current**, even when their useful meaning is buried in
  amendments or later partial replacements. ADR-0010 and ADR-0022 particularly require extracting
  surviving clauses rather than interpreting whole-record status as the complete answer.
- DESIGN still carries substantial historical implementation narrative and duplicated details.
  Only §3.9 and §9.9 have focused owner pages so far.
- The forward plan repeats execution/checkpoint material and still delegates live scope to a
  superseded plan: T1–T9, the D-7 model list, Stage 4's §15, and parts of Stage 5. Those live
  requirements must become readable without opening that plan.
- The completed documentation implementation plan is still selected as current work.
- ADR lint currently requires historical supersession targets to remain present, validates
  every loaded record's section references, and allocates IDs from existing files only. Deleting
  records without changing these semantics can break lint or reuse an old ID.

## 3. Target working set and authority

| Surface | Keep here | Remove or route elsewhere |
|---|---|---|
| `STATUS.md` | Short actual checkpoint, qualification boundary, links to current work | Per-session diaries and copied finding tables |
| `docs/README.md` | Task routes; one short Git recovery instruction | Historical reading lists and alternate resume plans |
| `docs/design/README.md` | Responsibility/dependency map and owner links | Duplicated contracts or readiness claims |
| `docs/design/DESIGN.md` | Scope, §B1–§B14 and compact section navigation | Detailed subdomain prose moved to owners; revision diary |
| `docs/design/sections/` | Current contracts, accepted targets, explicit known limitations and executable owner pointers | Slice-by-slice progress and repeated field/codebook definitions |
| `docs/adr/` | Current rationale and genuinely live proposals, each understandable directly | Superseded records and obsolete/mixed baselines after consolidation |
| Product forward plan | Remaining queue, dependencies, single current finding disposition, deferred triggers and exit criteria | Historical scheduling maps, closed-row accumulation and dependencies on retired plans |
| Reviews/evidence | The two reasoning reviews and other material with a named current consumer | Completed slice reviews, obsolete assessments and unused probe packages |
| Active standard/process skills | One declared standard and one consistent workflow | Old directives/templates and alternate process descriptions |
| Pins, publishing, library guidance | Their current operational instructions | Implementation-project narrative already represented by the working tools |

The two reasoning reviews remain useful supporting **Reference** while their findings are open.
They do not need to be read in full for every task: the forward plan carries concise actionable
dispositions and links to the exact finding/evidence. Other reviews remain only if an open issue
or current decision still needs them. A retained document needs a real consumer, not a historical
importance argument.

There is no obligation to retain one ADR for each old record, one page for each crate, or one
document for each review slice. Prefer a coherent responsibility or decision that can be understood
locally. Keep the existing six foundations and A1–A3 judgments; this is not a new principles system.

## 4. Execution order and boundaries

| Packet | Responsible role | Depends on | Deliverable / completion boundary |
|---|---|---|---|
| M0 — preservation and bounded triage | Migration executor | — | Recoverable inputs and exact removal candidates; no deletions yet |
| M1 — reconcile current product work | Product planner | M0 | Revised forward plan, all current findings accounted for, obsolete-plan dependencies removed |
| M2 — enable a current rationale baseline | Process/tooling executor | M0 | Accepted lifecycle policy, focused ADR tooling changes and aligned process instructions |
| M3 — consolidate architecture and rationale | Architecture editor | M1, M2 | Focused owners and current rationale; accepted/implemented/proposed distinctions preserved |
| M4 — remove legacy material | Migration executor | M1–M3 | Obsolete files removed from working tree, useful obligations and evidence preserved |
| M5 — align discovery and publication | Documentation tooling executor | M3, M4 | Source routes, navigation and search expose the curated working set |
| M6 — verify and finish | Migration executor / reviewer | M1–M5 | Focused acceptance, concise handoff and retirement of this temporary plan |

These are sequential handoff packets, not a request to start parallel writers in the shared tree.
Keep production changes out of these commits. If review reconciliation reveals a product decision
that cannot be made from current evidence, put that decision and its trigger in the forward plan;
retain the smallest needed rationale/evidence until it is resolved. Independent cleanup can continue.

### M0 — preserve inputs and establish the removal set

1. Read `STATUS.md`, the three primary plan/review inputs and the relevant owner pages. Inspect
   `git status`, the index and current branch before edits. Verify that later product work has
   not invalidated the stated baseline.
2. Preserve uncommitted work explicitly. At planning time both reasoning reviews, the
   `2026-09-25_bdd-decision-apis/` evidence and three `2026-09-25_library-fit-followup-*` evidence
   directories were untracked; STATUS also contained concurrent changes. These are not recoverable
   from Git merely because they are visible locally. Keep them intact and include the intended
   review/evidence artifacts in a scoped checkpoint before any associated retirement. Do not
   sweep unrelated untracked files into a commit.
3. Confirm raw evidence is covered by existing LFS rules and its local objects exist before
   retiring it. Do not remove environments, build caches or ignored capability corpora as part
   of this documentation task. A local checkpoint is not a remote backup; any later publication
   must include required LFS objects through the normal workflow.
4. Establish a pre-removal commit containing the material to be retired. Record its full revision
   once in the documentation recovery note when M4 lands. Use Git's existing paths/history for
   recovery; do not copy all content or create a per-file archive index.
5. Triage by current consumers and references, not by rereading every old review. Use the active
   plan's open dispositions, current authority links, `rg` and file lists. Read a legacy body only
   to recover a still-needed requirement, reason, evidence boundary or unresolved issue. Keep a
   short temporary packet checklist in this plan or the working diff; delete it at completion.

**Exit:** relevant uncommitted inputs remain intact; every deletion candidate has a recoverable
version and either no current consumer or an identified replacement. No production tests are needed.

### M1 — revise the existing product forward plan

Edit `behavioral-model-forward-plan_2026-09-24.md` in place. It remains the single product plan.

1. Replace the opening history with actual current state, scope, next dependency and explicit
   qualification limits. Keep necessary positive/withholding behavior constraints, Stage 3 exit,
   Stage 4/5 scope and deferred triggers. Collapse repeated queue/contract/checkpoint prose.
2. Bring forward still-relevant content currently delegated to the pivot/holistic plans. Restate
   execution obligations in this plan and architectural meaning in its owner. Remove references
   that require reading retired plan sections to know the current requirements.
3. Reconcile all original F01–F16, follow-up F01–F08 and existing ARC-01–03. Use qualified display
   IDs such as `RF/F01` and `RFU/F01`, with links to original IDs; do not rename the source findings.
   Existing `ARC-01`–`ARC-03` anchors remain stable. Findings can share one work item without losing
   their distinct failure, scope or closure criterion.
4. For each item state the responsible component, concise consequence, current disposition,
   dependency/trigger, intended correction or unresolved choice, and meaningful closure check.
   `Deferred` keeps a trigger. `Closed` requires correction evidence; moving a row is not closure.
   Mark the two reviews' §11 tables as transferred, linking to the plan rather than maintaining
   a second changing status table. Preserve the reviews' dated findings and conclusions.
5. Apply the follow-up's priority order, including prerequisites and the qualifications below.
   Library adoption remains a decision to establish on the owned contract. Do not schedule every
   suggested mechanism as an already accepted implementation requirement.

This is the **migration coverage map**, not a second progress register:

| Input IDs | Required treatment in the forward plan |
|---|---|
| ARC-01; RF/F13; RFU/F02, F03 | Shared native proof-kind admission and semantic decoding; preserve facet verdicts and served support closure. Arrow IPC alone does not make separate semantic decoders agree. Distinguish operation responses, evidence projection and native admission closure. |
| RF/F09; RFU/F05 | One resolved access contract, preserving literal/dynamic status and provenance across bare, qualified and aliased builtins. A literal-node fix alone does not close the reproduced no-read defect. Include real-provider and publication/serving controls. |
| ARC-02, ARC-03; RF/F05 | Explicit summary inputs, typed refusals, bounded cyclic reach fixed point and remaining predecessor/exit witnesses. Keep query-order and finite-base controls; missing contributions do not establish every broader consequence conjectured by the original review. |
| RF/F01, F02; RFU/F01 | Decision-only BDD operations, effective support and aggregate preparation/retention bounds. A bounded refusal is unknown; per-root limits do not bound total hydrated retention. |
| RF/F03, F04, F12 | Restriction and primitive theory with independent controls. Restriction is not the factor in a factor/quotient decomposition; non-minimal valid links are not automatically incorrect. Membership/integer equality alone cannot complete operation-wide Q09. |
| RFU/F07, F06 | Actual analyzer-readable corpus identity and canonical cache-fill admission/readback. Do not generalize the corpus finding into a claim that ordinary dependencies are entirely unhashed. |
| RFU/F04, F08 | Typed RCA identity/modality before extending that optional pipeline; one deployment spec/identity route before the next relevant endpoint/model upgrade. Preserve source-inspection versus executed-evidence distinctions. |
| RF/F06 | Compare engines at the first shared recursive contract after ARC-02/03. Keep bounded native worklist, Ascent and datafrog as evidence-based alternatives; timeout support is not a deterministic work budget. No unconditional Ascent adoption or arbitrary three-loop waiting rule. |
| RF/F07, F08 | Resolve closure row identity/provenance and actual SCC/topological ownership before choosing SQL or graph algorithm substitutions. Do not prescribe blind `UNION` or `kosaraju_scc` replacement. |
| RF/F10, F11, F14, F15 | Resolved function flags, explicit analyzer settings, input-order determinism and derivation ownership. Preserve the flags schema-migration boundary and the proportional next-edit trigger for consolidation. |
| RF/F16 | Correct §6.3's schema-evolution claim to distinguish implemented strict verification from a possible evolution mechanism. This is documentation correction, not evidence that a product migration occurred. |

Also carry the source review's §8.1 tool triggers: fcars remains an independent oracle; neither
z3 nor OxiDD becomes a dependency merely because it appeared in a review. Keep the follow-up's
qualified datafrog/Ascent capability assessment with the eventual engine decision.

**Exit:** the active plan accounts for all 27 named findings (16 + 8 + 3), without implying 27
separate implementations; one table owns their changing disposition. No live scope depends on
an inactive plan. Stage 3 remains incomplete and integrated qualification remains outstanding.

### M2 — make ADR retirement compatible with current rationale

Use the ADR skill for a concise new **documentation lifecycle** decision and a proportionate
design/target review under the existing binding. Allocate the next available ID at execution time
(0042 was next at inspection). This decision extends ADR-0041; it must not claim to supersede
every product decision merely because old files will be retired.

The policy should establish:

- Architecture pages own current contracts and targets; current ADRs own reasons and meaningful
  alternatives. Current readers must not need supersession chains to interpret either.
- Consolidate a mixed or obsolete ADR into a concise replacement when it carries useful current
  rationale. A still-current, self-contained ADR can remain regardless of age. Do not mechanically
  reissue every record or preserve one record for every implementation increment.
- Retire an ADR only after its surviving meaning has a current owner or is explicitly rejected
  or no longer in scope. Partial supersession is resolved clause by clause. A new baseline does
  not accept an unapproved target or silently choose an unresolved product alternative.
- Accepted records that remain are immutable under the existing rule. Use replacements, not
  rewritten accepted bodies. Retired records are recovered from Git; their former acceptance is
  not an obligation to keep them in the checkout. Proposed records are retained only for live choices.
- Current governing references must resolve in the working tree. Historical supersession metadata
  is provenance, not a dependency requiring retired files to remain. No permanent retirement register.
- Ordinary closed reviews, completed plans and obsolete evidence can be retired after current
  obligations and useful rationale are carried forward. Finding IDs and surviving evidence links
  remain intelligible; no requirement to retain every closed row or every review forever.
- A moved **live** section keeps its stable ID and useful relocation fragment. A retired document
  or obsolete section does not need a permanent empty stub. Never reuse a retired section ID.

Implement this in the existing owners, with focused tests:

| Owner | Change and verification |
|---|---|
| `scripts/adr.py`, `tests/scripts/test_adr.py` | Separate live governance from historical supersession. Keep strict section resolution, current governing ADR existence, metadata shape and accepted-body immutability. Validate reciprocal supersession when both records are retained; permit missing older `supersedes` targets as historical references. A retained superseded record's `superseded-by` must still lead to a retained successor; normally remove such obsolete records. Creation/supersede commands still reject a nonexistent requested live predecessor. Test that a missing governing ADR fails while a retired historical predecessor does not. |
| ADR ID allocation | Allocate above the maximum of current records and reachable Git history of numbered ADR paths, including deleted records. Inspect history only when creating a record; ordinary lint/build must work in a shallow checkout. If record creation cannot establish a safe historical maximum, fail with a clear history-fetch instruction instead of silently reusing an ID. Keep isolated non-Git test fixtures supported. No persistent counter or retired-ID catalog. |
| Generated ADR index | Present current accepted rationale and live proposals, with topic/section routes. Do not append a retired-record catalog. Missing historical predecessors render as historical identifiers with the single Git recovery route, never broken relative links. |
| `AGENTS.md`, ADR/handoff/design-review skills, reviewer instructions, binding, DESIGN process paragraphs | Describe the same ownership/lifecycle. Replace append-only revision-history and keep-all-closed-rows expectations with current-state maintenance. Preserve evidence discipline and review cadence. Remove old initial-plan reading/preservation instructions when that input is retired in M4. |
| Section resolver / publisher | Preserve one resolver and existing stable anchors. Do not add a source-code conformance scanner or require per-section proof metadata. Change resolver behavior only if a real move/retirement case needs it. |

The new policy changes documentation lifecycle, not Arrow schemas, provider semantics or the
product dependency family. Follow the existing ADR review requirement; do not manufacture a
second review program for every file deletion.

**Exit:** deleting a properly retired predecessor cannot break current ADR governance or reuse an
ID; missing current contracts still fail. Skills no longer instruct agents to rebuild retired history.

### M3 — consolidate architecture by responsibility, then its rationale

Keep §B1–§B14 in DESIGN. Move existing numbered sections without renumbering them, using the
shared resolver and the ADR-0041 relocation pattern for live content. The proposed file map is:

| Owner file under `docs/design/sections/` | Existing section IDs / responsibility |
|---|---|
| `facts-and-identity.md` | §3 through §3.8: fact authority, identity, fidelity, coverage, graph catalog |
| `behavior-model.md` (existing) | §3.9 and descendants: places, conditions, verdicts and coverage meaning |
| `acquisition-and-extraction.md` | §4 and descendants: pinned inputs, provider boundaries and pipeline orchestration |
| `storage-and-publication.md` | §5–§6: projections, canonical tables, validation/publication and generation boundary |
| `analytics.md` | §9 through §9.8: analytic contracts, algorithm ownership, FCA/RCA and determinism |
| `behavioral-analysis.md` (existing) | §9.9 and descendants: models, summaries and capability construction |
| `synthesis-and-serving.md` | §10–§11: claim construction, support closure, serving and embedding contracts |
| `validation-and-evaluation.md` | §8 and §12: independent checks, evaluation meaning and acceptance boundaries |

Keep §1, §2, a short §7 route to `docs/pins.md`, and §13's durable non-goals/deferral rationale in
DESIGN. The plan owns scheduling triggers; avoid copying a changing deferred-work table into §13.
Group or split the proposed files further only when a real reading/change boundary warrants it.

Each owner should answer, in ordinary prose: responsibility and consumers; important inputs,
outputs and invariants; accepted design and its reason; implemented limitations and live target;
extension points and dependency direction; where authoritative declarations and tests live.
Use a few links to real owners. Do not duplicate every field, invariant expression or test case.

Carry rationale forward in coherent decisions rather than slice chronology:

| Existing ADR material | Consolidation instruction |
|---|---|
| 0001–0011 and 0023 | Retire obsolete early process/topology/publication/interface/analytics accounts after recovering any surviving rationale. Inspect 0002/0005 and the surviving clauses of 0010/0011 rather than assuming their age or status makes every clause irrelevant. Current pins/contracts win over old version narratives. |
| 0012–0019 | Retain only records that remain self-contained and accurate for in-process frontends, pinned acquisition, graph catalogs, source corpus, allocator and snapshot/result ownership. Consolidate amendment-heavy or overlapping material by responsibility; retain real tradeoffs without the earlier sequence. |
| 0021–0022 and 0027–0039 | Consolidate current behavioral scope, attribution/coverage, condition/proof identity, model and exit/summary rationale. Remove the chain of intermediate handler/finalizer/predecessor decisions once the current basis and limitations are explicit. Preserve the parts of 0022 that survived its partial replacement. |
| Proposed 0020, 0024, 0025, 0028 | Resolve each proposal's current role. Keep a live undecided choice or replace it with an explicitly Proposed current alternative. Do not promote it to accepted because related code exists. Retire abandoned/superseded proposals once no unresolved obligation is lost. |
| 0026, 0040, 0041 | Keep their useful current build-loop, architecture-review and publishing decisions. Unrelated decisions survive process-clause replacement. Link to the new lifecycle decision where it narrows historical retention; avoid recreating these three records solely for numbering uniformity. |

Before replacing an ADR, inspect its **current governed sections and consumers**, not just its
supersession metadata. Use a replacement record when current rationale needs consolidation;
it can summarize multiple predecessors without reproducing their history. No numeric ADR-reduction
target is a correctness criterion. The criterion is that retained rationale is current and usable
without reading retired records.

Explicitly repair misleading current prose exposed by the reviews, including §6.3 evolution,
§11.1's byte/token guarantee and cache-result authority, corpus input closure, SCC/recursive engine
ownership, and typed serving/RCA claims. Keep supported intent and identify implementation gaps
with concise links to the forward plan. A source-traced defect remains a defect after editing prose.

Remove the DESIGN revision diary and long per-slice implementation/measurement narratives.
Keep only a dated, scoped result that still supports a current decision; move necessary execution
state to the forward plan/STATUS. Git preserves the replaced wording.

**Exit:** an agent can understand each current responsibility and its rationale from its owner
and relevant current ADRs. Every moved live section resolves once, current decisions govern the
right sections, and unresolved findings are visible without reading historical chains.

### M4 — remove legacy material after transfer

Use explicit file lists derived from M0. Do not delete by date alone or use broad cleanup commands
over the shared tree. The defaults below cover the complete documentation corpus:

| Existing material | Final disposition |
|---|---|
| `docs/plans/behavioral-model-pivot-plan_2026-09-24.md` | Retire after M1 internalizes still-live principles, models, Stage 4/5 scope and deferred obligations. |
| `docs/plans/behavioral-model-forward-handoff_2026-09-24.md` | Retire after relevant checkpoint information is in STATUS/current plan. |
| `docs/plans/architectural-documentation-and-search_2026-09-25.md` | Retire completed implementation plan after useful operating instructions/limitations are in `docs/publishing.md`; retain a concise dated local qualification receipt, not its execution log. |
| `docs/lightweight_architectural_documentation_and_search_proposal.md` | Retire adopted proposal after current rationale is covered by ADR-0041 and the lifecycle decision. |
| `docs/lightweight_architectural_documentation_and_search_other_repo.md` | Retire comparison input after ensuring its untracked copy is deliberately preserved in the scoped checkpoint. No ongoing authority in this repo. |
| `docs/behavioral_model_pivot.md`, `docs/full_cpg_pipeline_external_review.md` | Extract any unresolved obligation already needed by the current plan; retire research/assessment inputs. Do not reopen old work merely to produce completion certificates. |
| `docs/initial_plan/Initial_plan.md`, `DISPOSITION.md` | Retire the original research and its historical mapping after current scope is self-contained and M2 updates the special AGENTS instructions. Do not rewrite the research to make it agree with today's design. |
| Old review, deviation and structured-evaluation files | Retire after needed open obligations and useful scoped results have current owners. Preserve a still-relevant evaluation baseline only if a named current comparison consumes it. Earlier Accept results do not qualify the current tree. |
| The two reasoning reviews | Retain as current supporting Reference with transferred disposition pointers after M1. Retire later when no active finding/decision needs their detailed evidence. |
| Two core-3.0 calibration reviews | Retire after ARC-01–03 retain their consequence, scope, closure checks and recoverable evidence references in the forward plan. Keep a review temporarily if an ARC item still needs otherwise unrepresented evidence. |
| Evidence directories | Retain the exact packages consumed by open findings or current decisions; prune other packages as units after recoverability is established. A README and its raw artifacts stay coherent. An ignored executable/build output is never treated as an archival prerequisite. |
| Retired ADRs | Remove under M2/M3 policy; regenerate the current index. Update current governing references first. |
| Six legacy root Markdown files in `docs/design_review/design_principles/` | Retire `ADDENDUM.md`, `AGENT_DESIGN_DIRECTIVE.md`, `DATA_MODEL_DESIGN_CHARTER.md`, `DESIGN_REVIEW_TEMPLATE.md`, `REVIEW_REFERENCE.md`, and `rust_code_intelligence_data_graph_guidelines.md` after checking active standard/profile/binding already own the required meaning. Keep only the layers selected by `standard.toml`. |
| Root/docs READMEs, publishing, pins, library guide, tracked process skills | Keep; update routes and stale authority claims. Preserve ignored capability reference collections and original production fixtures. |

Do not replace every removed file with a stub. Retired review/finding links needed for a live
issue can point to the preserved revision, while the issue's current disposition remains local.
For current architectural section moves, retain the already agreed stable fragment behavior.

Add one short **Historical recovery** note to `docs/README.md`: the pre-removal revision and
how to inspect a path at that revision or use Git history. It is a recovery aid, not a reading
prerequisite or an archive index. Ensure Markdown links to deleted files are rewritten or removed
across retained docs, process instructions and any production comments that actually cite them;
comment-only link repair does not authorize unrelated product refactoring.

**Exit:** no obsolete document is kept solely to satisfy a stale link, supersession chain or
generic publisher glob. All surviving open issues still have one visible owner and usable evidence.

### M5 — publish the curated system

1. Update `docs/site.toml` using its existing publication/collection mechanism. Remove the completed
   docs plan from `current_work`; select only the product plan and this migration plan while active.
   After M6, only the product plan remains. Do not encode finding progress or architecture in TOML.
2. Stop publishing initial research, retired standards and retired decision/review/plan collections.
   Retained supporting reviews/evidence are **Reference**. Accepted current rationale remains
   **Current**, live proposals remain clearly labeled, and default search remains **Current**.
3. Keep **Everything** bounded to the retained published corpus. Historical recovery through Git
   is sufficient: hide/remove the History option when no historical pages remain, with a focused
   UI/metadata change if required. Do not preserve an archive corpus to populate a search filter.
4. Update `docs/README.md`, architecture map, root README, STATUS routes, skill links and the generated
   ADR index. Route questions about current reasons to the current owner/ADR; earlier decisions
   belong behind the single optional recovery note.
5. Preserve mdBook/Pagefind, isolated docs commands, offline link checking, revision honesty,
   prefix support, ignored local capability handling, and artifact CI. No tool replacement, remote
   deployment or new product gate is needed. Update path filters only if actual moved owners require it.

**Exit:** navigation and default search make the current model easy to find. Neither source routes
nor Everything require agents to sort through retired plans and contradictory early decisions.

### M6 — focused acceptance and final handoff

Use existing tests and commands. Add tests only for changed lifecycle/ID allocation and real
publication regressions; do not write assertions about arbitrary document counts or wording.

| Check | Expected evidence |
|---|---|
| `just docs-test` | ADR lifecycle/history numbering, section ownership/relocation, discovery and link transport tests pass, including the added retirement cases. |
| `just adr index`; `just adr lint`; `just lint-agents` | Current references and generated index agree; process routes resolve. These are also covered where applicable by the docs checks. |
| `just docs-check` | A fresh complete site builds and offline links/fragments pass over the curated corpus. |
| Existing focused Ruff/pyrefly commands for changed Python tooling | Changed documentation scripts remain lint/type clean; no product-wide check is needed. |
| Browser checks on the built site | Current default, Reference/Everything, one relocated section, one retained review/finding and one prefixed URL behave correctly; no empty History control or stale legacy page. |
| Diff and source-route inspection | No unexpected production/schema/pin changes, lost current finding, copied status register, stale deleted target or fabricated completion claim. |
| Proportionate assembled design review | An agent can carry out the scenarios below with bounded current context; review assesses the documentation system, not product qualification. |

Acceptance scenarios:

1. Resume product work: STATUS → forward plan identifies next dependency, open findings and
   qualification boundaries without opening an older plan.
2. Change an access/summary/serving contract: owner → current rationale → exact open finding
   explains intended semantics and the known defect without a chain of superseded ADRs.
3. Add a model or rendering: the relevant contract and composition boundary are clear; ordinary
   implementation does not require a new historical document or bespoke proof packet.
4. Revisit a library choice: the current consumer, rejected viable alternative and trigger are
   available; an unrun Ascent/solver proposal cannot be mistaken for an adopted dependency.
5. Recover history deliberately: the documented Git route retrieves a retired file; its absence
   does not break current lint, navigation or fresh site builds. Do not require fetching all LFS
   evidence during ordinary publication.

Report `passed`, `failed`, `blocked` or `not_run` with the actual command/scope. Product
`just test-all`, `just pilot`, Q01/Q03/Q05/Q09 and wheel/native qualification are **not_run for
this documentation migration**; their existing missing acceptance remains in the product plan.
No cleanup receipt closes RF, RFU or ARC product findings.

At completion, use the handoff skill: concise current state, focused docs receipts, unresolved
product boundary, and the forward-plan link. Put durable operating changes in publishing/process
owners and a short migration result in STATUS. Remove this completed plan from `current_work`
and the working tree in the final cleanup commit; Git retains execution detail. Run the final
docs checks against that resulting tree. Keep commits scoped, preserve concurrent work, and do
not push or deploy as a side effect of documentation cleanup.

## 5. Ongoing lifecycle after migration

- A change updates the owner whose meaning changed. A routine internal implementation change
  needs no architecture edit, new ADR or review artifact beyond the existing proportional process.
- A new decision carries its current reason and real alternatives. On replacement, transfer
  surviving meaning and retire the old record when it has no live consumer; no chain-reading duty.
- A review has a current consumer while it supplies an open finding or needed decision evidence.
  Once scheduled, the plan owns disposition. When its useful content is absorbed and findings no
  longer need the source, retire the review/evidence in the relevant completion commit.
- Finishing or superseding a plan transfers remaining obligations, updates STATUS/routes and
  removes the old plan from the working tree. Unlabeled old plans do not silently become current.
- Keep a dated qualification statement only for its actual scope and useful comparison. Replace
  a checkpoint instead of appending another session narrative.
- Review cleanup at meaningful scope completion or when replacing an artifact. No scheduled
  housekeeping gate, age-based deletion bot, document-count quota or permanent migration register.

The resulting maintenance task is small: keep current meaning and current work coherent, and
retire a document when its real consumer has gone.
