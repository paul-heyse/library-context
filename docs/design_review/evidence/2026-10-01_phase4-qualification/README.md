# Phase 4 qualification — 2026-10-01

**Qualification in progress.** The [Phase 4 plan](../../../plans/semantic-model-phase4-detailed-plan_2026-09-30.md)
owns the execution scope and acceptance. The [parent cutover plan §8](../../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition)
owns cross-phase finding disposition. This receipt bounds implementation claims; it creates no new gate.
Raw logs use Git LFS. Earlier restart receipts retain their original scope and date.

Commands use `python3 scripts/build_environment.py --` before Cargo.

| Command and receipt | Outcome and boundary |
|---|---|
| `cargo check -p lctx -p cpg-core --tests`, [check3](raw/phase4-resume-check3.log) | **passed**, 21.23s, after native qualification controls, derived coverage metadata and S0 profile-input repairs. |
| `cargo test --release --no-fail-fast -p cpg-core -p cpg-extract -p lctx-model -p lctx --test symbolic_composition --test catalog_evidence --test summary_publication --test synthesis_documentary --test synthesis_refutation --test symbolic_summary_synthesis --test analysis_schedule --test upper_frontier_closures --test domain_callable_aspects --test compile_facts -- --nocapture`, [functional3](raw/phase4-resume-functional3.log) | **failed**, four targets. Actual C1 PG both profiles, documentary PG, metadata, symbolic S0 and static closures passed. Summary replay/corruption controls ran, but Behavioral's final assertion wrongly required native argument and constructor qualifications to differ. Native composition had the same erroneous assumption. Upper CLI and negative S0 stopped at a derived-vs-native Flow admission defect. Corrections require current reruns. |
| `cargo test --release --no-fail-fast -p cpg-core -p cpg-extract -p lctx-model -p lctx --test symbolic_composition --test summary_publication --test synthesis_refutation --test compile_facts --test domain_admission -- --nocapture`, [functional4](raw/phase4-resume-functional4.log) | **failed**, three targets. Native symbolic composition2 and both actual Summary PG cases2 **passed** (intact/permuted and controls0–11). Upper CLI and negative S0 preflight then exposed checkpoint filtering of non-Facts vocabulary stages, excluding in-scope Analysis writers. Seven existing admission controls failed before their intended checks because the synthetic Pyrefly fixture omitted new signature-enumeration relations. Both corrections are integrated; runtime rerun pending. |
| `cargo check -p lctx -p cpg-core --tests`, [check4](raw/phase4-resume-check4.log) | **passed**, 13.42s, after checkpoint scope and signature-enumeration fixture corrections. |
| Earlier corresponding focused combined command, [functional2](raw/phase4-resume-functional2.log) | **failed**, five targets. Preserved provenance for corrected native receiver placement, constructor body identity, Facts-prefix replay, C1 option multiplicity and unmaterialized MDX refusal controls. Individual passing targets do not establish an enclosing pass. |
| [X0 expectation migration map](raw/x0-expectation-map.md), [exact candidate inventory](raw/x0-exact-deletions.txt) | Read-only mapped retirement preparation; deletion **not_run**. The source review and current plan own preservation/acceptance obligations. |

Full `just test-all` and `just hygiene` are **not_run** until all functional scope, retirement and assembled review are complete.
Live embedding, real-library upper-frontier pilots, product comparisons, performance/hydration measurements,
total-RSS and Phase 5 serving are **not_run**, outside this authorized Phase 4 qualification.

Receipt timing boundary: the new C1 source-link deletion, runtime-Known forgery and sibling-parameter
retarget controls were committed after functional2's catalog-selection binary had compiled.
Functional2's pass does not qualify those probes; their current actual PG rerun is pending.
