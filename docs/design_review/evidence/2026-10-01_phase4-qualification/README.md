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
| [X0 expectation migration map](raw/x0-expectation-map.md), [original candidate inventory](raw/x0-exact-deletions.txt), [dormant expectation partition](raw/x0-dormant-expectation-map.md), [applied paths](raw/x0-applied-paths.json) | Applied mapped retirement: 92 deleted source/snapshot paths, 11 bounded consumer/manifest modifications, one recovered P5 native-serving owner. Independent typed/native expectations qualified before removal; named P5 sources, services, fixtures and evaluation assets survive. Cargo.lock reflects manifest changes. |
| `cargo check -p cpg-core -p lctx-analytics -p lctx --tests`, [X0 check1](raw/phase4-x0-final-check1.log), [X0 check2](raw/phase4-x0-final-check2.log) | **composite passed**. Initial check **failed** because dependency trimming omitted leiden-rs needed by the retained independent native ranking oracle. Restored it as a dev dependency; complete rerun **passed**, 41.30s. No oracle was removed. |

All functional scope and retirement are integrated. The [assembled Design/Target review](../../reviews/design_review_phase4-assembled_2026-10-01.md)
is **Accept scoped**, 2026-10-01, at72ba0413. Full Q0 remains incomplete.

| Assembled command | Outcome and correction boundary |
|---|---|
| `NEXTEST_TEST_THREADS=8 just test-all`, [full1](raw/phase4-q0-test-all1.log) | **failed**. Release build passed (29m38s); Nextest discovered993 tests/199 binaries. Genuine older fixture failures were captured before the operator's confirmed concurrent `cargo clean` removed compiled binaries and produced many `double-spawn` errors. No passing full functional gate follows; pytest, separate PG gate and doctests were not_run. Facts-only memory/read fixtures, signature-enumeration writer inventory, two new normalized stage counts, shared prose FK targets, complete derived-read checker input and native exact Entry selection are corrected for rerun. |
| `just hygiene`, [hygiene1](raw/phase4-q0-hygiene1.log) | **failed** at ADR lint after lint-agents passed. Actual governing-decision citations/section references and stale index require correction; no check is waived. |
| Individual unfinished hygiene ids `just fixtures-check`, `just gold`, `just rules-scan`, `just rules-test`, `just ruff`, `just types` | **passed**, 2026-10-01. `just docs-check` failed the same ADR lint defects; it requires rerun. |
| `just deps`, [deps1](raw/phase4-q0-deps1.log) | **failed** after family, cargo-deny and Pyrefly fork checks passed: core's test-only analytics dependency was misplaced and its direct testcontainers dev dependency unused. Corrected ownership/removed unused dependency; named unlinked P5 recovery sources remain preserved. Remaining Hakari checks await the repaired rerun. |

The ADR owner/reference repair passed49 focused documentation tests in its isolated checkout
(`uv run --project /home/paul/library-context --no-sync pytest -q tests/scripts/test_adr.py tests/scripts/test_design_sections.py tests/scripts/test_docs.py`),
then integrated as eb6f98e8. Accepted ADR bytes and all lint obligations remain unchanged; current
ADR lint now **passed** after the operator explicitly authorized `just adr index`; the generated index includes current governing records. An initial worker `uv run pytest`
unintentionally began a project sync/maturin build; the worker stopped its owned processes
immediately, with no clean or cache/path changes. The passing rerun used the existing root environment.

| Q0 repair command | Current outcome |
|---|---|
| `just adr index`, `just build-features`, [index](raw/phase4-q0-adr-index1.log), [features](raw/phase4-q0-build-features1.log) | **passed**, explicit operator-authorized exception to hook-only generator timing. Removed three obsolete Hakari entries; no formatter was run. |
| `just adr-lint`, [ADR rerun](raw/phase4-q0-adr-lint3.log) | **passed**, all 61 records. |
| `just deps`, [deps2](raw/phase4-q0-deps2.log), [deps3](raw/phase4-q0-deps3.log) | **composite passed**. Deps2 refused stale Hakari entries; after the approved refresh, full deps3 passed including both Hakari checks. Named unlinked P5 sources remain preserved. |
| `just docs-check`, [docs2](raw/phase4-q0-docs-check2.log), [docs3](raw/phase4-q0-docs-check3.log) | **composite passed**, 254 canonical pages and zero link errors. Nine retained reviews now identify 21 retired source citations as historical paths within their original inspection scopes; no review judgment, evidence strength or publisher check was changed. |
| `just clippy`, [clippy1](raw/phase4-q0-clippy1.log), [clippy2](raw/phase4-q0-clippy2.log), [clippy3](raw/phase4-q0-clippy3.log), [clippy4](raw/phase4-q0-clippy4.log) | **composite passed**, complete [clippy7](raw/phase4-q0-clippy7.log) rerun after model/core/extraction lint repairs. Earlier compiler-exposed macro syntax, private-name collision, test-only imports and required non-Copy clone were corrected. Integration-test warnings and signature-enumeration type paths were repaired in [clippy5](raw/phase4-q0-clippy5.log)/[clippy6](raw/phase4-q0-clippy6.log). Public typed contracts, assertions, resource reservations and failure/proof ordering remain intact. |

The non-model scoped diagnostic (`cargo clippy --release --workspace --exclude lctx-model --all-targets --no-deps --quiet -- -D warnings`) **passed**, but does not replace full Clippy.
Eleven clean, fully integrated task worktrees were removed under the current cleanup instruction after ancestry/cherry-pick/patch checks. Four dirty and three unproven-integration task worktrees remain preserved, together with external spike worktrees and caches.

The coherent `NEXTEST_TEST_THREADS=8 just test-all` retry on3ebad979 [full2](raw/phase4-q0-test-all2.log) **failed** during release compilation: all sixteen active compiler jobs terminated with SIGTERM, without a preceding Rust diagnostic. Nextest execution, pytest, the separate PG gate and doctests were **not_run**. Cause is unconfirmed; the operator states their cleanup was long before this interruption and should not explain it. Compiled work remains for another retry. Store-check remains not_run; both must pass before Phase 4 exit is claimed.


The further `NEXTEST_TEST_THREADS=8 just test-all` retry [full3](raw/phase4-q0-test-all3.log)
**failed** after a successful 19m39s release build: 993 tests ran, 964 passed, 28 failed,
one exceeded the generic 300s timeout, and 12 were skipped. This is a fixture-repair receipt,
not full qualification. Python, the separate PostgreSQL gate and doctests were **not_run**.
Failures identified stale generic transfer names, incomplete model-reference closure, missing
explicit inactive embedding/retrieval definitions in full-model conformance fixtures, native
inventory/publication-boundary and resource-pool fixture errors, a pre-cutover unsupported-frontier
control, and the unrefreshed model snapshot. The four-compilation upper CLI control receives the
same 900s allowance already applied to the six-compilation control; neither is a performance claim.
The model `.snap.new` was inspected before `cargo insta accept`: 262→784 declared relations,
102→196 invariants, seven generic authority relations replaced by nominal owners, 14 retained
relations changed, and all 281 existing codebooks retained as unchanged prefixes. This is the
Phase 4 schema migration; no compatibility reader was added. Current runtime reruns are pending.


Focused model reruns are a **composite passed** for the repaired scope: the first compiler
rerun [model1](raw/phase4-q0-model-fixtures1.log) failed two test-only imports; [model2](raw/phase4-q0-model-fixtures2.log)
then passed90 unit/6 inventory tests but exposed full-model prefix requirements in lower fixtures.
[model3](raw/phase4-q0-model-fixtures3.log) passed90 unit,3 memory,10 stage and6 inventory controls;
its input model still omitted two declared invariant inputs. After adding those nominal targets,
`cargo test --release -p lctx-model --test domain_input` [model4](raw/phase4-q0-model-fixtures4.log)
**passed**, all6 input controls. Assertions and codebooks remain intact. The
[accepted snapshot rerun](raw/phase4-q0-model-snapshot2.log) **passed** using the previously compiled
model-description binary; its enclosing full current-tree rerun remains pending.


`cargo test --release -p lctx-postgres --test domain_transfer --test native_inventory
--test installation --test generation_catalog --test unicode_values --no-fail-fast`
[PG fixtures1](raw/phase4-q0-pg-fixtures1.log) **failed**, three targets. Native inventory's actual
Facts-prefix/omission/forged-status controls and Unicode transport **passed**. Transfer reached
control-stage completion then refused its incomplete publication transport. Complete-model
installation/catalog lifecycle controls now close real scheduled vocabulary prefixes, but final
validation exposed missing canonical synthesis settings. Both fixture corrections require rerun.


The full-model [PG fixtures2](raw/phase4-q0-pg-fixtures2.log) rerun **failed** one target:
generation catalog2 and installation8 controls passed, including the drift matrix, busy/reset,
lease, inventory and model-reinstall controls. Installation's clean-state control exposed an
actual inspector discrepancy: reconstructed non-staging vocabulary views had NULL ACL, while
real views retain explicit owner-only ACL after sealed-phase revocation. The correction replays
the shared sealed lowering phase after reconstructing prefix views and retains exact ACL/grant
comparison; current production rerun remains pending.


`cargo test --release -p lctx-postgres --test installation --test domain_transfer --no-fail-fast`
[PG fixtures3](raw/phase4-q0-pg-fixtures3.log) **passed**: transfer1 (15.67s) and installation9
(147.20s), including exact clean ACLs across every state, a sealed-prefix unauthorized-grant/refusal
and revocation/clean twin, and the complete existing drift matrix. Transfer now has actual Facts
and completed source grants with honest incomplete conformance coverage; the original roundtrip,
derivation/provenance, scope mutations and failed-state assertions remain. This is a model-contract
fixture, not qualification of a behavioral provider.

The direct fresh-CLI [live check](raw/phase4-q0-live-store1.log) **failed** on the earlier model's
installation. Read-only inventory found zero readers and one unselected published facts generation.
The [reset dry run](raw/phase4-q0-store-reset-plan.log) refused pending confirmation as expected;
`target/release/lctx store reset --confirm lctx` [reset](raw/phase4-q0-store-reset1.log) **passed**,
dropping that obsolete generation and reinstalling the current model. This authorized regenerable
state pivot does not replace the pending named `just store-check` or disposable-generation tests.

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
