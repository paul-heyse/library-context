# Semantic-model incremental alignment — coordinated implementation plan

**Proposed · 2026-10-02.** This series converts the
[incremental alignment review](../design_review/reviews/design_review_semantic-model-incremental-alignment_2026-10-02.md)
into bounded implementation work. It retains the implemented semantic model and its single
owner, hard compilation layers and PostgreSQL generation lifecycle. Publishing this series
implements the requested planning work; production corrections require execution authorization.
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

All remedies are **Scheduled / Proposed**, 2026-10-02; implementation checks are **not_run**.
These rows own subsequent state and closure evidence. Source review §§7/9 remain the dated
diagnoses/opportunities. O1–O6 are local shorthand for its numbered §9 entries, not new defects.

| Source ID | Scheduled correction | Closure boundary / current evidence |
|---|---|---|
| F01 | N0 exact native refusal | Exact causes and unknown/original-identity preservation, including legitimate defaulted formal; Proposed |
| F02 | N1–N3 pure native preparation | All semantic composition moved, store admission retained, independent legitimate/tamper cases; Proposed |
| F03 | E3 one retained analytic policy | Records/identity/kernel settings and all forty community runs preserved; Proposed |
| F04 | E0 finite binding | Every upper route, phase, graph need and publication group integrated; Proposed |
| F05 | E1–E2 exact closure/grants | Four builders/loaders migrated, distinct epochs and validator order preserved; Proposed |
| F06 | T0 gate composition | All intended controls retained, one integrated workspace execution; Proposed |
| F07 | D0 meaningful discovery | Finite serving inventory and actual tools/list explain numeric choices; Proposed |
| F08 | T2 owner reconciliation | Current navigation/labels with interrupted qualification retained; Proposed |
| O1 (§9.1) | E4 borrowed CPU/progress | Heartbeat, fallback, cancellation boundary and retained drain controls; Proposed |
| O2 (§9.2) | E5 shared S0 analytic parents | Three relations decoded/retained once; independent replay scope unchanged; Proposed |
| O3 (§9.3) | D1 typed packet bindings | All bindings, empty attempted reads, child and prepared coverage; Proposed |
| O4 (§9.4) | T1 declared runtime scripts | Owned-input mutation and unrelated-script stability controls; conservative other roots retained; Proposed |
| O5 (§9.5) | D2 admitted safe failure metadata | Actual tool/resource payloads and complete admitted error envelopes; early-error limit retained; Proposed |
| O6 (§9.6) | T2 guidance + E0/E1/E2/D0 extension controls | Existing owners plus independent binding/invariant/epoch/schema challenges; Proposed |

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
