# Design review: test, validation and oracle architecture

Command excerpts identify verification scope; original invocations remain in the cited logs and Git history.

**Design tier · Target purpose · 2026-10-04**

**Decision: Revise the assurance architecture.** Preserve model-owned enforcement and
independent semantic challenges; consolidate repeated validation instruments and prepare test
infrastructure at the boundary that actually needs it.

The current architecture distinguishes executable domain declarations, persisted structural and
lifecycle enforcement, strict derivation replay, and independent challenges to shared semantic
defects. These responsibilities should survive consolidation. The material problems are narrower
than “too many checks”: one source-call obligation is registered as three identical whole-stage
replays; the standard focused test recipe prepares unrelated native adapters; immutable test
inputs repeatedly enter complete publication journeys; physical validation preparation repeats
by consumer; and the validation owner presents a retired oracle seam as current coverage.

These causes justify architectural revision without establishing that published answers are
incorrect. Corrections and savings are **Proposed**. No full qualification, testing-time reduction
or memory reduction is claimed.

## 1. Scope and evidence boundary

The inspected baseline is `main`, `50615e06fed40a621afc9231b929f18d03454fdd`. Unrelated dirty
changes in `crates/lctx-workspace-hack/Cargo.toml` and `docs/library-utilization.jsonl` were
preserved and excluded. The principal assessment was produced by a fresh `design-reviewer`,
with two read-only code mappers and coordinator reconciliation of decisive source evidence.

The standard is Core **3.2**, its design-review template, code-intelligence profile **1.3** and
the [repository binding](../design_principles/binding/library-context.md). The architectural
owners are [validation and evaluation](../../design/sections/validation-and-evaluation.md),
[semantic model](../../design/sections/semantic-model.md) and
[storage and publication](../../design/sections/storage-and-publication.md).

The subject includes typed construction, generated structural admission, relational invariants,
source-sensitive publication, generation lifecycle, Rust/Python tests, provider conformance,
generated runtime oracles, fixture preparation, test target organization, functional gates and
hygiene interfaces. Existing gate policies are candidates for revision, not immutable premises.

This review does not certify every fact, algorithm or served packet. Real-library qualification
and activation remain stopped; heldout evaluation remains sealed. Product usefulness, live
embeddings and total RSS are outside the inspected scope.

Evidence is source inspection (**Implemented / Interface-checked, 2026-10-04**), existing scoped
receipts and one bounded observation probe. The recent `just test-all`
was cancelled during workspace compilation before tests launched. Its 15m41s CLI build is build
evidence, not a test-runtime pass or failure. Q0 remains open in [STATUS](../../../STATUS.md).
Existing timings inform the inquiry; they do not create a historical retention obligation.

## 2. Responsibilities, contracts and semantic ownership

The assurance domain contains distinctions that affect validity: valid construction versus valid
persisted content; structural versus relational validity; completed predecessors versus candidate
outputs; frozen prefixes versus later vocabulary; publication versus selection; replay agreement
versus independent correctness evidence; preparation reuse versus execution-result reuse.

| Owner | Responsibility and consumer contract |
|---|---|
| `lctx-model` | Typed identities, record declarations, local validation, pure relational checks, stage contracts, coverage and replay semantics; no I/O. |
| `lctx-postgres` | Physical lowering, COPY, actual content verification, privilege transitions, completion, publication and reader leases. |
| `cpg-core` / extraction adapters | Compose declared inputs/outputs with production mechanisms and preserve provider meaning and configuration. |
| Independent controls | Supply authored known answers, malformed persisted cases, provider parity and generated CPython observations; never write facts. |
| `justfile` / build environment | Prepare prerequisites and invoke tools without becoming another semantic authority. |
| Qualification owners | Decide what receipts establish and what remains unqualified. |

Dependency direction is appropriate: semantic contracts do not depend on the store, orchestrator
or transport (`crates/lctx-model/src/lib.rs:1–6`); storage consumes model declarations and check
factories (`crates/lctx-model/src/domain/model.rs:25–44`).

| Evidence family | Meaning and consumer |
|---|---|
| Native provider assertions | Attributed extracted/inferred facts under recorded configuration; consumed by extraction and normalization, not runtime proof. |
| Derived relations | Results under a named model and retained source universe; consumed by replay and publication. |
| Stored receipts | Actual content, counts and provenance; consumed by grants/admission, with matching metadata insufficient to prove current contents. |
| Generated CPython observations | Observed guards, returns and identity on bounded generated programs; independent counterexample evidence, not general proof. |
| Gold and registered questions | Evaluation expectations only; excluded from compiler inputs. |

**A2 is satisfied within the inspected enforcement scope:** the model includes the consequential
distinctions and governs these paths. This does not certify the complete compiler or serving model.

### Intrinsic guarantees and remaining runtime obligations

Nominal IDs prevent some typed cross-domain mistakes. `Batch::new` uses the declared record type,
validates rows, rejects conflicting payloads sharing an identity and owns reservations
(`crates/lctx-model/src/domain/record.rs:548–610`). Charged `Rows` applies owned validity/conflict
rules (`crates/lctx-model/src/domain/normalized/rows.rs:19–38`).

These guarantees do not prove reference existence, generation/prefix membership, provider/context
relationships or correct derivation. They also cannot protect raw SQL, persisted corruption or a
faulty lowering. PostgreSQL legitimately enforces structural contracts again: generated DDL
supplies generation membership, keys, code membership, widths, finite floats and nominal references
(`crates/lctx-postgres/src/generations/ddl.rs:214–315`). This is a distinct enforcement boundary
of the same declaration, rather than an independently authored domain definition.

Persisted negative controls also have a distinct purpose. Publication controls mutate stored rows
and require completion to refuse without leaving receipts or sibling results
(`crates/lctx-postgres/tests/publication_checks.rs:188–231`). Vocabulary controls corrupt frozen
contributions and require subsequent refusal (`crates/lctx-postgres/tests/vocabulary_epochs.rs:726–755`).
Typed construction does not replace these controls.

Repeated valid example construction is not independent evidence merely because it occurs in
another crate. Consolidation should retain semantic challenges where they establish meaning,
mechanical conformance where lowering can fail, and lifecycle controls where effects can escape.
Each retained control should identify its contribution.

## 3. Findings

<a id="F01"></a>

### F01 — One source-call replay obligation has three equivalent registrations

**Principles:** FP-03, FP-06, DP-06, DP-10, DP-16 · **A3 violated**

**Owner:** `lctx-model::domain::execution::source_call_records`

`SourceCallHeader`, `SourceCallRun` and `SourceInvocation` attach separate invariants
(`crates/lctx-model/src/domain/execution/source_call_records.rs:22`, `:57`, `:112`). The
`source_call_inventory`, `source_call_invocation_replay` and `source_call_header_replay` factories
all use identical `invariant_inputs()` and create the same `SourceCheck::new` (`:549–589`).
There is no mode selecting a narrower obligation. `finish` reconstructs and compares the entire
output universe: runs, headers, members, boundaries, outcomes, calls, releases and arguments
(`:643–751`).

`ValidatedModel` preserves the three distinct names (`domain/model.rs:299–339`), and the store
executes each separately (`generations/receipts.rs:287–315`). These instruments do not challenge
different semantics or implementations. A source-call extension expands three complete executions
of the same obligation, and their names obscure how much independent coverage exists.

**Proposed correction:** give the whole-stage replay one validation owner and one execution per
relevant input universe. Keep diagnostic subdivisions inside that operation where useful.
Alternatives are one explicit validation unit consumed by multiple declarations, or genuinely
narrower checks that no longer each reconstruct the whole stage.

Deleting two attachments alone is insufficient: supported finite models containing headers or
invocations must not lose validation because the remaining anchor is absent. Preserve required
closure or explicit validation-unit membership.

**Closure evidence:** inspect supported model/frontier construction and establish complete coverage
with exactly one execution. Retain independent malformed-header, missing-invocation and inventory
controls. Delete redundant factories, names and multiplicity-only assertions. A measurement can
quantify savings; source inspection already establishes the duplicated instrument.

**Disposition:** Scheduled, open in the [testing architecture coordinator](../../plans/testing-architecture-pivot-plan_2026-10-04.md#5-findings-and-current-checkpoint), P1.
The coordinator owns current status and closure; this review retains the source diagnosis.

<a id="F02"></a>

### F02 — Verification preparation follows broader application effects than the tested obligation

**Principles:** FP-01, FP-03, FP-06, DP-10, DP-17, DP-23 · **A1 violated**

**Owners:** `justfile`, native readiness and test harness organization

The `test *args` recipe always invokes `native-adapter-ready` (`uv sync --locked`) before Cargo
applies package/test selection (`justfile:73–78`). Pure model controls already expose bounded
contracts, such as cursor invalidation (`crates/lctx-model/tests/serving_contracts.rs:205–251`).

Package/target-scoped bare Cargo commands already permit isolated pure verification under
AGENTS.md. The defect is the standard recipe's unconditional preparation of both native adapters,
together with repeated compilation of shared fixture modules across integration targets. This
increases prerequisite and change cost for recipe users; it does not mean every scoped route
requires native or PostgreSQL setup.

Every ordinary `ServingFixture::start` starts disposable PostgreSQL, migrates, installs the full
model, prepares package input, compiles through Catalog and admits serving
(`crates/lctx/tests/fixtures/serving_support.rs:104–213`). Several read-only control families
independently use the same `SOURCE`. Large helper modules are included into separate integration
targets. These journeys provide legitimate acceptance evidence, but repeated preparation creates
pressure when a local assembly change must instantiate unrelated publication machinery, or a
shared-helper edit recompiles its copies across targets.

The coordinator's read-only Cargo metadata inventory found **223 integration targets**; a lexical
path-include inventory found `serving_support.rs` included by 13 targets. These counts describe
packaging, not measured compile cost or sufficient proof of a defect by themselves.

**Proposed correction:** organize verification around pure semantics, provider conformance,
persisted admission/lifecycle and representative integrated journeys. Provide focused pure
verification without unrelated wheel preparation; retain explicit readiness for adapter tests
that use Python `--no-sync`.

Group cohesive controls into fewer integration harness targets with internal modules and stable
test names. Share preparation for read-only assertions over the same immutable seed within a
controlled lifetime. Keep fresh or cloned state for privilege, lifecycle, cancellation and tamper
controls. A mutable global fixture is not an acceptable substitute for isolation.

A support crate is an alternative when cross-crate reuse justifies it; ordinary modules and
cohesive harness targets are the simplest initial route. No generic fixture framework is needed.

**Closure evidence:** demonstrate the pure recipe path without native/PG prerequisites, trace a
representative helper edit through the reduced target organization, preserve grouped assertions
and prove mutation cases cannot contaminate peers. Report compilation, preparation and execution
cost separately.

**Disposition:** Scheduled, open in the [testing architecture coordinator](../../plans/testing-architecture-pivot-plan_2026-10-04.md#5-findings-and-current-checkpoint), P4/P5.
File counts alone neither close this finding nor establish savings.

<a id="F03"></a>

### F03 — Physical validation preparation repeats without an explicit stable-input reuse boundary

**Principles:** FP-03, FP-06, DP-09, DP-10 · **A3 constrained**

**Owner:** `lctx-postgres::generations`

Stage admission checks stored source receipts and rehashes content
(`crates/lctx-postgres/src/generations/stage_validation.rs:162–187`). Publication repeats source
verification (`publication_validation.rs:43–89`). Each invariant separately resolves and visits
physical inputs (`publication_validation.rs:91–148`; `receipts.rs:287–315`). Earlier vocabulary
resolution also recomputes its receipt (`validation_views.rs:37–50`). Visits construct physical
SELECTs and decode streams (`generations/mod.rs:1022–1125`).

The integrity obligation is legitimate: frozen metadata cannot prove unchanged physical rows.
But adding another invariant over the same prefix adds preparation/scanning according to consumer
fanout, even where one stable view could supply several checks.

`stage_read_checks` is consumer-grant evidence, not an existing result cache: validation precedes
receipt insertion, and reader leases verify it later (`stage_validation.rs:259–319`; `lease.rs:198–204`).

**Proposed correction:** establish a genuine stable-input boundary first. Under appropriate
relation locks or a stable read snapshot, resolve/verify each exact physical relation and prefix
once, then feed compatible ordered streams to the required checks. Preserve each check's owned
semantics. Use bounded groups or separate scans where combined state exceeds the memory budget.

Transaction identity alone is insufficient: READ COMMITTED can observe different rows between
statements. Reuse must cover generation, relation, prefix, model/physical contract, contents,
ordering and relevant configuration. Cross-transaction reuse additionally needs proven
immutability or revalidation. Keep fresh integrity checks at external admission boundaries and
damaged-row controls; a previous success cannot authorize skipping current physical integrity.

**Closure evidence:** equivalent accept/refuse behavior for clean/damaged rows, incorrect receipts,
distinct prefixes and interruption; demonstrated locking/snapshot premises; same-workload scan
and decode counts, wall time and peak state. Benefits remain unmeasured.

**Disposition:** Scheduled, open in the [testing architecture coordinator](../../plans/testing-architecture-pivot-plan_2026-10-04.md#5-findings-and-current-checkpoint), P2/P3.
Its Proposed owned-immutability/audit contract deliberately reconsiders the remedy's routine
external-integrity promise; this review's original evidence and recommendation remain dated evidence.

<a id="F04"></a>

### F04 — The validation owner presents a retired oracle seam as current coverage

**Principles:** FP-06, DP-21, DP-22, DP-24 · **G7 fails for this claim**

**Owner:** `docs/design/sections/validation-and-evaluation.md`

The owner labels matched-source Pysa and CrossHair controls Implemented/Tested and names
`pysa_tito_control_uses_the_same_source_as_finite_summary_fixture` against `summary_flows`
(`validation-and-evaluation.md:84–96`). The former Pysa receipt reproduces the comparison with
`cargo test -p cpg-core --test compile …`
(`evidence/2026-09-25_pysa-tito-rule/README.md:15–42`). That test and target are absent from the
current tree. Searches across current crates, tests, scripts and evaluation paths found no current
comparison under that name. `pysa_tito_shapes` remains analyzer input in `fixture_corpus`, which
does not constitute executing Pysa's oracle.

The live Pyrefly harness explicitly checks driver/configuration parity against the same provider,
not provider correctness (`crates/cpg-extract/tests/harness.rs:1–5`, `:48–125`). These categories
must remain distinct.

**Proposed correction:** remove the obsolete current coverage claim and retired reproduction
pointers. Inspect substantive consumers, transfer surviving active semantic obligations to their
current owner, then delete obsolete evidence folders, index entries and backlinks. A former pass
or documentation backlink alone is not a current consumer. Do not retain obsolete material as
historical recovery instructions.

Reconstruct a current TITO comparison only when an active semantic obligation requires it; do not
restore retired controls or storage seams solely because they once passed. Current review/probe
evidence remains only while it serves an active finding or decision.

**Closure evidence:** current oracle claims resolve to current invocations/consumers, surviving
independent-challenge obligations have one owner, and obsolete evidence/commands/links are removed
without dangling references. No new oracle qualification is implied.

**Disposition:** Scheduled, open in the [testing architecture coordinator](../../plans/testing-architecture-pivot-plan_2026-10-04.md#5-findings-and-current-checkpoint), P0/P4.
Retention requires a substantive current consumer.

## 4. Change scenarios, alternatives and reduced demand

A **new relation/invariant** should add one authoritative meaning, specialized behavior and
meaningful controls; structural lowering follows declarations. F01 shows why instrument count
does not establish obligation count; F03 shows why stable input preparation should not follow
consumer count.

A **provider upgrade** requires adapter/conformance work. Same-provider CLI parity detects driver
mistakes; known answers and runtime observations detect different shared provider/translator
defects. Neither substitutes for the other.

A **mechanism substitution**, such as batched scans or grouped harnesses, must preserve universe,
prefix, ordering, refusal, resource limits and lifecycle. It must not introduce another semantic
validator or move domain interpretation into test setup.

**Changed source content/membership** requires addition/deletion-aware reuse. The build wrapper
already hashes sorted paths and contents of declared native inputs
(`scripts/build_environment.py:24–49`); existing controls cover unchanged timestamps, deletion
and package scope (`tests/scripts/test_build_environment.py:75–102`). Preserve that mechanism;
its tests were not rerun here. Model/native identity changes require their own architectural
assessment, not incidental weakening of invalidation for speed.

**Persisted corruption or cancelled publication** requires fresh/cloned state and actual PostgreSQL
enforcement. **Shared semantic defects** require independently established expected meaning;
deriving all expectations from production would make the suite cleaner but less capable.

**Documentation, local and dependency changes** should select work by affected contracts.
Unknown/broad changes warrant wider checks; dependency-family or toolchain changes legitimately
invalidate broad prepared artifacts. Documentation alone does not require product reconstruction.

| Alternative | Judgment |
|---|---|
| Increase timeouts/concurrency with current organization | Leaves repeated preparation/replay intact; more concurrency can worsen contention. |
| Delete broad validator/oracle categories | Weakens persisted integrity or independent evidence where obligations differ. |
| Consolidate by owned obligation and reuse bounded preparation | Preferred Proposed direction; removes demonstrated repetition while preserving guarantees. |
| Build a universal impact/result-cache framework | Defer until simpler consolidation is exhausted; introduces new dependency/completion authority. |

Existing Cargo reuse, cohesive harnesses, Nextest selection, pytest fixtures, disposable PostgreSQL
and declared inputs provide building blocks. PostgreSQL retains structural enforcement and model
validators retain relational meaning. The specialized CPython observation/comparison consumer is
justified; no established library was demonstrated to replace its whole semantic contract.
**G8 does not fail.** Compare library capabilities at the resolved version when a concrete missing
mechanism warrants investigation; a new library or framework is not a review prerequisite.

Preparation reuse is simpler than reusing “passed” results: binaries and immutable seeds can be
reused while assertions run anew. Result reuse requires complete keys, environmental premises
and explicit completion states. Cancellation, partial output, stale prerequisites and failures
must not become success. Resource/capability budgets affect admission: a large-budget pass cannot
authorize a smaller-budget request. Include budgets in the key or perform fresh owned admission
and charging. Deadlines/cancellation retain current-request semantics.

Smarter evaluation should preserve unique failure detection while reducing repeated demand:
use small equivalence classes and metamorphic/property checks for declared transformations,
targeted invalid controls for enforcement boundaries, and representative end-to-end journeys for
integration. A shared fixture or generated expectation is not independent corroboration. This
review does not justify blindly reducing Hypothesis examples or deleting slow tests by duration.

Policy already delays integrated gates until functional scope is complete. There is no duplicate
oracle invocation in `test-all`: `oracles` is a focused convenience recipe, while pytest includes
those tests (`justfile:80–87`). Repeated warm Cargo invocations also do not establish repeated
compilation. A future policy can use affected-contract checks and representative journeys for
localized scope, reserving complete qualification for assembled boundaries. That requires the
ADR/binding/AGENTS route. Selected receipts establish selected scope, not Q0 or product acceptance.
Prefer transparent selection with conservative fallback over a new check registry.

## 5. Independent judgments and evidence limits

| Judgment or gate | Verdict within this review |
|---|---|
| A1 Localize change | **Violated:** F02's standard preparation/harness boundary. |
| A2 Encode domain meaning | **Satisfied, inspected scope:** owned declarations/operations govern enforcement. |
| A3 Extend through composition | **Violated:** F01 repeats whole-stage replay; F03 constrains reuse. |
| FP-01/03/06 | **Violated** at the identified preparation/instrument boundaries. |
| FP-02/04/05 | **Satisfied** within inspected contracts; no complete compiler certification. |
| G1 Authority | **Pass, scoped static:** semantic owners identified. |
| G2 Fidelity | **Pass** for inspected assurance distinctions; complete provider/packet fidelity not assessed. |
| G3 Validity | **Pass, static implementation:** rejection paths exist; runtime qualification pending. |
| G4 Hidden behavior | **Pass** for inspected validation: checks read/reject; preparation effects explicit. |
| G5 Recovery | **Pass** for inspected lifecycle design; recovery/cancellation not rerun. |
| G6 Transformation/reuse | **Pass** for inspected current keys; proposed batching/result reuse unqualified. |
| G7 Claims | **Fail**, limited to F04's current coverage claim. |
| G8 Library leverage | **Pass, scoped:** no demonstrated generic replacement need. |
| CI-G1 / CI-G2 | **Unresolved** for complete compiler/served claims; their assurance boundaries were assessed, not all outputs qualified. |
| CI-G3 Evaluation integrity | **Pass, scoped static:** inspected reference lanes remain separate from production inputs. |

The coordinator's bounded probe **passed** on local **2026-10-04**:

```text
python3 scripts/build_environment.py -- uv run --no-sync python tests/scripts/test_semantic_soundness.py --bundle
```

Three runs took **0.659, 0.507 and 0.635 seconds**, each producing nine groups, 52 functions and
110 observation runs. Receipt: `/home/paul/.cache/lctx-assurance-review/2026-10-04/oracle-bundle-observation.json`
(UTC 2026-10-05 02:41; local review date 2026-10-04).

This **Measured** result covers bundle construction and isolated CPython observation only. It
excludes Rust production, PostgreSQL publication and serving comparisons. It supports preserving
this cheap independent challenge; subtracting it from the historical 1101s serving receipt would
not establish savings across different runs.

The normalized `cargo metadata --offline --locked --no-deps --format-version 1` inventory
**passed** without building. It establishes target organization only. Full functional, hygiene,
real-library and product qualification are **not_run** for this review. No long baseline campaign
or clean rebuild was performed; total current gate cost and achievable savings remain unmeasured.

## 6. Decision route and next work

F01–F03 concern execution/preparation; F04 concerns claims and discoverability. They interact
without requiring one redesign. Duplicate replay consolidation is direct; batched scans require
stable-input premises; harness organization can proceed independently. Correct oracle claims and
retire obsolete material without waiting for performance work. Carry surviving obligations to
current owners, then remove obsolete checks/evidence/pointers. No historical preservation duty
arises merely from a former pass.

| Priority / dependency | Responsible component and next action | Closure route |
|---|---|---|
| Truthful current coverage | Validation/evaluation owner: F04 correction and scoped retirement. | Current invocations/owners and no obsolete pointers. |
| Direct duplicate removal | Model validation owner: F01 shared obligation with finite-model coverage. | Exactly-once complete validation and retained negative controls. |
| Bounded local verification | Tooling/harness owners: F02 prerequisite selection and cohesive fixtures/targets. | Isolated route, preserved mutation isolation and measured cost by phase. |
| Stable preparation reuse | Store owner: F03 snapshot/locking premise before scan/result reuse. | Clean/damaged/prefix/cancellation equivalence plus scan/memory measurements. |

The [testing architecture coordinator](../../plans/testing-architecture-pivot-plan_2026-10-04.md#5-findings-and-current-checkpoint)
now owns scheduled F01–F04; the discussion above retains the review's original recommendations,
not a competing execution sequence. Changed validation ownership or gate policy follows ADR plus owning DESIGN/binding/
AGENTS text. Ordinary harness refactoring within preserved contracts does not require an ADR
solely for code movement.

**Revise** the target assurance architecture while preserving semantic/persistence boundaries.
The next design should execute each obligation through one owned instrument, bound local
verification prerequisites and distinguish reused preparation from reused results. Acceptance
must preserve negative/lifecycle behavior and independent evidence before claiming lower demand.
This review enables consolidation planning; it does not close Q0 or activate real-library serving.
