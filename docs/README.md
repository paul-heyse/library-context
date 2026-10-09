# Documentation

Start with the task, then read the owning contract and adjacent consumer. Expand scope when a
contradiction or dependency requires it; the whole corpus is not the default reading context.

| Task | Start here |
|---|---|
| Understand the system | [Architecture map](design/README.md) → relevant owner |
| Resume work | [STATUS](../STATUS.md) → [persisted execution plan](plans/persisted-graph-execution-plan_2026-10-07.md) (current compiler/consumer pivot) → [graph-native replacement coordinator](plans/graph-native-pivot-plan_2026-10-05.md) (hard pivot, audit remediation, contracts, dependencies and acceptance) → [forward plan](plans/behavioral-model-forward-plan_2026-09-24.md) (product/research queue) |
| Change conditions, models or summaries | [Semantic owner §15.6–§15.9](design/sections/semantic-model.md#section-15-6) → affected model producer/consumer → [model/compiler plan](plans/graph-native-model-compiler-plan_2026-10-05.md); the [graph-native coordinator §7](plans/graph-native-pivot-plan_2026-10-05.md#7-sole-finding-disposition-and-optional-capabilities) owns current pivot disposition |
| Change schemas or identity | [Facts and identity](design/sections/facts-and-identity.md) → [model owner](../crates/lctx-model/src/lib.rs) → governing ADR |
| Change serving, retrieval or embeddings | [Synthesis and serving](design/sections/synthesis-and-serving.md) → [storage and publication §6.4](design/sections/storage-and-publication.md) |
| Review the proposed retrieval and evaluation architecture | [External proposal](library_context_evidence_retrieval_spec_evaluation_external_proposal_and_review.md) → [target design review](design_review/reviews/design_review_evidence-retrieval-and-programmatic-evaluation_2026-10-06.md): primary programmatic evaluation, outer agentic feedback, capability fit; its findings and approved rule changes now route through the [extension coordinator](plans/evidence-retrieval-and-evaluation-plan_2026-10-06.md) |
| Implement evidence/retrieval and primary evaluation | [Extension coordinator](plans/evidence-retrieval-and-evaluation-plan_2026-10-06.md) → [programmatic evaluation plan](plans/programmatic-evaluation-plan_2026-10-06.md) and ER extensions in the four graph-native supporting plans; programmatic primary loop with versioned outer feedback; [revised plan review](design_review/reviews/design_review_evidence-retrieval-and-evaluation-plan_2026-10-06.md) accepts the target at Proposed strength; the [assembled implementation review](design_review/reviews/design_review_evidence-retrieval-evaluation-implementation_2026-10-06.md) supplies scheduled correction findings at coordinator §6 |
| Improve compilation and comparable consumer inefficiencies | [Persisted graph execution plan](plans/persisted-graph-execution-plan_2026-10-07.md): native compilation, exact completed-state foundations, shared access/conversion/preparation, consumer migration and direct sealing; owns [catalog-speed review](design_review/reviews/design_review_catalog-compilation-speed_2026-10-07.md) F01–F05 and additional source-inspected corrections |
| Improve Rust build and test compilation times | [Compilation-cost coordinator](plans/rust-compilation-costs-plan_2026-10-08.md) owns F01–F04 disposition, shared mechanics, provenance, profiles and harnesses; its [coroutine companion](plans/rust-coroutine-compilation-amplification-plan_2026-10-08.md) develops structural-first containment and confirmed recurring loader/producer corrections from the [coroutine review](design_review/reviews/design_review_rust-coroutine-compilation-amplification_2026-10-08.md). Corrections, local-profile qualification and final optimized acceptance remain open, preserving available parallelism |
| Inspect a slow compiler while it runs and after it finishes | [Compilation-cost diagnostics](plans/rust-compilation-costs-plan_2026-10-08.md#11-opt-in-compilation-diagnostics): run-owned Cargo analysis, selected-unit rustc query/LLVM timelines, monomorphization estimates and rotated native CPU samples; commands in [AGENTS.md](../AGENTS.md#commands) |
| Execute and qualify the persisted-execution corrections | [Correction plan](plans/persisted-execution-corrections-plan_2026-10-07.md) develops all [correction-causes findings](design_review/reviews/design_review_persisted-execution-correction-causes_2026-10-07.md); [persisted coordinator §8](plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition) owns disposition. Both rule changes are accepted; production corrections are integrated and focused qualification is in progress. The [integrated review](design_review/reviews/design_review_persisted-execution-corrections-integrated_2026-10-08.md) accepts the examined source architecture, including physical hydration; execution acceptance remains open |
| Improve agent workspace effectiveness | [Proposal](plans/agent-workspace-effectiveness-plan_2026-10-07.md) → [capability review](design_review/reviews/design_review_agent-workspace-effectiveness-capabilities_2026-10-07.md): corrections, removals and additions for capable agents; recommendations remain Proposed |
| Implement the graph-native replacement | [Coordinator](plans/graph-native-pivot-plan_2026-10-05.md) → its model/compiler, SurrealDB realization, native serving and projection plans; the two nominated target/capability reviews are its design basis |
| Build the differentiated API/evidence product | [Target design §14](design/sections/api-and-evidence-product.md) → [current PR0–PR6 queue](plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) |
| Deploy native persistence or serving | [Graph-native coordinator](plans/graph-native-pivot-plan_2026-10-05.md) → [native realization plan](plans/graph-native-surrealdb-realization-plan_2026-10-05.md) and [serving plan](plans/graph-native-serving-plan_2026-10-05.md); these implemented native consumers follow the user-accepted compiler stage; targeted receipts are in the coordinator |
| Review a design or implementation | [Review skill](../.claude/skills/design-review/SKILL.md) → [repository binding](design_review/design_principles/binding/library-context.md); for library fit, start from the [local capability skills](../.claude/skills/README.md) |
| Pivot testing, validation or oracles | [STATUS](../STATUS.md) names the current verification boundary and its owning plan; commands are in [AGENTS.md, Commands](../AGENTS.md#commands) (`just verify`, `just fixture`, ADR-0134) |
| Operate the native store | [SurrealDB runbook](surrealdb.md): runtime configuration, publication, selection, backup/restore and reader quiescence; operator activation is explicit |
| Research a library capability | [Local capability skills](../.claude/skills/README.md) → Context7, then the `library-research` route, at the locked version; holds are in [Pins](pins.md) |
| Understand why a decision was made | Owner section's `> Decision:` line → [ADR index](adr/README.md) |
| Maintain this site | [Publishing operations](publishing.md) |

## Repository map

Paths in this reference table are relative to the repository root.

| Path | What |
|---|---|
| `docs/design/DESIGN.md`, `docs/design/sections/` | Architectural collection; stable § IDs. DESIGN §2 holds §B1–§B14 |
| `docs/README.md`, `docs/publishing.md` | Task routes and isolated documentation commands; site navigation/search is derived |
| `docs/plans/` | The accepted graph-native replacement series, implemented-baseline cutover/assurance owners and product/research forward plan; finished or superseded plans are removed once their obligations move |
| `docs/adr/` | Current decision records (accepted and open proposals), a generated index, and `TEMPLATE.md` |
| `docs/design_review/design_principles/` | The layered design standard, declared in `standard.toml`: seven foundations (FP-01–07), including execution fit for the intended workload; independent architectural judgments A1–A4, supporting rules DP-01–24 and gates G1–G8, the CI profile, and the repository binding (ADR-0040/0130) |
| [Heuristics for Efficient Architecture](design_review/design_principles/core/efficient-architecture-heuristics.md) | Companion to the design principles: use relevant qualitative patterns during design, planning and consequential implementation choices, before physical mechanisms become entrenched |
| `docs/design_review/reviews/` | Review outputs: evidence, never authority; kept while a finding they supply is open |
| `docs/design_review/evidence/` | Optional probes, spikes and investigations behind decisions. When created, use one `YYYY-MM-DD_<topic>/` folder with a README; raw outputs and binaries through Git LFS; never venvs or `target/`. This location convention does not require a review to create or run probes |
| `docs/pins.md` | Holds: pins that should not be bumped casually, with reason and revisit trigger |
| `crates/` | Single Rust workspace: `lctx-model` owns semantic/wire declarations, policies and shared invariants; `cpg-extract` acquires/captures and produces pinned native facts; `cpg-flow` supplies ty flow; `cpg-core` owns native completed-view compilation and admitted artifacts; `lctx-analytics` supplies pure native kernels; `lctx-embed` supplies the embedding effect; `lctx-surrealdb` realizes native compiler state/graphs/cache/projections, `lctx-publisher` seals the admitted compiler database and independently imports/restores verified content, `lctx-serving` implements fixed-snapshot operations; `lctx` is the CLI. Current serving shapes, wire/native contracts and ranking derive from `lctx-model`; the old schema/bundle authorities are retired |
| `python/` | `lctx_mcp` FastMCP transport and numerical library adapter; `lctx_semantics` exposes model-derived pure wire helpers. NativeSession and native persistence/serving are implemented; operator activation is pending |
| `eval/` | `behavior/` pre-registered question sets, `gold/` evaluation-only gold extract and freeze, `heldout/` sealed until increment 5 |
| `libraries/` | One committed uv project per analyzed library (`pyproject.toml` with `[tool.lctx] release`, `.python-version`, `uv.lock`); `libraries/README.md` has the add/upgrade procedure (ADR-0117). Environments go to `build/envs/` (gitignored) |
| `fixtures/python/` | Tiny Python packages to analyze. Input data: never executed or linted |
| `third_party/` | `pyrefly-<ver>.patch`: the one commit our Pyrefly fork adds to the upstream tag (ADR-0117, `docs/pins.md`) |
| `scripts/` | `adr.py`, `design_sections.py` and `docs.py` (documentation), `check_family.py`, `check_agents.py`, and gold/eval scripts |
| `rules/`, `rule-tests/` | ast-grep rules. They grow only from design-review findings |

## Authority and search

The architectural collection owns accepted contracts and labeled targets. Current ADRs own
rationale and open choices; the forward plan owns product sequencing and the current disposition
of scheduled product findings. The persisted execution plan owns the current compiler/consumer pivot and its findings; the
graph-native coordinator retains its original receipt boundaries. Architectural owners and the
runbook describe available operator routes.
Retained reviews hold dated evidence for open findings; STATUS links
the current checkpoint. Executable declarations own detailed implementation contracts.

The site derives navigation and search from the retained working set. **Current** is the default:
architecture, entry pages, the active standard and process, accepted decisions and selected work.
**Reference** contains proposals, supporting reviews and evidence, pins and guidance.
**Everything** searches both. A search result's scope is a reading aid, not a truth verdict;
evidence labels still apply. Local capability indexes are not bundled into the website.

## Historical recovery

Retired plans, reviews, evidence, research input and ADRs are not part of the working set
(ADR-0042); current owners carry every surviving obligation, so recovery is never a reading
prerequisite. To recover a deleted path:

- Find the commit that deleted it: `git log --diff-filter=D --name-only --format=%h -- '<path or glob>'`
  (for an ADR id, `'docs/adr/NNNN-*'`).
- Show its last content: `git show <commit>^:<path>`; list a deleted directory with
  `git ls-tree -r --name-only <commit>^ -- <dir>`.
- The documentation migration removed its retired set in `df1ebd8`; its parent
  `f54b09d090e66abcee1fcc90d2005524defe4582` holds all of it (`git show f54b09d:<path>`).
- Raw evidence is in Git LFS: `git lfs fetch origin <commit>^ --include=<path>`, then
  `git show <commit>^:<path> | git lfs smudge`.

The 2026-10-04 retirement of superseded implementation worktrees has local recovery assets at
`/home/paul/.local/share/library-context/worktree-recovery/2026-10-04/`. Its README, disposition,
verified Git bundle, patches and dirty/untracked archives preserve their content without retaining
active checkouts. Repository refs `refs/archive/worktree-closeout/2026-10-04/*` also anchor their
committed history. The archived Pyrefly prototype's unique raw analysis receipts are stored there;
these historical assets are not runtime inputs or current qualification evidence.
