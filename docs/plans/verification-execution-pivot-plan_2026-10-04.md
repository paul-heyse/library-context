# Verification execution pivot

**Implemented / Tested; assembled qualification composite passed, 2026-10-05.** Supporting design for the
[testing architecture coordinator](testing-architecture-pivot-plan_2026-10-04.md), which owns
F01–F04 disposition and combined acceptance. This plan owns replacement test families, fixtures,
independent challenges, preparation and commands. Runtime validation meaning and proof reuse belong
to the [validation plan](validation-execution-pivot-plan_2026-10-04.md).

## 1. Rebuild the required assurance footprint

Derive controls from current product meaning and failure mechanisms, then map existing controls
to those obligations. Do not start by preserving every test or making the historical suite pass.
Every test, oracle, snapshot, fixture, gate and evidence folder is eligible for replacement or
deletion. A unique guarantee still needs a meaningful challenge; duplicative assertions and promises
deliberately removed from the current contract do not.

The current release-profile bare Cargo route already permits package/target selection. The broad
`just test` recipe adds unconditional root `uv sync --locked` first, even when the selected test
requires neither Python nor PostgreSQL. Root project dependencies currently include both native
adapters and MCP. `just test-all` also builds the CLI and prepares PostgreSQL before its broad
Rust/Python/doc-test run. `just hygiene` couples unrelated docs/policy checks with all-target Clippy
and a live default-store check. These are replacement surfaces, not required target architecture.

There are 223 integration targets in the source review's static inventory and 13 includes of
`crates/lctx/tests/fixtures/serving_support.rs`. These are packaging observations, not measured
costs or target-count goals. The strongest specific opportunity is repeated complete Catalog
construction for compatible read-only assertions. Keep cohesive semantic groups; do not turn all
tests into one enormous binary or consolidate unrelated providers for a smaller file count.

## 2. Families, expectation sources and effects

Use cohesive harness modules within existing crates first. A shared support crate is warranted
only where actual cross-crate reuse outweighs its dependency/compile surface. No generic fixture
framework, global assertion-result cache or new orchestration service is selected.

| Family | Actual scope and owner | Required preparation and expectation source |
|---|---|---|
| Pure semantics | `lctx-model`, `lctx-analytics` and other pure kernels: identity, model closure, uncertainty, wire values, finite algorithms, outcomes and resource ownership. | Constructed typed inputs and finite independently authored known answers/counterexamples. No PG, CLI acquisition or native wheels. Existing FCA/odis/fcars controls remain bounded and distinct. |
| Provider conformance | `cpg-extract` and `cpg-flow`: captured source identity, actual native Ruff/ty/Pyrefly configuration, source/role correspondence and facts. | Declared analyzer inputs and real provider APIs. Same-provider CLI checks driver parity; authored shapes or independent providers/runtime observations challenge semantic truth separately. |
| Store and validation | `lctx-postgres` and relevant `cpg-core`/CLI boundaries: model lowering, COPY/closure, privileges, receipts, validation bindings, audit, leases and recovery. | Real disposable PostgreSQL 18 through the generation store. Minimal model-shaped data for physical/lifecycle controls; use real producers only for composition claims. |
| Native and transport | Python adapters, `lctx` and MCP: real conversion, process/worker lifetime, wire listing/calls, errors, cancellation and guards. | Only the adapters used, real Python/native paths, explicit wire expectations and real FastMCP transport. Function-only checks do not certify transport. |
| Independent runtime challenges | Generated CPython raw-flow and served-claim observations; bounded finite analytic oracles. | Generated executable programs outside `fixtures/python`, pinned interpreter, time/example bounds and independently obtained observations. Complete compiler comparison is required for a correctness claim; observation-only runs have their narrower label. |
| Representative assembled journeys | Catalog/behavioral compilation through serving, admitted generation/lease and native/MCP wire behavior. | A small set selected by unique cross-boundary contracts; real store/providers/adapters. No real-library activation or product comparison unless separately authorized. |
| Tooling and documentation | Build/readiness scripts, ADR/docs resolver, fixture/gold registration, dependency family policy, Python lint/types and Rust compile-fail contracts. | Scope-specific tools/data. No compiler reconstruction for documentation; gold remains evaluation-only and heldout remains sealed. |

For every migrated group, state the contract challenged, its owner, the failure it can expose and
the expectation's independence. Keep this beside cohesive harness code and architectural owners;
the temporary migration table in the coordinator is enough for execution. Do not add a permanent
coverage registry requiring every test to register itself.

Remove redundant test-only validator implementations. Publication and tests use the same pure
model validators; independent controls deliberately calculate different expected evidence where
shared logic could reproduce a defect. Generated physical checks challenge declarations/lowering
with invalid external rows, not every constructor field through duplicate assertions. Retain
meaningful ID/state compile-fail and schema migration controls; an unchanged generated snapshot is
not independent evidence that meaning is correct.

## 3. Harnesses and immutable fixture cohorts

Consolidate cohesive integration targets into one target with internal modules to reduce repeated
compilation of large helper code. Runtime sharing needs a second decision: Nextest executes each
test in its own process, so a static/OnceLock in a target does not share setup across test functions.
This is **Interface-checked, 2026-10-04**, using
[official Nextest process documentation](https://nexte.st/docs/configuration/extra-args) and installed
Nextest 0.9.146. Do not claim fewer target binaries alone removes repeated fixture setup.

For expensive compatible read-only cases, use one explicitly named cohort test that prepares a
seed once and calls independently named case functions. Each case reports its result and the
cohort evaluates remaining cases after an assertion failure, then fails with the named failures.
Use ordinary test assertions/results and a small local case runner; no new general test framework.
A panic-safe boundary may collect ordinary assertion panics, but a poisoned fixture or failed
preparation fails the cohort and reports its unexecuted cases rather than claiming them passed.
All cases' names remain discoverable in source/output; the cohort is the runner selection unit.
Cheap pure tests retain individual test functions for granular selection and parallelism.

The cohort owns the disposable database/container, source capture, installed model and completed
generation. Its seed specifies source bytes/release/profile/model/configuration, publication
frontier and immutable generation identity. Reuse compatible read-only cases over that seed;
each case opens fresh grants/leases, budgets, request state and service/client guards and closes
them before the next case. Admit current resources even when seed preparation is reused.
Different semantic configuration or earlier/later frontier uses a different seed.

Privilege, lifecycle, selection, cancellation, repair, corruption and writer races may mutate
shared state. Give them fresh or safely cloned disposable state with owned cleanup; do not execute
them against a cohort seed consumed by unrelated cases. Cloning is an optional local optimization
only after the clone establishes independent database/generation/config identities. Test failure,
panic, timeout and cancellation must release resources or make the owning run fail explicitly.
Do not make one test's pass depend on execution order or another case's mutation.

Start with the read-only families including `serving_support.rs`; inspect their actual mutations
before grouping. Replace full compile-through-Catalog fixtures with smaller typed/store seeds only
when the control does not claim producer integration. Keep representative complete producer-to-wire
journeys and independent served fidelity challenges. Direct insertion is appropriate for a store
lowering control but cannot be labeled actual provider or serving-composition qualification.

Cross-process fixture sharing is not the initial target. A setup-script/server would require a
new ownership, readiness, cleanup and crash protocol; cohort lifetime supplies the present need
more simply. Reopen if cohort granularity materially harms diagnostics/parallelism or the same
expensive immutable seed has many genuine cross-process consumers. No persistent mutable test
database or globally cached previous pass is introduced.

## 4. Preparation and reuse

Preserve Cargo release-profile artifact reuse, stable normalized paths and the current O2 workspace/
O3 imported dependency policy. No shared `cargo clean`, new global optimization level or extra
frontend threads. Continue the build-environment wrapper for bare Cargo/Python invocation.

Python readiness follows imports/effects, not all root dependencies. Use existing uv workspace
selection and dependency groups; this is **Interface-checked, 2026-10-04**, against the official
[uv workspace](https://docs.astral.sh/uv/concepts/projects/workspaces/) and
[sync](https://docs.astral.sh/uv/concepts/projects/sync/) documentation through Context7. Intended
primitives are `uv sync --locked --package lctx-semantics --inexact` for its native closure,
the corresponding `--package lctx-storage`/`lctx-mcp` for those consumers, and
`uv sync --locked --only-group dev --inexact` for root tooling without product installation.
For an assembled session, prepare the union of selected package/group requirements once, before
tests start. Member dependencies may legitimately require another adapter; follow actual closure.

Scoped readiness uses inexact sync because exact subset sync can remove packages another selected
family needs from the shared environment. Sync only before the run, serialized through the existing
environment ownership; no syncing while native grants/workers are live. Root groups are not assumed
to be inherited by a selected member: request needed tooling groups explicitly. Verify actual
installed distribution availability/content after switching scopes, not just an old readiness flag.
No new per-family virtualenv is needed initially. If concurrent environment preparation cannot be
made safe with this lifecycle, isolate run-owned environments rather than weaken readiness.

Keep each native adapter's declared-input membership/content key in
`scripts/build_environment.py` and its own pyproject. A source addition/deletion, shared input,
lock/model or build configuration change invalidates readiness. After successful readiness, use
`uv run --no-sync` for the selected tests/scripts so assertions cannot silently rebuild dependencies
or change the environment under an active fixture. Scripts importing no product use root tooling;
pure Rust verification invokes no uv sync at all.

Preparation reuse means cached artifacts and session-owned immutable fixture state. Assertions run
fresh. Do not add a persistent pass cache keyed only by source digests: database effects, process
lifetime, ambient tools and incomplete runs make its authority unclear. Complete runtime validation
proof reuse belongs to the store contract, not to the test runner.

## 5. Oracle and evidence replacement

Audit the actual invocations behind `validation-and-evaluation.md` §8.1. The named Pysa TITO test
and old `cpg-core --test compile` target are absent. The two CrossHair `typing.cast`/`assert_type`
specializations and the Pysa evidence folder do not establish current compiled/served correctness.
Remove these current coverage claims and retired reproduction pointers. Transfer any substantive
active requirement into the current model/provider/serving owner, then delete those obsolete folders,
index rows and backlinks. Prior success or a backlink alone is not a consumer.

Do not rebuild historical Pysa/CrossHair paths merely to preserve an instrument list. A current
TITO obligation can use a justified bounded independent challenge; choose one only if the actual
claim and source correspondence need it. Pysa's shared provider graph is not independently correct
because its taint result differs. Timeouts, obscure results and unexplored paths remain inconclusive.
No passing label for a mocked or observation-only comparison.

Retain and consolidate current generated-runtime challenges from
`tests/scripts/test_flow_soundness.py`, `test_semantic_soundness.py` and the served-claim controls,
with their distinct raw-flow versus serving claims. Compute common generated inputs/observations
once within a compatible cohort, but independent CPython expectations remain outside compiler
inputs. Explicit seeds and example counts bound each run; counterexamples get minimal focused
regressions rather than an ever-growing replay campaign. Execute only generated programs, never
the analyzed library or `fixtures/python`; no network, gold feedback or heldout access.

Keep the implemented analytic K1/K2 controls: finite incidence double derivation, independent
concept enumeration and odis consequences establish different claims. Compare normalized semantic
sets, positive-support semantics and sound partial results, not identical basis spelling. Their
bounds remain at most eight attributes/twelve objects with explicit finite case counts. Production
charged-kernel fit and O08 disposition remain at the analytics owner/coordinator. Move enduring
rationale there and retire the supporting analytic plan once its last active consumer moves.

## 6. Commands and cadence to implement

Implement small explicit recipes, rather than a generic impact inference engine. Proposed public
names below define the intended scope; implementation can refine spelling when updating every
consumer together. Each selected family accepts ordinary filters and errors on an empty/unrecognized
selection; `--no-tests=pass` must not hide a missing required qualification family.

| Command | Selection and prerequisites |
|---|---|
| `just verify-model`, `just verify-analytics` | Release package-scoped pure controls; analytics includes bounded dev oracles. No native/PG readiness. |
| `just verify-providers` | Selected extractor/flow targets and their actual CLI/provider prerequisites only. |
| `just verify-store` | Real disposable PG controls over selected store/model boundaries; no default operator store. |
| `just verify-serving` | Native conversion, MCP/wire and read-only cohorts plus isolated lifecycle cases; prepare their exact adapter/CLI/store closure once. |
| `just verify-oracles` | Bounded actual raw-flow/served semantic comparisons; explicit producer/adapter prerequisites, no duplicated invocation through another family. |
| `just verify-tooling` | Selected scripts/build/docs/compile-fail controls and their declared environment; documentation-only changes select docs checks. |
| `just qualify` | One assembled target run: union readiness once, all required families and representative production journeys once, required compile-fail/doc contracts and full keep-going Clippy. Applicable non-functional checks run once for the same tree. |

Ordinary functional work runs compile checks and focused affected contract controls. At functional
scope completion, rerun affected consumers and their applicable non-functional checks. Widen to
assembled qualification for this pivot, a changed shared model/receipt/trust/transport contract,
or uncertainty that selected controls cannot resolve. Record chosen scope and limits explicitly;
selection is implementer judgment, not a hidden dependency algorithm. Minor unrelated documentation
or library changes do not automatically trigger the assembled product gate.

T4 replaces `test`, `check`, `test-all`, `py-test`, `oracles` and `hygiene` where their broad or
duplicated meaning is superseded. Preserve useful leaf checks, routing them to affected surfaces;
remove unused aliases rather than keep a second legacy gate. `store-check` must support an explicitly
owned disposable configuration in qualification; the operator-store inspection remains an explicit
operator action. Full Clippy uses `--keep-going`, Nextest uses `--no-fail-fast`, and cohort reporting
collects independent failures. The assembled launcher collects results across independent families
without skipping the remainder after the first failure; readiness failure blocks its dependents,
reports the missing prerequisite and still permits unrelated families to run.

Update AGENTS, binding, justfile, command comments, affected plan/runbook consumers and the governing
ADR together before replacement acceptance. Supersede the relevant ADR-0110 cadence rather than
editing its accepted bytes; the root owns formatting/generation through scope-end `just turn-end`. Existing Q0/Q1 owners map
outstanding acceptance to required replacement families, retaining open status until actual evidence
passes. This is a deliberate gate-policy change, not a claim that focused checks equal old full gates.

## 7. Work packages and verification

| Package | Deliverable and responsibility | Prerequisite and acceptance |
|---|---|---|
| T0 — Establish contract/control map | Root and family owners identify current promises, unique failure mechanisms, independent expectations and actual side effects; map open qualification obligations. Correct F04 owner claims and retire obsolete evidence after transfer. | Coordinator P0; static source/consumer audit suffices. Every retained requirement has a current owner and proposed replacement. No exhaustive flow tracing or old-green run. |
| T1 — Consolidate pure/provider harnesses | `lctx-model`, analytics and extraction owners group cohesive modules, delete duplicated validators/multiplicity assertions and preserve independent challenges. Native parity remains labeled parity. | T0 map; validation V2 interface only for tests of changed registration. Focused release controls catch positive/negative, missing/partial and budget distinctions. |
| T2 — Scope readiness | Build-environment/Python owners implement §4 recipe preparation and content/membership invalidation. Pure/docs routes never prepare adapters. Run-owned readiness is established before fixtures invoke `--no-sync`. | T0 imports/effects map. Controls reveal add/delete/shared-input/model/lock invalidation, member/group selection and no package-removal contamination across scopes. Actual adapter imports/model agreement pass. |
| T3 — Consolidate immutable cohorts and oracles | Serving/store/oracle owners migrate repeated complete fixtures to minimal contract seeds or shared read-only cohorts; isolate mutation cases and keep representative end-to-end journeys. Consolidate runtime observations only with exact input correspondence. | T0 semantic map, T2 readiness; validated immutable-store contract for cohorts relying on new proofs. One seed setup per compatible cohort; named case failures collected; fresh guards, failure cleanup and deliberate contaminating mutation controls pass. |
| T4 — Replace command/policy surface | Root implements §6, updates ADR/owners/AGENTS/binding and qualification consumers. Deletes legacy gate/preparation aliases and registers no competing ledger. Routes leaf checks by affected surface. | All family migration functionally implemented; coordinator P3 / validation V4 ready for its required controls. Selection lists required targets, fails missing families, and no-fail-fast reporting exercises independent failures. |
| T5 — Assembled acceptance and retirement | Root runs new qualification on the current tree, fixes failures, reruns targeted affected controls, records scope/results at coordinator and hands off. Removes superseded helpers/plans/review only after their surviving obligations have homes. | T1–T4 plus validation V1–V4. Real PG/native/MCP and full keep-going Clippy pass; selected receipts alone do not satisfy assembled qualification. Optional timings are limits if not run. |

T1 and T2 can proceed independently after T0, with root retaining ownership of shared recipes.
T3 can consolidate source modules before proof reuse lands; its new proof-dependent assertions
require the implemented validation contract. Deletion happens with each replacement; T5 audits
remaining stragglers rather than postponing required consumer migration.

Acceptance challenges the new architecture: no unrelated readiness for pure tests; no stale native
artifact after declared-input changes; exactly one shared preparation for compatible read-only
cases; isolation for mutations; honest reporting of all independent cases/families; current oracle
claims backed by live comparisons; complete required target selection; and real assembled contract
behavior. Structural setup/scan counts can be Tested without asserting speed. Time preparation,
compilation, checks and resource use separately if measuring benefits; coordinator §6 owns the
measurement boundary. No new full legacy run, historical evidence retention or performance campaign
is required to implement and qualify this pivot.
