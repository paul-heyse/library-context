# Semantic-model incremental alignment — coordinated implementation plan

**Implemented; scoped Tested acceptance complete · 2026-10-03.** This series converts the
[incremental alignment review](../design_review/reviews/design_review_semantic-model-incremental-alignment_2026-10-02.md)
into bounded implementation work. It retains the implemented semantic model and its single
owner, hard compilation layers and PostgreSQL generation lifecycle. Execution is authorized;
§7 owns actual implementation state and acceptance receipts. The original plan-authoring
receipt remains scoped to authoring.
Phase 5 real-library qualification and activation remain stopped at the operator's request.

## 1. Purpose, baseline and foundation assessment

The intended result is a design that is easier to extend locally and inspect through serving,
without replacing its architecture. Native semantics move to their declared model owner;
analytic policy, stage binding and dependency closure have one authored representation;
discovery explains numeric choices; preparation and CPU execution use deliberately narrow
reuse/lifetime boundaries. Independent tooling and documentation improvements proceed without
waiting for product qualification. Every further opportunity was investigated with intent to
include a substantiated bounded improvement, rather than requiring a measured speedup first.

The baseline is main `42551010c579c2f7c9ccd1f7449d36911917ee99` plus 117 initially dirty tracked
paths preserved on 2026-10-02. Those changes are input, not changes authored by this plan.
Static source inspection and focused independent advice establish the Proposed corrections;
no runtime probe, production build or new product qualification was performed during authoring.
The source review remains **Revise / Implemented** at its original inspected boundary.
Its F01–F08 labels below always mean that review's IDs, not similarly numbered older findings.

The [architecture map](../design/README.md),
[semantic model §15](../design/sections/semantic-model.md),
[analytics §9](../design/sections/analytics.md) and
[synthesis/serving §11](../design/sections/synthesis-and-serving.md) remain architectural owners.
ADR-0085/0086/0106/0108/0114 constrain this work. Plans and reviews are not competing authorities.
Use core design principles 3.2, code-intelligence profile 1.3 and the repository binding.

The foundation assessment found useful contracts to preserve, rather than replace:

- Model declarations, checked constructors and shared validators already own semantic meaning.
  The actual-native preparation boundary, rather than the whole service, needs correction.
- Schedule already checks order and grants; exact epoch requirements must lower into its single
  grant per relation. Merely extracting current name-dedup loops would lose meaning.
- Generation guards, actual stored-row membership, charged preparation and original receipt
  reads remain independent of pure helper agreement and declared packet coverage.
- S0 already reads nominal inputs once; the proven duplication is decoding/retaining three
  analytic parent relations in two inventories. Reuse ends at that boundary.
- The existing pinned runtime supports borrowed blocking placement and ordinary MCP metadata.
  Neither mechanism supplies hard CPU cancellation or universally bounded early error envelopes.
- Conservative revision capture is sound. Only the broad scripts subtree has a demonstrated
  bounded narrowing opportunity; whole-model and workspace-source capture remain conservative.

These are static conclusions about design potential, not measured performance or newly tested
correctness. Existing Phase 5 and prior-phase receipts keep their original scope and date at
[STATUS](../../STATUS.md) and their current plan owners.

## 2. Document and responsibility map

| Document | Coherent target and implementation responsibility |
|---|---|
| This coordinator | Combined target, sole current disposition, shared dependencies/identities, F06 gate composition, O4 script capture, F08/O6 owner guidance and overall acceptance |
| [Native preparation](semantic-model-native-preparation-plan_2026-10-02.md) | F01 exact refusal and F02 pure prepared semantics/formal resolution; model meaning versus store effects |
| [Execution foundations](semantic-model-execution-foundations-plan_2026-10-02.md) | F03 fixed analytic policy, F04 finite stage binding, F05 exact closure/grants, O1 borrowed CPU/progress and O2 S0 analytic-parent reuse |
| [Serving discovery](semantic-model-serving-discovery-plan_2026-10-02.md) | F07 meaningful schemas, O3 typed packet/read bindings and O5 safe metadata within admitted failure envelopes |

One author/integrator owns shared declaration, build, identity, navigation and disposition
surfaces. Semantic ownership does not change when a package migrates consumers in several
crates. Companion documents specify contracts and acceptance; execution state lives only here.
The [independent proposed-target review](../design_review/reviews/design_review_semantic-model-alignment-target_2026-10-02.md)
assesses this combined design separately from the original implementation diagnosis.
Its 2026-10-02 verdict is **Accept scoped / Proposed**, with no new material target finding;
implementation closure and wider qualification remain separate.

## 3. Combined contracts and extension locality

Native preparation consumes borrowed selection data plus typed owned relation inputs and returns
opaque prepared semantics. PostgreSQL retains guard/receipt/membership checks, leases, budgets,
paging and response envelopes. The refusal correction precedes integration so unsupported
defaulted formals remain public and preserve DefaultStabilityUnknown rather than becoming an
invalid formal or generic EntryValueUnknown. Prepared helper agreement never admits proof rows.

The finite upper-stage inventory supplies declarations, publication groups, graph needs and
execution binding. Model closure takes that checked publication order and owner epoch policy;
it retains exact validator/consumption requirements, then mechanically projects sufficient
grants. Actual consumers read acknowledged epochs separately. Analytic policy supplies both
identity and executable kernel settings; no orchestration table acquires semantic authority.

Shared graph/parent preparation needs a common acknowledged source universe. CPU placement
retains those borrows and charges through execution. Separately spawned progress sampling owns
only operational measurements; cancellation cannot publish completion and executing opaque
kernels drain before release. No asynchronous detachment, worker registry or new hard deadline
is necessary for this bounded improvement.

Serving schemas derive labels from codebooks and semantic descriptions from model annotations.
Packet mappings derive from closed typed bindings; attempted reads, including empty reads and
prepared dependencies, check against them while canonical store admission stays independent.
Failures use existing pinned tool/resource mechanisms after original-grant and complete-envelope
admission. Early admission exceptions retain their existing safe path and explicit limit.

Future changes justify these seams without adding those features to this plan:

| Scenario | Target edit route and preserved distinction |
|---|---|
| New native path case | Model prepared operation and independent legitimate-case control; store does not interpret new path semantics |
| New analytic policy | One fixed authored policy changes records, complete identity and kernel settings; run observations remain separate |
| New upper stage or graph consumer | One finite binding plus model declaration; phase/grant compatibility checked before effects |
| New invariant dependency or earlier vocabulary read | Shared closure preserves exact epochs/order and sufficient grants; actual consumed sources remain owner-declared |
| New selection predicate or packet | Model vocabulary/semantics, generated meaning and typed packet dependency binding; no Python semantic dictionary |
| Substitute a numerical mechanism | Existing complete numerical-result boundary plus requalification; no broad provider framework |

## 4. Coordinator-owned work packages

### T0 — F06: one integrated Rust test execution

Current `test-all` includes the release workspace run via `check`, then `test-postgres` repeats
a package subset under the same profile/features/environment. Make the integrated command run
that workspace once while retaining every intended PostgreSQL control, Python/oracle suite,
compile-fail doctest and required release CLI build. The standalone subset remains convenient.

Add an ordinary read-only PostgreSQL image readiness prerequisite covering both pinned images;
pull/setup remains with existing setup/end-of-turn mechanisms. Keep an explicit CLI-build
prerequisite. Proposed recipe composition is `test-all: postgres-test-ready test-cli-build check
test-doc`, and `test-postgres: postgres-test-ready test-cli-build` followed by its existing
package subset. Preserve `check`'s Rust/Python meaning, INSTA_UPDATE=no, release profile,
no-fail-fast, store fixtures, oracles and docs tests. Confirm prerequisites run before consumers
and only once through Just dependency composition; do not accidentally remove standalone tests.

Remove the subset invocation from the integrated path and update command documentation.
Coverage comparison uses current package/test inventory and flags, not a passing shortened
suite alone. The final authorized integrated gate establishes the new composition; it does not
close Phase 5 Q0. Keep Cargo jobs16 and default single frontend thread; Nextest8 caps test
processes rather than requesting eight threads per process. No unexplained SIGTERM diagnosis
or performance promise is attached to deduplication.

### T1 — O4: declared embedded scripts, conservative remaining capture

Current [producer fingerprint](../../scripts/producer_fingerprint.rs) captures all workspace
crate src/sql/migrations/models, manifests/build scripts, workspace Cargo/lock/toolchain,
third_party, specs and scripts. A production-source search found the embedded deployment
runner bytes in [deployment.rs:257](../../crates/cpg-extract/src/deployment.rs#L257), plus the
fingerprint mechanism used by the extraction build. The runner's own imports are standard
library modules; it has no sibling-script import closure. Administration/documentation scripts
are not production inputs merely because they share that directory. Whole-model source capture
already excludes these scripts and remains untouched.

Introduce one small embedded-runtime-script declaration owned by extraction, shared by the
build fingerprint and the deployment receipt's embedded bytes. An ordinary macro/declaration
can emit paths for build-time capture and named byte constants for runtime use; it must be
usable in both contexts without parsing Rust source. Include the declaration and fingerprint
mechanism themselves in capture. Re-audit direct script embedding/reads at implementation time;
any additional actual runtime script joins that declaration before narrowing the wildcard.

Replace only whole-scripts traversal with those declared inputs. Retain every other conservative
root and unused-sibling/input-membership invalidation. No generic include scanner, per-crate
revision system, cache service, new generator or split semantic-model revision is planned.
The tradeoff is a small explicit script-maintenance obligation backed by a shared embedding
site, in exchange for preventing unrelated administration edits from changing producer identity.

Mutation controls must show runtime bytes, a newly declared script and fingerprint mechanism
changes invalidate identity; unrelated script edits/additions do not. Keep all existing positive
source/model/spec/manifest/lock/toolchain/patch mutation controls and relocation/cache exclusions.
Do not claim compile speedup without measuring it. The first producer digest change is an
intentional hard cutover; reconstruct derived state from final pinned inputs, never use an
old-format reader. Record this identity choice through the ADR/owner route during implementation.

### T2 — F08/O6: current owners and concrete extension guidance

Reconcile the architecture map, design binding, product-target flow, AGENTS.md command/availability
descriptions and adjacent analytic/serving owner prose with actual Phase 0–5 implementation
and the stopped qualification boundary.
Replace dormant cpg-schema/returning-normalization references with the declared current owners.
Retain accepted ADRs unchanged; amend owning sections when implemented meaning changes and
write/supersede an ADR only for a changed decision or consequential alternative.

Include a concise owner-oriented extension route in existing documentation, not another
register: a fact family joins typed relations/support, append-only family/provider coverage,
frontier, normalization and owning invariant; a finite model joins checked model construction,
its declared inputs and production/replay controls; a predicate joins model vocabulary,
classification/algebra and generated schema. Existing Basic/Extended family controls already
establish part of this route. No new fact family, heap identity or unrequested feature is added.

O6's executable contribution is carried by the concrete E0/E1/E2 and D0 controls: missing stage
bindings, extra invariant dependencies, distinct vocabulary prefixes and generated descriptions
must be challenged through independent expected behavior. Document where each extension goes
and which invariants it must independently challenge. A universal invariant/capability registry
would duplicate existing admission and is rejected. R4 simultaneous multi-variant Summary work
keeps its separate trigger; this plan does not mark it complete.

F08 closes when owner→decision→open-work navigation reaches current owners without conflicting
implementation labels. Keep source-review dated findings and incomplete qualification receipts.
Move surviving obligations to owners before deleting a document that loses its last current
consumer; no bulk documentation retirement is needed here.

## 5. Dependency order and shared editing surfaces

| Ready work | Contract needed before dependent integration | Parallelism and shared files |
|---|---|---|
| N0 exact refusal; T0 gate composition; T1 script capture; D0 schemas; initial T2 prose | Existing owner contracts | Independent capabilities; model/wire edits still need one identity integrator |
| E0 finite binding/order; E3 analytic policy | Existing declaration and policy owners | Logically independent; E0 must expose existing checked order before E1 integration |
| E1 closure → E2 consumer migration | Working order contract, then working closure operation | E3 and E2 share analytics/build.rs; serial patches by one writer |
| N1 public formal domain → N2 pure semantics → N3 service integration | N0 refusal meaning; borrowed selection seam; exact epoch conventions from E1 where used | N2 and D1 share selection/native service surfaces; coordinate ownership rather than concurrent edits there |
| E4 CPU/progress | Integrated E0 stage/runtime surface | No dependency on completing all analytic-policy work; borrowed lifetime remains existing contract |
| E5 S0 reuse | E2's actual consumed-input/epoch migration | Shares synthesis producer/replay surfaces; migrate together |
| D1 packet binding; D2 failure metadata | Current canonical readers; current admitted envelope contract | D2/D0 share schema/WireIdentity/bridge; integrate serially. D1 needs no universal stage rewrite |
| Final T2 reconciliation, review and acceptance | All functional packages integrated | Reconcile actual owners and removal; one final tree for combined gates |

This is an order of prerequisite capabilities, not whole-document barriers. A settled Proposed
interface permits dependent design; only a working migrated contract permits dependent acceptance.
Native preparation can start before E2 is complete, but cannot claim exact-input integration
while using obsolete loaders. Implementation discretion may combine tightly coupled slices;
preserve their independent acceptance obligations. No temporary parallel authority is needed.

Before production edits, inspect the current dirty tree and assign file ownership. Keep one
writer for declarations, model build/identity, analytics/build.rs, compilation/stage runtime,
selection/native services, Python wire/server, justfile and shared documentation. Delegation
serves independent scope; worktrees are only for genuinely concurrent production editing.

## 6. Identity, removal and regeneration

| Changed surface | Required identity consequence |
|---|---|
| Native refusal/preparation/formal resolution | Native definition captures all moved semantic sources, including inventory and new preparation; global model capture rotates |
| Analytic policy | Complete structured policy supplies definition semantic identity and kernel settings together |
| Closure/stage declarations | Schedule digest and exact consumed receipts retain checked order/epochs; captured model/producer identities reflect source changes |
| Schemas/public failure contract | WireIdentity includes descriptions and failure shape/meaning; model capture reflects owner changes |
| Packet dependencies | Derived packet mapping identity captures typed sources and composition |
| Embedded runtime-script closure | Producer identity changes for owned runtime inputs; unrelated script changes cease invalidating it |
| Operational sampling | No telemetry timing or placement enters semantic content digests |

Remove each redundant authority with its migrated consumer: generic refusal substitution,
store semantic composition/formal reconstruction, duplicated policy defaults/recipe, stage
name lists/dispatch, four closure loops and blanket epoch overwrites, duplicate S0 parent stores,
string-authored packet bindings and whole-scripts fingerprint traversal. Shared validation and
independent challenges remain; deletion never means weakening a guard, oracle or control.

At final integration, inventory affected generations/caches/projections and current readers.
Quiesce only project readers, preserve current receipt obligations, then retire/reset obsolete
derived state through existing store/generation services and reconstruct from pinned inputs
when that functional validation is authorized. A failed/interrupted rebuild remains unselected
and resumable through existing lifecycle; no rollback assets, historical runtime authority or
compatibility readers are retained. Do not restart the stopped FastMCP qualification campaign
merely to retire changed model identities. Record any deferred reconstruction as a limit and
prerequisite to serving activation; never call a stale generation validated for the new tree.

## 7. Current disposition — sole execution owner

All remedies are **closed / scoped Tested**, 2026-10-03. The final complete functional gate
passed on production commit `50f58ffa`; every hygiene constituent passed through named repairs.
The receipt below preserves the failed first run, actual controls and qualification limits.
These rows own subsequent state and closure evidence. Source review §§7/9 remain the dated
diagnoses/opportunities. O1–O6 are local shorthand for its numbered §9 entries, not new defects.

| Source ID | Scheduled correction | Closure boundary / current evidence, 2026-10-03 |
|---|---|---|
| F01 | N0 exact native refusal | **closed / Tested**; native model and actual service/transport controls preserve exact causes, Unknown and original IDs, including legitimate defaulted formals |
| F02 | N1–N3 pure native preparation | **closed / Tested**; composition/formal resolution has one model owner; store admission/membership/tamper controls and both profile native fixtures pass |
| F03 | E3 one retained analytic policy | **closed / Tested**; record/identity/kernel mutation controls and analytic integration pass; all forty community runs retained |
| F04 | E0 finite binding | **closed / Tested**; closed stage/graph/publication binding and missing-route controls pass; both profile upper-frontier fixture compilation passes |
| F05 | E1–E2 exact closure/grants | **closed / Tested**; four builders/loaders migrated; extra invariant, distinct epoch/order, visible Catalog and missing checkpoint controls pass |
| F06 | T0 gate composition | **closed / Tested**; dry inventory and final complete gate retain one workspace execution plus real PG, Python/oracles, doctests and fresh native adapters |
| F07 | D0 meaningful discovery | **closed / Tested**; finite route/codebook/annotation schema controls and actual tools/list pass |
| F08 | T2 owner reconciliation | **closed / Tested**; current owner/navigation labels and extension guidance published; docs/ADR/agent checks pass, stopped qualification retained |
| O1 (§9.1) | E4 borrowed CPU/progress | **closed / Tested**; heartbeat, inline fallback, sampler stop and cancellation/drain controls pass; hard cancellation and speed remain unclaimed |
| O2 (§9.2) | E5 shared S0 analytic parents | **closed / Tested**; one decoded parent inventory, synthesis/documentary and independent replay controls pass |
| O3 (§9.3) | D1 typed packet bindings | **closed / Tested**; typed binding/child/prepared coverage, empty attempted reads and actual packet controls pass; canonical admission remains independent |
| O4 (§9.4) | T1 declared runtime scripts | **closed / Tested**; declared-input mutation/deployment and unrelated-script stability controls pass; conservative other roots retained |
| O5 (§9.5) | D2 admitted safe failure metadata | **closed / Tested**; actual legacy/modern HTTP/stdio tool/resource failure bytes and original-grant complete envelope controls pass; early-error limit retained |
| O6 (§9.6) | T2 guidance + E0/E1/E2/D0 extension controls | **closed / Tested**; existing owner guidance plus independent stage/invariant/epoch/schema challenges pass; no universal registry introduced |

Older findings retain their identities/disposition at
[parent cutover §8](semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition) and
[Phase 5 §10](semantic-model-phase5-detailed-plan_2026-10-01.md#10-finding-routes-limits-and-current-state).
Closure here does not infer closure there or reopen their scoped closed codec/assertion findings.
PR6, R4, live embedding availability, real-library serving and activation keep their existing limits.

## 8. Verification, review and completion

During authorized production implementation run normalized release compile checks and targeted
tests/probes for the package's meaning. Use existing independent numerical, native, epoch,
store and transport controls rather than generating expectations from the new helper. Store
tests use real disposable PostgreSQL 18. Model/grant/type agreement alone cannot prove semantic
or transport fidelity. Inspect schema snapshot changes before accepting them as migrations.

Before changed ownership/identity decisions, use the ADR and owning-section route. A fresh
bounded design/target review of this Proposed combined scope precedes execution. At integrated
completion independently review native/store, closure/loader and serving interactions against
the same scenarios, distinguishing architecture from fidelity judgments. Static review can
settle design; planned runtime claims still require their appropriate controls.

After all functional scope is implemented, run `NEXTEST_TEST_THREADS=8 just test-all` and
`just hygiene` once for the same tree. T0 removes duplicated workspace/subset execution but
keeps all functional controls. Fix findings and rerun the failed named check; repeat broader
gates only for a subsequent material change or unresolved failure. Preserve policy/resource
settings and classify actual outcomes as passed/failed/blocked/not_run. Clippy and other hygiene
work remain required; formatters/generators remain end-of-turn-hook owned.

These future gates qualify the incremental corrections, not unrun real-library Q0 journeys.
No arbitrary throughput/RSS/retrieval-quality threshold is added. Measurements can later test
performance hypotheses when a workload or decision needs them; their absence does not keep
otherwise completed functional corrections open. Universal early-error envelope qualification
requires a separate transport decision before making that claim.

For this **plan-authoring** scope, run `just docs-check` after publication, check links/anchors
and preservation of initial dirty files, and record actual results at handoff. `just docs-test`
is only needed if publisher/resolver implementation changes. Product gates, Clippy, pilot
compilation and reconstruction are not_run for document creation; no new production claim is made.

Completion of later remediation requires every scheduled row's meaningful controls, complete
consumer migration/removal, current owner/ADR prose and passing combined gates with their tree
boundary. Record interrupted optional reconstruction/measurements as explicit limits rather
than fictitious success. Once this plan loses its last current execution consumer, carry surviving
obligations to enduring owners and retire it under the current-working-set policy.

### Plan-authoring receipt, 2026-10-02

| Actual action | Outcome and scope |
|---|---|
| `just docs-check` | **passed**; publication, ADR/agent metadata and offline links; 266 canonical pages, zero link errors |
| SHA-256 comparison with `/tmp/lctx-alignment-plan-baseline_2026-10-02.json` | **passed**; all 117 initial dirty tracked files unchanged during authoring |
| `git diff --check` on authored/staged documentation | **passed**; whitespace integrity only |
| `just docs-test` | **not_run**; publisher/resolver implementation unchanged |
| Production compile/tests, `just test-all`, `just hygiene`, pilot and reconstruction | **not_run**; plan-document creation only; stopped qualification preserved |

Four plans and the independent Proposed-target review are published. The coordinator owns
scheduled disposition; companion contracts, current-work navigation and STATUS agree. This
receipt establishes documentation completion, not any scheduled production correction.


### Implementation receipt, 2026-10-03 — scoped acceptance complete

Execution began on main `ed2d68d7` with unrelated library-utilization changes preserved.
T1/ADR-0115 is committed as `d84a2287`, T0 gate composition as `761687bd`, and the integrated
N/E/D implementation, architectural owners, ADR-0116 and independent review as `e3e7c78d`.
Adapter freshness is committed as `2a03adc8`; final transport/deadline control corrections as
`50f58ffa`. Concurrent operator guidance `628b018d` and unrelated role/catalog edits are preserved.
The [independent implementation review](../design_review/reviews/design_review_semantic-model-alignment-implementation_2026-10-02.md),
including Appendix C dated 2026-10-03, accepts the bounded design at **Implemented** strength.
Runtime acceptance is established separately by the commands below.

Persistent receipts reside in
`/home/paul/.cache/lctx-alignment-execution/receipts-2026-10-03/`; each completed command has a
log and exit file. Bare Cargo/uv commands use `python3 scripts/build_environment.py --` to
normalize artifact paths. Tests retain release settings, Cargo jobs16, frontend1 and
Nextest8 test processes. PostgreSQL controls use real disposable PostgreSQL 18.

| Current command / boundary, 2026-10-03 | Outcome and receipt |
|---|---|
| `cargo nextest run --release -p lctx-model --test dependency_closure -p cpg-core --lib -p lctx-postgres --test stage_validation --no-fail-fast` | **passed**, 109, zero skipped; `focused-controls.log`; includes extra invariant/epoch, visible Catalog rejection, missing checkpoint rejection, explicit granted read and CPU drain controls |
| `uv run --no-sync pytest -q python/lctx_mcp/tests/test_transport_envelope.py python/lctx_mcp/tests/test_wire_contract.py` | **passed**, nine; `wire-final.log` |
| `uv run --no-sync pytest -q tests/scripts/test_build_environment.py` | **passed**, nine; `native-build-inputs-rerun.log`; content changes, non-newest deletion, unrelated stability, member scope and shell exports |
| Uncached locked refresh of both native adapters, then `just native-adapter-ready` | **passed**; `abi-uncached.log`, `native-adapter-ready.log`; installed storage and CLI match all 240 captured current model sources in `adapter-source-agreement.json` |
| `just --dry-run test-all` | **passed**; `test-all-dry-final.log`; images, CLI, adapter readiness, one workspace Nextest, Python/oracles and doctests retained |
| First `NEXTEST_TEST_THREADS=8 just test-all` | **failed**; `test-all.log`, exit100; 944 run, 942 passed, two failed, two measurement controls skipped; Nextest ID `cd7780ff-daad-4249-a512-9fac259cdbe3`; Python/doctests not reached |
| Final `NEXTEST_TEST_THREADS=8 just test-all` | **passed**, exit0; `test-all-final.log`; Nextest ID `908dbabc-3b27-4703-8ee1-eb63df80e7b9`; 944 Rust passed, two optional measurement controls skipped, 230 Python/oracle passed, 32 doctests passed (20 compile-fail, 12 positive); includes adapter readiness |
| `just hygiene` and named repairs | **composite passed**; `hygiene-v2.log` stopped at Ruff; repaired Ruff and all remaining named checks passed; agent/ADR lint, fixtures, gold, rules scan/test, types, docs, deps, full workspace all-target Clippy and store-check are passing |
| `just clippy`, `just ruff`, `just types` after final control corrections | **passed**; `clippy-final.log`, `ruff-header.log`, `types-header.log` |
| `just docs-check` | **passed** after final disposition/handoff publication; `docs-final.log`, exit0; 269 canonical pages, 9513 links checked, zero offline link errors |
| `target/release/lctx store reset --confirm lctx`, `just store-check` | **passed**; `store-reset.log`, `store-check.log`; current model installed, zero generations and zero findings |
| FastMCP Q0, reconstruction, serving activation, R4 and performance/retrieval measurements | **not_run**; prior operator stop and separate qualification boundaries retained |

The first complete workspace execution exposed two control defects, corrected in `50f58ffa`:
modern HTTP resources supplied the tool name in the MCP routing header, and the cumulative
CPU deadline stopwatch started after admission while asserting the entire original duration.
The resource header now names its URI; the CPU control compares elapsed work against the
actual admitted remaining deadline and still challenges cumulative refusal/drain. These fixes
change test controls, not production runtime behavior. The first run's CPython served-path
soundness and shared embedding replay controls passed; its failures remain recorded rather
than being presented as an initially clean gate.

Earlier targeted integration failures established the repairs rather than closure: Catalog
inferred ordinary fact requirements retain the frozen Facts checkpoint while actual reads
retain sufficient grants; visible Catalog contributors are rejected within the consumed
vocabulary prefix. A1 typed projection dispatch follows its complete owner macro. The current
109-test receipt independently challenges those requirements, epoch distinctions and missing
checkpoint refusal. Earlier model/native/store/script and actual packet receipts remain
bounded to their dated source; the final full gate is the combined acceptance boundary.
A host restart interrupted initial checks and removed temporary logs; no pass is inferred for
those interrupted commands.

The final model digest is
`12242a5558134418fc3b54fdb526628d1715720e60e6199a8a6079f6066582da`.
The model-describe snapshot changes were inspected and accepted as a **schema migration**:
model digest and S0 input ordering changed, with no relation fields or existing codes changed.
Native adapter startup subsequently refused a stale storage capture despite a reported
successful build. The uncached refresh repaired it; explicit capture agreement supplies the
freshness receipt. Declared uv file patterns and per-member content/membership fingerprints
now cover transitive model/macros/store sources and embedded image specifications. Readiness
runs before Nextest while fixture invocation remains no-sync; no broad cache cleanup or
build serialization policy was introduced. Installed uv was 0.12.22; historical documentation
was not treated as proof of that installed version's behavior.

Retirement was scoped to one old unselected staging generation,
`de1e0c2b09a319134fcef595a9bcbe89`, after checking that there were no connected readers or serving
vectors/artifacts. Before mutation, `retirement-controls.json` preserved all 14 control tables,
204 stage receipts, nine outcomes, one checkpoint, 12 epoch receipts and 124 read checks.
SHA-256: `49dc3c2e4863df869151194544edfe3caf8a457f2e1b4fa3c5ae639ad00d0fa3`.
The reset dry run refused with expected exit2 after listing that generation; confirmed reset
and named store-check passed. No obsolete runtime authority remains in the local project
store. The archive remains outside runtime state; reconstruction from final pinned inputs is
still a prerequisite to activation and was not used to restart stopped real-library work.

Acceptance is scoped to these incremental corrections and fixture/transport controls. Failure
seams force operation outcomes through the actual native service, original grant and transport;
they do not establish physical backend fault behavior. Borrowed CPU placement does not promise
hard kernel cancellation. Early failures without an admitted request retain their existing
safe path; universal early-error bounds, retrieval quality, speed and total RSS are unmeasured.
The two skipped optional measurement controls require `LCTX_P0E_INPUT` or
`LCTX_PROJECTION_GENERATION`/`LCTX_PROJECTION_RECEIPT`; their absence does not close R4 or
real-library serving qualification. All scheduled functional corrections are complete within
this scoped acceptance. The next product/qualification action remains operator-authorized Q0
and reconstruction, rather than additional alignment implementation.
