# Agent workspace effectiveness: capability review

**Decision: Revise. Proposed target review, 2026-10-07.**

The proposal identifies real friction and supplies useful directions: explicit synchronization,
fixture-backed bare commands, truthful selection inspection, useful logs and supported checkout
preparation. Retain those capabilities. Revise mechanisms that make worktrees ordinary policy,
eagerly prepare unrelated environments, suppress discoverable tools, or leave agents assembling
command selection and long-running job management themselves.

The target is a capable workspace for equally capable Claude and Codex agents. Commands should
expose useful operations, explain their effects and accept ordinary tool arguments. They should
support the agent's decisions about how to work. Reducing context, increasing wrapper usage or
enforcing a preferred procedure does not independently establish effectiveness.

This review independently reassesses the revised proposal and its integrated first review.
A diagnosis can remain sound while its remedy needs replacement. Recommendations below remain
**Proposed**; this review implements no harness or runtime changes.

## 1. Scope and evidence

| Field | Value |
|---|---|
| Subject | [Agent workspace effectiveness plan](../../plans/agent-workspace-effectiveness-plan_2026-10-07.md): AE-01–20, D1–7, P1–7, rule changes and rejected alternatives |
| Baseline | HEAD `43cd6501`, including integration of the [first review](design_review_agent-workspace-effectiveness_2026-10-07.md) |
| Concurrent work | Dirty `cpg-core/{artifact_manifest,facts,workspace}.rs` and `semantic-model.md` belong to paused product work; preserved and excluded from judgment |
| Standard | Core/template 3.3, heuristic companion 1.0 and [repository binding](../design_principles/binding/library-context.md), resolved through [standard.toml](../design_principles/standard.toml) |
| Tier / purpose | Design / target |
| Reviewers | Fresh independent design-reviewer; coordinator source inspection, capability research and reconciliation |
| Workload | One workstation, Claude and Codex with concurrent workers; short exploration, focused Rust/Python controls and longer native journeys |
| Scope boundary | Repository recommendations; host/user opportunities remain notes. Product semantics, test-family completeness, quantitative performance and operator-store activation are excluded |

The code-intelligence profile 1.5 was consulted. Its production graph, attribution and serving
gates do not apply to these workspace commands. Core fidelity and evidence rules govern selection,
preparation, reuse and results. The relevant existing architectural owner is
[DESIGN §1](../../design/DESIGN.md#section-1), including the development and verification workflow;
existing rules describe the design being assessed, rather than restricting its alternatives.

Inspection covered the proposal, first review, [evidence README](../evidence/2026-10-07_agent-workspace-effectiveness/README.md),
recipes, environment/verification/fixture scripts, runtime configuration and selected consumers.
Transcript counts and drills remain dated historical evidence, not a new effectiveness study.
AE-16's memory contents and AE-18's skill-scan behavior were not independently refreshed.

**Interface-checked, 2026-10-07:** source, installed tool help and selected upstream contracts.
Functional tests, builds, synchronization, fixture launches, runtime configuration changes and
performance measurements are **not_run** for this review. Future acceptance examples are not
executed tests. **Documentation passed, 2026-10-07:** `UV_NO_SYNC=1 just docs-check`
(336 canonical pages, zero link errors) and `git diff --check`. `just turn-end` is **not_run**:
its whole-tree formatter would rewrite paused concurrent edits. No formatting or generated-code
repair is folded into this review.

## 2. Capability boundaries and composition

| Owner | Responsibility |
|---|---|
| Command selection | Resolve selected boundaries, ordinary arguments, prerequisites and effective defaults once; inspection and execution consume that result |
| Environment preparation | Explicitly prepare a project and requirement; inspect readiness without installing or rebuilding |
| Fixture management | Own server substrate, attachment-local mutable state, optional retained serving content and cleanup |
| Run management | Preserve logs/results and expose status and cancellation across tool calls, using runtime facilities where sufficient |
| Checkout preparation | Prepare any checkout; optionally create a worktree and transfer selected work |
| Discovery/navigation | Explain available commands, source-navigation operations, prerequisites and fallbacks |
| Instructions/configuration | Supply durable context and useful capabilities without competing workflow mandates |

Ordinary functions, small records and existing recipes can supply these boundaries. No workflow
engine, daemon, capability registry or universal workspace model is justified.

The distinctions that matter are concrete: a server is not its initialized content; an installed
environment is not necessarily a matching loaded extension; executed work differs from reused
setup; cancellation differs from a code failure; parallel execution differs from conflicting
mutation of a checkout.

## 3. Findings and concrete proposal changes

### F01 — Transparent commands still lack parameterized composition

**FP-01/03/06; DP-06/08/24; A1/A3.**

D4 adds valuable inspection modes but leaves selection fragmented.
[verify.py](../../../scripts/verify.py) defines families, boundary names and requirements;
`native_controls.py` (since folded into `verify.py`) supplied package defaults and MCP setup.
The current parser accepts one family and one optional boundary; MCP arguments reach pytest
after the Rust journey. An agent selecting compiler and MCP controls still assembles separate
invocations, filter destinations and outcomes. Printing the procedure does not provide composition.

**Replace D4's opening contract:**

> Resolve an invocation into one command plan containing selected boundaries, arguments,
> packages/targets, prerequisites and effective defaults. Print, list, execution and results
> consume it. Support repeated boundary selection and tool- or boundary-scoped arguments.
> Families remain named shortcuts; arbitrary underlying arguments remain available.

For example, a **Proposed** entry point could express:
`just verify --select compiler:producer --select serving:mcp --nextest-filter 'test(admit)' --pytest-filter native_session --print`.
Convenience filters must not restrict the underlying tools' supported arguments. Derive help and
boundary descriptions from executable definitions; do not maintain a second catalogue.

Keep static command preview distinct from actual discovery. Nextest already supplies machine-readable
lists and existing-build reuse; use those facilities rather than inventing a binary cache (§6).
A mixed invocation needs the appropriate discovery operation for each tool, not a universal
nextest-only list. Coverage completeness remains with the testing owner.

**Acceptance:** select Rust and Python boundaries together with different filters; each filter
reaches its owner. Invalid targets name the actual package set. A failed boundary preserves the
other boundary's outcome. Adding a boundary updates its definition and genuinely new behavior,
without independent help/preview/result classifiers.

### F02 — Logs and a status file do not supply a usable long-run handle

**FP-02/05/06; DP-19/21/24; A2/A3; G5.**

AE-10 diagnoses scattered logs and polling. D4/P4 provide files but no operation for finding,
observing, cancelling or recovering an identified run. Current launchers use synchronous
`subprocess.run`. A directory alone leaves process ownership and interruption behavior to callers.

**Add to P4:**

> Preserve foreground execution. Support an optional run handle that later commands can use
> for status, logs and cancellation. Record phase/current command, waiting reason, process
> identity and terminal result. Cancellation affects the owned command tree and its run-owned
> fixtures. Show the failed or unexecuted command and its prerequisites for a deliberate rerun.

First use each runtime's existing background execution, wait and cancellation facilities when
they satisfy the task. Associate its handle with the repository run directory. Add a small
repository launcher only for the unmet need to recover runs across commands or sessions; no new
scheduler or always-running service is required. The same capability should accept arbitrary
commands rather than being confined to verification.

Preserve child exit/signal information. Keep boundary outcomes `passed`, `failed`, `blocked`
and `not_run`, with interruption as a separate termination reason. Infrastructure classification
needs evidence; an unexplained nonzero exit is not automatically an environment failure.
Pruning selects completed runs and preserves active or explicitly retained runs.

**Acceptance:** start a long run, observe it from a later tool call, cancel it and retrieve its
terminal result while another run survives. A killed launcher leaves a discoverable interrupted
run rather than permanently reporting “running.” A rerun uses the original selection without
silently replaying successful work.

### F03 — Preparation is insufficiently scoped and bare-command reliability is overstated

**FP-01/02/06/07; DP-09/10/18; A1/A4; G4/G6/G7.**

D1's explicit-sync direction is sound, but:

- Full root sync is the proposed repair even though tools-only preparation already exists.
- D3's prepare calls sync and ready, while D1 says ready calls sync again.
- Blanket non-syncing launch includes the separately locked service in
  [embed_serve.py](../../../scripts/embed_serve.py); root [pyproject.toml](../../../pyproject.toml)
  explicitly excludes that environment. Root sync cannot repair it.
- RC01's “bare Cargo needs nothing” does not remove inherited target paths handled by
  [build_environment.py](../../../scripts/build_environment.py).
- The justfile shell computes native input fingerprints for every recipe, including unrelated
  documentation and inspection.

**Replace “one synchronizer” with “one explicit preparation route scoped by project and requirement”:**

> Prepare only the requested environment and capability. Tools-only preparation does not build
> the extension. Native preparation establishes matching inputs once. Independently locked
> services retain their own explicit preparation route. Ready composes those operations without
> repeating synchronization. Every blocked message identifies the applicable repair.

Keep non-syncing repository launches where that preparation route exists. Retain an optional thin
environment launcher accepting arbitrary commands and reporting effective paths. Bare tools remain
available; describe their inherited configuration honestly. Compute native fingerprints only
where native synchronization or freshness observation needs them.

Correct P2's dependency detail: make the declared maturin build requirement exact, not merely
constrained during resolution while retaining `maturin>=1.15.0`. This is ordinary dependency
maintenance, not a new approval or ADR requirement.

**Acceptance:** tools-only preparation works without building the extension; native preparation
is reused when unchanged; a missing service environment names its own repair; static preview
does not fingerprint native sources or create fixtures. No live vLLM launch is needed to test
command construction and project selection.

### F04 — Environment ownership needs an honest managed boundary

**FP-02/05; DP-19/20; A2; G5.**

Shared/exclusive locking improves D1, but bare importers and explicit syncing overrides do not
participate automatically. Conversely, a pure Rust command should not borrow the Python
environment for its entire lifetime merely because it uses a fixture. Existing
`BOUNDARY_REQUIREMENTS` already distinguishes these effects.

**Clarify D1:**

> Managed commands hold shared ownership while they use a shared Python environment. Managed
> synchronization takes exclusive ownership and reports contention. Pure commands do not borrow
> that environment. Acquire ownership once per command tree; nested managed operations reuse it.
> Unmanaged commands remain available outside this coordination guarantee.

Place coordination against the effective environment's stable identity. A hard-coded main
`.venv` lock cannot protect an explicitly selected alternate environment. Keep setup usable
before the environment exists, and avoid replacing the lock's identity during setup.

Version-specific source settles part of P2 step 0: maturin 1.15.0 attempts to unlink the old
editable extension, ignores unlink errors, then copies its replacement (§6). Successful unlink
preserves an existing mapped inode, but publication is not atomic for new imports. It supports coordination
of cooperating readers and writers, not a blanket refusal or a claim that bare consumers are protected.
Package freshness on disk also does not identify the extension already loaded by a persistent
process; optional `build_identity()` serves that distinct diagnostic purpose.

**Acceptance:** pure Rust fixture work does not hold the Python environment lock for the test
lifetime; managed native consumers do. Two readers overlap; a writer reports its wait; nested
fixture execution does not deadlock. An intentional override names the guarantee it bypasses.

### F05 — Kept substrate, attachment state and reusable serving content are conflated

**FP-02/04/05; DP-09/19; A2/A3; G3/G5/G6.**

D2's lifetimes are useful. Fresh launcher-supplied cache and selection paths do not isolate every
command: [native_journey.rs](../../../crates/lctx-serving/tests/native_journey.rs) chooses its
own namespace/cache/viewer names, and [cache.rs](../../../crates/lctx-surrealdb/src/cache.rs)
contains test-owned database choices. Retaining a server also differs from retaining the published
content that P4's MCP reuse needs.

**Specify separately:**

> A kept fixture retains server substrate. An attachment receives owned scratch/configuration
> and identified mutable databases or namespaces. Arbitrary commands choosing their own names
> remain responsible for that state; concurrent attachment does not promise universal isolation.
>
> Reusable serving content additionally identifies its producing inputs, configuration, immutable
> content and producing run. New attachment state preserves that content. Reuse records the
> skipped obligations and checks that the expected retained realization remains available.

A source fingerprint alone cannot establish that server state survived. Reuse the existing
realization's identity and applicable readiness semantics; do not introduce a separate proof
system or repeatedly rebuild the full journey to establish a cheaper prerequisite.

Add fixture inventory/status. Orphan ownership must distinguish process lifetimes, including
PID reuse or reboot, and account for surviving owned children. A launcher killed by SIGKILL can
leave its command alive. Sweep must preserve live consumers or explicitly terminate its owned
surviving command tree before removing the server. Advertise automatic sweeping and make it
overridable; inspection never sweeps.
Keep the prior stable-port/restart correction and actual OOM diagnostics.

**Acceptance:** two attachments have distinct mutable state while using retained immutable
content; stopping one preserves the other. A killed launcher with a surviving child is not
misclassified as disposable idle state. Missing or altered retained content is reported despite
a matching source fingerprint.

### F06 — Worktrees become ordinary policy before their need is established

**FP-03/06/07; DP-16/20; A3/A4; G5/G6.**

AE-13 supports an available isolation capability. It does not establish that every parallel
build/test needs a separate source tree and environment. RC03 nevertheless makes that the ordinary
route, potentially adding preparation and compilation when commands share one stable revision.

**Replace RC03/D3's policy:**

> Concurrent commands may share a stable checkout when their effects do not conflict. Worktrees
> support independent revisions or conflicting mutable state. Preparation works in any checkout,
> including runtime-created worktrees. Shared and per-checkout build directories are selectable,
> with their effective paths shown.

There is also a concrete missing override case: uv 0.12.22 uses an absolute
`UV_PROJECT_ENVIRONMENT` as-is, and the current normalizer leaves it unchanged (§6).
A session carrying main's absolute environment can therefore prepare main even from a worktree.
Checkout preparation must select its intended environment explicitly, expose deliberate alternate
selection, and coordinate that selected environment. Checking only `VIRTUAL_ENV` is insufficient.

Keep create/remove helpers optional. Define `--carry` as an explicit copy: selected tracked,
staged/unstaged, untracked and binary content, with staging treatment and conflicts reported.
Preserve the source checkout and exclude environment/credential material. Removal reports dirty
or unintegrated work rather than treating it as disposable.

P5's artifact-build probe can inform defaults; it cannot establish a universal worktree requirement.
Do not promise warm workspace/link reuse simply because sccache remains enabled (§6).

**Acceptance:** disjoint checks share one stable checkout; an independent worktree is prepared
from a main-started session carrying main's absolute `UV_PROJECT_ENVIRONMENT`; import origin
and effective lock follow the selected checkout. Explicit carry preserves mixed-content work and
the source checkout; integration and removal act only on the chosen worktree.

### F07 — Blanket runtime suppression removes capability and misstates restoration

**FP-02/06; DP-15/24; A3; G7.**

D6 suppresses broad plugin categories and all Claude AI connectors. Evidence supports a specific
competing workflow injection and distracting unauthenticated connectors, not removing every
potentially useful capability.

The restoration promise is also wrong as a general contract. The installed Claude 2.1.293 embedded
settings schema marks `disableClaudeAiConnectors` as any-source-true. Ordinary higher-priority
false settings cannot be promised to undo a true setting elsewhere. Current official documentation
confirms those semantics (§6); no override test was run.

**Replace D6:**

> Remove specifically identified competing workflow injection. Preserve discoverability of useful
> tools and skills. Explain unavailable or unauthenticated capabilities. Prefer selective
> configuration and deferred discovery where supported. Any suppression names the capability
> removed and its version-qualified restoration route.

Retain useful descriptions, navigation and task routes. Equal runtime capability does not require
equal context size. Reducing first-request tokens alone is not acceptance.

**Acceptance:** a fresh session discovers and uses a relevant capability whose full description
was not initially loaded. The selected competing workflow remains absent. A documented restoration
route works at the installed version. A failed connector does not conceal working tools.

### F08 — Discovery, freshness and maintenance need narrower, usable contracts

**FP-01/06; DP-18/21/22/24; A1; G4/G7.**

P6's checks cannot establish that every generated output is fresh.
[check_gold.py](../../../scripts/check_gold.py) can report `not_run` with exit zero when its skill
is missing; snapshot hints remain heuristic. Describe freshness of enumerated outputs, support
selection, preserve those distinctions and name outputs requiring tests. Report each output's
regeneration command without silently invoking it.

Extend this with **scoped maintenance**. The current [justfile](../../../justfile) runs global
formatting and generators at turn end, even when a documentation task shares a dirty production
tree. Offer selected formatting/repair and applicable generator operations, preserving explicit
whole-tree operation. The root should not need to choose between touching another agent's work
and leaving its own maintenance incomplete. This is a capability change, not another lint.

Make P7 navigation actionable: definition, references, hover/document symbols, tool discovery
and workspace selection; use workspace-symbol search where actually exposed. Supply file/symbol
search fallbacks. Current-session MCP metadata exposes Rust definition/reference/hover/document
symbol operations, but that alone does not prove a warm or correctly bound workspace. A short
route in existing documentation is sufficient; no new index or standing navigation procedure.

Keep precise P1 cleanup. “2.8 GB of logs” is not a deletion selector; preserve useful captures.
P7's memory changes belong in user-level notes pending an explicit request, not repository
completion. Keep D5's durable routes and deliberate overlap where isolated workers need it.

**Acceptance:** freshness distinguishes clean, stale, heuristic and not-run results; selected
maintenance leaves unrelated dirty files byte-identical. Navigation locates a moved module using
available tools or the stated fallback. Cleanup identifies exact candidates.

## 4. Keep, correct, remove and add

| Packet | Recommended change |
|---|---|
| P1 | Keep confirmed catalog retirement and precise stale cleanup; remove bulk log deletion as an implied action; scope memory separately |
| P2 | Correct to project/requirement-scoped preparation, managed ownership and exact build dependency; remove duplicate sync, unconditional native hashing and blanket reliability claims |
| P3 | Keep one fixture boundary and stable-port/OOM fixes; distinguish substrate, attachment state and serving content; add inspectable inventory and sound orphan identity |
| P4 | Keep preview/discovery distinction and boundary outcomes; add composable selection, usable run handles, recoverable results and unique per-run reports |
| P5 | Keep optional checkout helpers; remove prescribed isolation for ordinary parallel commands; correct inherited environment and carry/removal contracts |
| P6 | Keep enumerated read-only checks; add selection and scoped maintenance; preserve heuristic/not-run limits |
| P7 | Keep current instructions and useful overlap; replace blanket suppression with selective capability discovery; add actionable navigation and restoration examples |

Priorities follow consequence: prevent wrong-environment mutation, misleading results and
destructive lifecycle mistakes; then remove repeated preparation and command assembly; then improve
discovery and precise cleanup. Dependency order differs: preview, help and logs need not wait for
all of P2, and pure fixture lifecycle work need not depend on Python synchronization. Retained
serving reuse does depend on identifying its initialized state.

Reconsider a native versus Docker fixture on pinned provenance, lifecycle and total integration
burden; the current image choice is not an architectural prohibition on alternatives. Retain the
rejection of a repository build lock, a non-bypassable import gate and a universal affected-tests
predictor. None is needed to deliver these capabilities. Module-layout normalization and a replacement
skill scanner remain unjustified by the inspected evidence.

## 5. Independent reassessment of the first review

The [first review's](design_review_agent-workspace-effectiveness_2026-10-07.md) IDs remain its own;
the F numbers above identify new follow-up findings.

| First review | Assessment of the integrated remedy |
|---|---|
| [F01](design_review_agent-workspace-effectiveness_2026-10-07.md#f01) | Retain lifetime/port correction; attachment state and orphan ownership need this review F05 |
| [F02](design_review_agent-workspace-effectiveness_2026-10-07.md#f02) | Retain shared/exclusive distinction; scope and managed reach need F04 |
| [F03](design_review_agent-workspace-effectiveness_2026-10-07.md#f03) | Explicit flags/runtime reach improved; independent preparation projects need F03 |
| [F04](design_review_agent-workspace-effectiveness_2026-10-07.md#f04) | Artifact claim appropriately narrowed; worktree policy and environment transfer need F06 |
| [F05](design_review_agent-workspace-effectiveness_2026-10-07.md#f05) | Preview/discovery and skipped-journey reporting retained; composition/lifecycle need F01/F02/F05 |
| [F06](design_review_agent-workspace-effectiveness_2026-10-07.md#f06) | Scoped observation and overridable identity retained; preparation/ownership need F03/F04 |
| [F07](design_review_agent-workspace-effectiveness_2026-10-07.md#f07) | Revised attribution/labels retained; historical counts do not select a remedy |
| [F08](design_review_agent-workspace-effectiveness_2026-10-07.md#f08) | Interpreter constraint and heuristic label retained; maintenance/result scope need F08 |

AE-01–07 remain useful environment/fixture diagnoses. AE-08–10 need actual composition and run
operations. AE-11 needs bounded freshness; AE-12 remains with the testing owner. AE-13 supports
optional isolation. AE-14–17 support useful current context and discovery. AE-18 does not justify
a new scanner. AE-19/20 support precise retirement. No historical diagnosis is relabelled as a
freshly measured improvement.

## 6. Library fit and remaining uncertainty

The coordinator and library researcher used Context7, installed help and source. These are
**Interface-checked, 2026-10-07**, not end-to-end acceptance:

| Capability | Evidence and consequence |
|---|---|
| Editable extension replacement | [maturin v1.15.0 binding generator](https://github.com/PyO3/maturin/blob/v1.15.0/src/binding_generator/mod.rs), `add_artifact`: attempt unlink, ignore its error, then copy into the source tree. P2's source-path question is settled; atomic publication/concurrent execution is not established. Existing version-labelled cached source was inspected, without independent binary/source attestation |
| Effective environment | [uv 0.12.22 project configuration](https://github.com/astral-sh/uv/blob/0.12.22/docs/concepts/projects/config.md), Project environment path: absolute `UV_PROJECT_ENVIRONMENT` is used unchanged; `VIRTUAL_ENV` normally is not used by project operations unless `--active` is selected |
| Selection and existing builds | Installed nextest 0.9.146 `list --help` and `run --help` expose JSON/binary listing, metadata/archive reuse and remapping; [listing](https://nexte.st/docs/machine-readable/list/) and [archives](https://nexte.st/docs/ci-features/archiving/) support using these mechanisms. Existing-build reuse must state freshness responsibility |
| Per-test reports | [Nextest JUnit](https://nexte.st/docs/machine-readable/junit/) uses a profile output path and omits skipped tests by default. Concurrent runs need unique destinations; wrapper boundary outcomes cannot be inferred solely from missing JUnit entries |
| Cargo reuse | Workspace profiles are incremental; [sccache Rust caveats](https://github.com/mozilla/sccache/blob/v0.17.0/docs/Rust.md) and cached 0.17.0 source exclude incremental compilation from caching. Sccache does not establish warm workspace or linker reuse. Shared-build concurrency remains unmeasured |
| Tool discovery/restoration | [Claude MCP documentation](https://code.claude.com/docs/en/mcp#scale-with-mcp-tool-search) describes deferred tool discovery; its [connector settings](https://code.claude.com/docs/en/mcp#disable-claude-ai-connectors) describe any-source-true disablement. Installed 2.1.293 schema confirms that restriction. Current docs differ from that schema on explicitly supplied proxy connectors, so no bypass is promised |

These findings refine the first review: when unlink succeeds, maturin does not overwrite the old mapped file,
and retaining sccache does not prove either universal cold builds or universal warm reuse.
No comparison warrants a tool upgrade in this review.

The simplest viable alternative is a set of composable operations over existing uv, Cargo,
nextest, Git, runtime process tools and the fixture owner. It offers selected preparation and
inspection directly, then adds only missing lifecycle/selection behavior. This avoids both the
current hand-assembled workarounds and a new mandatory workflow layer.

Remaining settling checks concern actual build-directory contention, runtime discovery/restoration,
and managed replacement/lifecycle behavior. Each belongs to the proposed changed capability's
focused acceptance, not a new benchmark campaign or prerequisite to unrelated improvements.

## 7. Architectural judgment and rule impacts

| Judgment/gate | Verdict and basis |
|---|---|
| A1 Localize change | Unresolved: command selection and project preparation need coherent owners |
| A2 Encode domain meaning | Violated: managed ownership, attachment state and interrupted work remain incomplete |
| A3 Extend through composition | Violated: multi-boundary composition is absent; prescribed isolation and blanket suppression narrow choices |
| A4 Fit execution to workload | Unresolved: repeated/eager setup and unnecessary checkout preparation remain |
| G1 Authority | Unresolved: inspection/execution do not yet share a complete resolved command definition |
| G2 Fidelity | Unresolved: selection, interruption and reused state need the distinctions above |
| G3 Validity | Unresolved: retained-state validity and attachment boundaries |
| G4 Hidden behavior | Unresolved: preparation, inspection and maintenance effects |
| G5 Consistency/recovery | Fail: owner-death sweep can conflict with surviving work; run recovery is incomplete |
| G6 Reuse | Unresolved: effective environment and retained serving state |
| G7 Claims | Fail: restoration and bare-command reliability promises exceed their mechanisms |
| G8 Library leverage | Pass, scoped to inspected alternatives: existing tools provide credible mechanisms without another framework |
| CI-G1–3 | Not applicable: no production code-intelligence semantics are changed |

Rule impacts below are **Proposed**, not applied changes:

| ID | Rule/location | Recommended change and dependent findings | If the rule is retained |
|---|---|---|---|
| RC01 | Plan RC01; AGENTS environment route | Accurate inheritance and optional normalized execution; F03/F06 | Remove the unqualified bare-Cargo reliability claim |
| RC02 | Plan RC03; AGENTS worktree rule | Isolation according to conflicting effects/revisions; F06 | P5 remains an available capability with narrower usage |
| RC03 | Plan RC04/RC07; testing/environment instructions | Managed ownership and project/requirement-scoped preparation; F03/F04 | Narrow guarantees and name preparation costs |
| RC04 | D6/P7 runtime settings | Selective capability-preserving configuration; F07 | Remove the easy restoration/equal-capability claims |
| RC05 | P7 memory step | Move memory changes to user-level notes pending explicit request; F08 | Repository completion excludes memory edits |
| RC06 | AGENTS and DESIGN §1 whole-tree turn-end rule | Allow applicable scoped maintenance without rewriting unrelated dirty work; F08 | Document the unperformed whole-tree step when preservation prevents it |

Changes to architectural workflow owners follow their existing decision route if selected;
this review does not amend accepted ADRs. D5's durable-instruction direction and D7's confirmed
catalog retirement remain appropriate.

**Disposition:** F01–F08 and RC01–RC06 are unscheduled recommendations owned by this review pending
proposal revision. Revisit them when revising the corresponding packets, before implementing
their affected capability. If selected for execution, transfer current dispositions to the
existing plan's §9 and link back here; do not create another execution ledger.

**Bounded decision: Revise at Proposed maturity.** Keep the enabling directions, correct the
mechanisms and add the missing composable operations. Harness implementation, complete
test-family coverage, product qualification and measured effectiveness are not established.
The next action is to revise the existing proposal around these capability contracts and its
revealing acceptance cases.
