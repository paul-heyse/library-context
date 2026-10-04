# Target-alignment handoff: foundation-first execution

**Operator-directed handoff, 2026-10-04.** Consumer: the next agent planning/executing the target-alignment series. This is restart context and reading guidance, not another execution ledger. The [coordinator](target-implementation-alignment-plan_2026-10-04.md) owns current disposition, dependencies and combined acceptance; [STATUS](../../STATUS.md) owns the live checkpoint. Refresh both and the actual tree before starting.

## 1. Objective and current boundary

The operator wants the most foundational work prioritized and existing code fully migrated to integrate the design improvements: semantic contracts, library capabilities/new libraries where justified, architectural/mechanical consolidation, foundational tests, consumer integration and obsolete-path deletion. Do not interpret this as a handful of urgent bug fixes or documentation closure alone. A large aggregate incremental scope is acceptable; a complete overhaul is not selected.

Prioritize foundations that make subsequent analysis/serving extensions reliable. Address every applicable opportunity in the coordinator, including bounded investigations. A supported library improvement can become implementation scope after its decision; a retain/defer outcome must identify evidence and reopening trigger. Library availability alone does not justify adoption, and license is not a rejection reason.

This turn writes handoff documentation only. Production corrections remain Proposed. For subsequent authorized execution, use the [execute-plan skill](../../.claude/skills/execute-plan/SKILL.md); use [plan-execution](../../.claude/skills/plan-execution/SKILL.md) to resolve consequential scheduling gaps without repeating plan creation or the comprehensive design review. Local implementation choices remain with the implementer.

**Separately stopped:** Phase 5 real-library reconstruction/activation, selection/registrations, live vectors, PR6 and sealed confirmation evaluation. Fixture/native/model/actual disposable-PG controls for foundational implementation are distinct from that stopped operator campaign. Do not reset/select the stopped staging merely to get a fixture pass.

## 2. Read first, then follow the relevant owner

| Reference | Why the next agent needs it |
|---|---|
| [AGENTS](../../AGENTS.md), [STATUS](../../STATUS.md) | Current commands, permissions, preservation, hook/build rules, runtime state and verification boundaries |
| [Coordinator](target-implementation-alignment-plan_2026-10-04.md), especially §§3–7 | Shared decisions, dependencies, migration/Q0 and **single current disposition table** for transferred review obligations |
| [Model contracts/execution plan](model-contract-execution-alignment-plan_2026-10-04.md) | F1–F6 and IF1–IF4; main architectural/mechanical foundations and conditional compute adoption |
| [Analyzer fidelity plan](analyzer-fidelity-alignment-plan_2026-10-04.md) | A1–A4 and IA1–IA4; input fact correctness, observation consumers, provider policy/coverage and parser mechanics |
| [Serving provenance/presentation plan](serving-provenance-presentation-plan_2026-10-04.md) | S1–S4 and IS1–IS4; input-domain admission/resource identity, render contracts, effects and product readiness |
| [Target review](../design_review/reviews/design_review_target-implementation-alignment_2026-10-04.md) | Corrected diagnosis and preservation constraints. New F01–F03 are distinct from similarly named older findings |
| [Library-leverage review](../design_review/reviews/design_review_library-leverage_2026-10-04.md) and [evidence index](../design_review/evidence/2026-10-04_library-leverage/README.md) | Original inventory and alternatives. Follow-up review/selected plans supersede mistaken remedies, not original evidence |

Do not load every historical document before a ready package. Read the responsible supporting plan, owner and decisive adjacent consumer; follow additional references for a concrete uncertainty.

Architectural routes:

- [Architecture map](../design/README.md), [DESIGN §B](../design/DESIGN.md), [semantic model §15](../design/sections/semantic-model.md): ownership, declarations, closure, identity, pure kernels and publication.
- [Acquisition/extraction §4](../design/sections/acquisition-and-extraction.md): independent analyzer roles, hermetic inputs, exact source correspondence and fork maintenance.
- [Storage/publication §5–§6](../design/sections/storage-and-publication.md), [PostgreSQL runbook](../postgresql.md): physical lowering, generations, migrations, leases and effects.
- [API/evidence product §14](../design/sections/api-and-evidence-product.md), [synthesis/serving §10–§11](../design/sections/synthesis-and-serving.md): consumer meanings, evidence closure, retrieval/rendering and wire resources.
- [Analytics §9](../design/sections/analytics.md), [validation/evaluation §8/§12](../design/sections/validation-and-evaluation.md): projection/kernels, independent controls and evaluation isolation.
- [Standard manifest](../design_review/design_principles/standard.toml) and [repository binding](../design_review/design_principles/binding/library-context.md): core/template 3.2, CI 1.3, evidence labels, review cadence and disposition. No new principle-by-principle review for ordinary implementation.

Read each owner's Decision line for rationale and rejected alternatives. The most relevant accepted records are:

- [ADR-0085 typed semantic domain](../adr/0085-typed-semantic-domain.md) and [ADR-0086 generations](../adr/0086-immutable-postgresql-generations.md): model/store authority and declaration-based lowering.
- [ADR-0105 vocabulary epochs](../adr/0105-analysis-vocabulary-epochs.md) and [ADR-0108 analysis owners](../adr/0108-immutable-analysis-owners.md): exact immutable read/publication boundaries.
- [ADR-0117 analyzer families](../adr/0117-independent-analyzer-families.md), [ADR-0118 pins](../adr/0118-pinned-compute-and-analyzer-families.md), [ADR-0121 evidence packets](../adr/0121-analytical-evidence-packets.md): independent parsing/provider roles and current enrichment seams.
- [ADR-0073 wire contracts](../adr/0073-catalog-wire-contracts.md), [ADR-0114 serving](../adr/0114-generation-serving-contracts.md), [ADR-0116 preparation](../adr/0116-bounded-semantic-preparation-and-consumption.md): Rust wire authority, generation pinning and bounded actual-work lifetimes.
- [ADR-0071 product](../adr/0071-api-evidence-product.md) and [ADR-0078 cutover](../adr/0078-current-design-cutover.md): evidence discovery target, deliberate replacement and qualified current state.

## 3. Foundation-first implementation route

This is priority guidance within the coordinator's dependency graph, not whole-document synchronization barriers. Dependencies and shared edit ownership determine actual concurrency.

| Priority group | Deliver complete producer/consumer scope |
|---|---|
| Fact fidelity, A1 | Preserve native Bound/unattached candidate formulas; remove fabricated Unbound reaching evidence and validator exception. Integrate native producer, adapter, model replay, raw-origin PG/wire and independent controls together |
| Declaration/composition foundations, F1/F2 | One finite common publication expansion; owned grant construction and typed Local refusal. Migrate actual stage/provider/core consumers; preserve exact epochs, profile exclusions and validator composition |
| Representation foundations, F3/F4 | One PG physical-column convention; explicit declaration/mapping identity encodings with conservative source invalidation. Migrate every lowering/reader/cache consumer and review representation/identity consequences |
| Finite mechanics/effects, F5/F6/A4/S4 | Typed Rows lookup, named charged indexes, existing Diagram support; execution cap diagnosis and migration lock lifetime; qualified cursor/string/token/package/identifier helpers and PG options/retry/consumer-based deletions |
| Fact enrichment foundations, A2/A3/IA4 | Establish useful supplementary Ruff observation consumers or retire unused seam; attributed policy decisions/custom-builtins/local coverage; PEP 695 attachment coverage. Do not lower typing branches into runtime proof |
| Admission/provenance foundations, S1/S2 | A pure admitted-library domain reused across catalog/retrieval and a generation-bearing resource renderer. These are semantic foundations even though their consumers are serving routes |
| Presentation/corpus foundation, S3 | Owned display/source/expression modes and finite code labels; migrate synthesis/retrieval and version/rebuild derived text/vector contracts before future quality work |
| Conditional foundation mechanisms, IF/IA/IS | Resolve consumer and contract fit early where it changes a foundational package; integrate enabled changes fully before Q0. Product-only comparisons retain frozen-task/activation triggers |

A1 has the highest immediate fidelity consequence and need not wait for F1–F4. F1/F2 can proceed independently after shared declaration interfaces settle. F3/F4 overlap model/artifact compatibility work; coordinate versions and final refresh. Small helpers need only their actual consumer prerequisites. S1/S2 can run alongside foundational consolidation but share wire/native/model surfaces. Do not defer required consumer migration to an unspecified later cleanup.

For each delivered contract, identify all live producers/readers/lowerings/explainers, migrate them, remove independent old mechanics, and exercise a revealing negative case through the new owner. Deriving two outputs from one mistaken declaration is not independent acceptance. A local package can finish while its broader finding still has other contributors.

## 4. Consequential details already settled

- **A1 is a schema/wire change, not a lossless row deletion.** Candidate evidence must retain pre-pruning reachability/narrowing qualifications and availability/precision. Bound+unattached has zero mapped members and a SubjectBoundary; no fabricated reaching/narrowing target. LoopHeader may have partially mapped expansions, so do not generalize unattached⇒zero to every kind. Reaching completeness stays separate from narrowing precision. Retained formula alone never authorizes Entry proof.
- **F1 starts with synchronous finite publication declarations.** The common eleven-record family does not require an async heterogeneous visitor. Broader port/load/write machinery needs a current repeated consumer and Catalog/upstream composition controls; keep actual awaited work explicit where simpler.
- **F2 preserves different concepts.** Decoder reachability, actual consumed rows, exact `(relation, epoch, stream order)` validation requirements and sufficient grants are separate. Summary Facts/Model epochs already survive. Different nonempty validator lists remain incompatible; no unconditional union. Use sorted relation lookup rather than a new maintained map.
- **F3 does not turn hidden physical columns into semantic fields.** Generation/epoch/tag conventions remain PG-owned; codec meaning remains model-schema-bound. Keep independent live catalog/view/FK/ACL checks.
- **F4 does not promise refactor-stable model IDs.** Source/path/manifest/lockfile fingerprinting stays conservative. KeySink v3 already has an independent byte framing test. Embedding Spec already has specified ordered compact JSON and a persisted cache consumer; JCS is not automatically better or required.
- **S1 resolves captures before filtering.** Name denotes matching FirstParty release/input plus explicit corpus associations, never all installed dependencies or arbitrary latest release. Unknown domain gets a dedicated typed failure, not missing-generation `Error::Absent`. Admitted empty can succeed; CompleteDomain describes finite canonical enumeration, with analyzer coverage separate.
- **S2 preserves authored bytes.** Render from GetCapabilityResponse with compact GenerationKey/capability metadata and existing assertions. URI remains process-relative. Bump resource-presentation identity and complete-envelope budgets; S2 alone does not require canonical corpus rebuilding.
- **S3 preserves exact values and uncertainty.** Display/source/executable expression differ. Keep signed zero, special floats/NaN bits, bytes, factory and unknown defaults. Renderer changes version the corpus and invalidate affected derived artifacts.
- **Compute remains selective.** Pure charged kernels and model-owned in-memory relational lowering are eligible. DataFusion membership joins exist; pure operations do not inherently prohibit a library engine. Scoped CI-07 rationale is recorded at §15; it is no exemption for generic duplication.

## 5. Library opportunities must receive explicit decisions

The coordinator §7 includes the full original M1–M4/N2 inventory and all named deferrals. Use that coverage rather than creating another library register. Assess technically compelling opportunities across the foundation scope; do not automatically restrict attention to currently used API calls.

| Evidence/skill route | Questions to resolve |
|---|---|
| [Compute/storage evidence](../design_review/evidence/2026-10-04_library-leverage/l1-compute-storage.md); DataFusion and SQLx/PostgreSQL skills | Model-owned bulk lowering, typed dynamic SQL when needed, migration connection/lock lifetime and useful diagnostics |
| [Analyzer evidence](../design_review/evidence/2026-10-04_library-leverage/l2-analyzers.md); python-analyzers skill | Export/Branch observations, policy comparisons, custom builtins, source helpers and conditional Glean/Griffe/docstring/SCIP uses |
| [Reasoning/graph evidence](../design_review/evidence/2026-10-04_library-leverage/l3-reasoning-graphs.md); rust-reasoning/rust-graphs/fixedbitset skills | Fused BDD exists policy, evidence-preserving SCC, weighted ranking, FCA/RCA and Ascent BYODS/custom relations under the actual bounded contract |
| Serving plan IS packages; FastMCP/PyO3 skills | Final bytes, startup admission/cancel/drain, task tracking and embedding twin boundaries |
| [Pins](../pins.md), [skill selection](../../.config/library-skills.toml), [pin-check](../../.claude/skills/pin-check/SKILL.md) | Exact sources/features/family boundaries, useful pydantic/vLLM skill selection, custom engine build compatibility and migrations after an upgrade |

Use `.claude/skills/<name>/SKILL.md` as the live pinned capability route; check pins before transferring a claim. For an uncovered library consult Context7 per AGENTS, and source-check the required pinned Rust capability when Python docs are inadequate. Source/code review is not itself a Context7 requirement.

Historical P1–P6 evidence used an isolated nightly-2026-09-28 harness; it does not qualify the production toolchain/tree. Static inspection may settle a fit question; focused probes are discretionary for material uncertainty. Measurements are needed for speed/quality/RSS claims.

Require the actual replacement contract: same universe/lineage, order/multiplicity/nulls, deterministic work/stop, typed pending/refused outcomes and charge-before-retention/mutation. BDD value parity does not imply refusal-policy parity; condensation(false) preserves arcs but creates internal self-loops; weighted PageRank must handle distinct neighbors; odis is not wholly batch-only; the earlier Ascent probe did not establish universal impossibility. State the remaining integration cost and retire the replaced path after adoption.

Ruff Export/Branch seams without a useful consumer should be removed instead of indefinitely maintained. Dev oracles remain dev-only unless a distinct justified production decision changes that boundary. Griffe cannot independently corroborate gold-scored items sharing its extractor. Sealed data and gold references never become compiler inputs or tuning data.

Shared capability skills are repo-agnostic under current AGENTS. T0 moves stale project-specific guidance into existing repository owners and removes the shared project overlay through the skill maintenance route. Accepted ADRs are not substantively edited; use [adr](../../.claude/skills/adr/SKILL.md) and update the owner if a meaningful decision changes.

## 6. Remaining acceptance and historical documents

This series carries the remaining assembled acceptance into [coordinator Q0](target-implementation-alignment-plan_2026-10-04.md#5-migration-and-verification). Prior plans retain original finding IDs/receipts; do not run parallel competing full campaigns or erase failures.

| Reference | Remaining or preserved context |
|---|---|
| [Enrichment coordinator §7](code-facts-analytical-enrichment-plan_2026-10-04.md#7-current-disposition-and-checkpoint) | Integrated A0/A1/A2, C1–C7, G1, R1/R2, K1/K2; failed broad attempt and bounded repair receipts; pending Named migration/current artifacts and Q1 |
| [API/evidence enrichment](api-contract-and-evidence-enrichment-plan_2026-10-04.md), [guarded origins](guarded-origin-analysis-plan_2026-10-04.md), [references](python-reference-enrichment-plan_2026-10-04.md), [kernel assurance](analytic-kernel-assurance-plan_2026-10-04.md) | Existing contract/control designs for affected consumers. G2/G3 remain deferred until useful complete multi-candidate exclusion is demonstrated; ty navigation/odis stay dev-only |
| [Expansion coordinator §7](code-facts-expansion-plan_2026-10-03.md#7-current-checkpoint-and-next-action) | Integrated migration/facts/analysis/product work; surviving conditional/generated, Terminal and raised-type actual PG journeys |
| [Migration](python-analyzer-migration-plan_2026-10-03.md), [normalization](code-facts-normalization-plan_2026-10-03.md), [behavioral](code-facts-behavioral-analysis-plan_2026-10-03.md), [analytics/product](code-facts-analytics-product-plan_2026-10-03.md) | Original pipeline contracts and applicable targeted controls. Migration's old 1.3.1 baseline is historical, not current linked supply |
| [Earlier incremental alignment](semantic-model-incremental-alignment-plan_2026-10-02.md), [cutover](semantic-model-cutover-plan_2026-09-29.md) | Earlier scoped closure and ownership/deletion obligations; not qualification of the expanded current fact pipeline |
| [Phase 5 §10](semantic-model-phase5-detailed-plan_2026-10-01.md#10-finding-routes-limits-and-current-state), [forward plan](behavioral-model-forward-plan_2026-09-24.md) | Separately stopped operator activation, product/research triggers and PR6 readiness. Avoid reviving obsolete automatic Stage 4 adoption rules |

**Current pending schema:** four nullable fields across two existing Named enum relations, plus any new selected alignment changes. Review `.snap.new` before `cargo insta accept`; never auto-update snapshots or renumber codebooks. New condition/identity/wire fields can add migration work; do not accept a predecessor schema as current.

During implementation use release-profile touched-crate checks and targeted meaningful controls, including actual disposable PostgreSQL 18 through the generation store. Tests must expose semantic countercases and damaged inputs, not only repeat generated expectations. Complete required foundational tests and migrations before broader output validation.

After all selected functional packages/enabled investigations integrate, run `just test-all` and `just hygiene` on the same tree. Hygiene includes current all-target Clippy and policy/type/docs/store checks; the automatic formatter does not run or fix them. Repair and rerun the failed `just <id>`; record composite receipts honestly. Prior passing counts are not a current full pass.

Finish hook-owned source formatting before final fingerprint-qualified schema/CLI/native adapters and fixture/store journeys. Model identity hashes raw Rust. End the editing turn so the hook can run, or use an already scoped operator exception if actually applicable; do not assume earlier one-time exceptions authorize another manual formatter/generator. Subsequent source repairs reopen matching artifacts and affected verification. No compatibility reader, rollback store or second authority is selected; rebuild from pinned inputs and protect/quiesce readers where replacement is required.

## 7. Workspace, runtime and evidence checkpoint

Baseline observed for this handoff: `eeeff748` on main, which created four plans and updated current owners. Production code was not changed by that authoring commit. `docs/library-utilization.jsonl` is a concurrent advisory refresh; preserve/exclude it from scoped changes. Only main remains in `git worktree list`; previous agents concluded. Recheck status before editing.

Plan publication: `UV_NO_SYNC=1 just docs-check` **passed**, 2026-10-04, 297 canonical pages and zero offline errors. The handoff's own publication receipt is recorded in STATUS. Scoped fresh plan advice found no must-fix issue; it is not formal whole-system or runtime acceptance. This handoff runs no production builds/tests/qualification.

Recent historical production controls: thirteen targeted model/Structural/actual-PG controls passed before source formatting; predecessor full Clippy passed only for its recorded earlier source. Current schema/artifacts/full gates remain pending. The initial enrichment full attempt failed (997 passed, 95 failed, four timeouts, three skipped); focused repairs did not convert it to a clean full gate. See original receipts rather than reconstructing success from counts.

Stopped staging `d3a3fa026a1237a1c4e175b9b1019eeb` was last recorded unselected with 204 receipts. Refresh state read-only if needed. Default store/client registrations were untouched. The stop receipt is `/home/paul/.cache/lctx-phase5-qualification/2026-10-03/operator-pivot-interruption.json`. Worktree recovery content is under `/home/paul/.local/share/library-context/worktree-recovery/2026-10-04/`; earlier alignment receipts are under `/home/paul/.cache/lctx-alignment-execution/receipts-2026-10-03/`. These are local recovery/evidence, not alternate runtime authorities.

Rust uses pinned `nightly-2026-09-29`, Cargo jobs16, default frontend1, sccache/Clang/mold and stable shared intermediates. No routine cargo clean or blanket build serialization; coordinate actual same-tree contention. `NEXTEST_TEST_THREADS=8` limits concurrent test processes, not Rust compiler frontend threads or all threads created inside a test process. Do not change settings based solely on earlier disk cleanup; the operator freed substantial space and preferred the existing build approach.

For bare Cargo/uv normalize the build environment with `eval "$(python3 scripts/build_environment.py --shell)"`; use `just`'s normalization where available. Python is 3.14.7 via uv, type checker pyrefly. Never pass floating +nightly/+stable. Preserve O2 workspace/O3 imported/O2 build-script/proc-macro profiles and benchmark artifacts.

AGENTS grants local PG18 superuser administration through lctx_superuser; never expose credentials or use them in product configuration. Runtime roles remain non-superuser. Prefer disposable owned test resources. The earlier external probe that deleted other containers was stopped; do not invent a new blocker, but qualify resource ownership if interference reappears.

## 8. Coordination and completion

Use [shared roles](../../.agents/roles/README.md) for bounded implementation, library investigation, independent advice/review and testing when parallelism or context isolation is worthwhile. Root owns design/integration/acceptance; assign one owner for shared declarations, manifests/pins, physical conventions, schema/wire/version updates and generated surfaces. Separate worktrees only for truly parallel production edits; preserve siblings' changes and integrate consumers before accepting completion.

Keep source IDs TA-Fnn/LL-Fnn/LL-On intelligible and update coordinator §7 after evidence. Supporting plans and this handoff never copy mutable progress. Correct enduring ownership/meaning in existing architectural sections and ADRs; retire documents only after surviving obligations and last consumers move.

A foundation is delivered when its producer and every applicable current consumer use the target contract, obsolete independent mechanics are gone, required failure/unknown/resource distinctions survive, and meaningful focused controls pass. Series completion additionally requires Q0. Optional untriggered benefit measurement or separately stopped product activation is a recorded limit, not a reason to claim either failure or completion incorrectly.

First next action: refresh the live tree and coordinator, establish shared edit ownership and the ready foundational slice, then carry the authorized implementation through migration and targeted acceptance. A1 and F1/F2 are ready starting points; inspect F3/F4 migration dependencies early, and evaluate library alternatives that can change those decisions before investing in dependent code. Continue the full foundation-first scope rather than stopping after the first corrected defect.
