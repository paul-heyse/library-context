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
| `cargo test --release --no-fail-fast -p cpg-core -p lctx-model -p lctx --test catalog_selection --test synthesis_refutation --test compile_facts --test domain_admission -- --nocapture`, [functional5](raw/phase4-resume-functional5.log) | **failed**, one target. Current C1 actual PG mutation controls2 **passed**, CLI facts/upper seedless frontiers/nonzero briefs and cold/warm/cleared fake embedding cache controls3 **passed** across both profiles (505.48s), and admission controls11 **passed**. Negative S0 native compilation/publication completed, then its reader failed on an invalid Serving fixture connection configuration; that fixture is corrected and the proof-backed consumer assertions require rerun. The named six-compilation CLI test receives a 900s Nextest timeout based on this observed runtime; this is not a performance claim. |
| `cargo test --release --no-fail-fast -p cpg-core --test synthesis_refutation --test catalog_selection -- --nocapture`, [functional6](raw/phase4-resume-functional6.log) | **failed**, negative S0 target; strengthened actual C1 PG controls2 **passed** (36.41s). Native publication/reads completed but the recursive fixture produced no RefutedUnderModel conclusion. |
| `cargo test --release -p cpg-core --test synthesis_refutation -- --nocapture`, [diagnostic1](raw/phase4-resume-refutation-diagnostic1.log) | **failed**, confirms complete native Flow membership, two finite Conditional proofs and explicit ConditionTransferUnsupported Unknown residuals. The recursive input cannot correlate distinct native guard evaluations or establish the proposed finite negative. It is replaced by an explicit runtime-false source candidate; current native/store rerun pending. |
| `cargo test --release --no-fail-fast -p cpg-flow -p cpg-core --test flow_shapes --test synthesis_refutation -- --nocapture`, [native1](raw/phase4-resume-refutation-native1.log) | **failed**, negative target still had no finite conclusion: retaining the false value alone did not retain its native reaching origin. The previously compiled native suite31 passed; its binary did not contain the later origin probe. |
| `cargo test --release -p cpg-flow --test flow_shapes -- --nocapture`, [false-origin1](raw/phase4-resume-false-origin1.log) | **failed**, actual false value candidate exists but its native parameter-reaching set is empty. No formal origin is manufactured. |
| Same combined native/store command, [native2](raw/phase4-resume-refutation-native2.log), then `cargo test --release -p cpg-flow --test flow_shapes -- --nocapture`, [false-origin2](raw/phase4-resume-false-origin2.log) | **composite passed focused scope**: actual PG negative S0 control1 **passed** (86.83s), including exact Summary Refutation, coverage membership, kind18 finding, Limits assertion and selected brief, plus missing-proof/erased-membership/Partial-coverage refusal twins. Native2's enclosing command **failed** an old discarded-count expectation because retained candidates are no longer discarded. After correcting that expectation, current native controls33 **passed**, including a false native formal origin, Defined/Unbound competitors retained together, inactive definitions excluded from live reads and computed-false refusal. |
| Earlier corresponding focused combined command, [functional2](raw/phase4-resume-functional2.log) | **failed**, five targets. Preserved provenance for corrected native receiver placement, constructor body identity, Facts-prefix replay, C1 option multiplicity and unmaterialized MDX refusal controls. Individual passing targets do not establish an enclosing pass. |
| [X0 expectation migration map](raw/x0-expectation-map.md), [exact candidate inventory](raw/x0-exact-deletions.txt) | Read-only mapped retirement preparation; deletion **not_run**. The source review and current plan own preservation/acceptance obligations. |

Full `just test-all` and `just hygiene` are **not_run** until all functional scope, retirement and assembled review are complete.

Independent focused design advice (static, 2026-10-01) accepts checked-false direct bare-name
identity sources backed by actual native uses, and complete native reaching retention only for
nonempty, wholly exact-false sets without loop headers. Every Defined/Undefined/Deleted state
survives; live/approximate sets keep prior pruning. Entry exact-one/formal-origin admission,
complete coverage and proof validation remain unchanged. Existing semantic types suffice;
[ADR-0111](../../../adr/0111-checked-false-native-source-candidates.md) records this bounded native
policy and rejects preferred-formal selection, live inactive-row inclusion and no-read relabeling.
The provider build digest includes the changed mapping source. This advice is not the assembled
exit review.
Live embedding, real-library upper-frontier pilots, product comparisons, performance/hydration measurements,
total-RSS and Phase 5 serving are **not_run**, outside this authorized Phase 4 qualification.

Receipt timing boundary: the new C1 source-link deletion, runtime-Known forgery and sibling-parameter
retarget controls were committed after functional2's catalog-selection binary had compiled.
Functional2's pass does not qualify those probes; functional5 supplies their actual PG pass.
