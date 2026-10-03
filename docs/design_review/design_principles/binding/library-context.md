# library-context binding

The repository layer of the design standard, and the successor to `ADDENDUM.md`. It defines no
principles; it maps the [core design principles](../core/design-principles.md) and the
[code-intelligence profile](../profiles/code-intelligence/principles.md) onto this repository's
binding decisions, review cadence, vocabularies and code. `docs/design/DESIGN.md` plus `docs/design/sections/` is the architectural collection; DESIGN §2 delegates review mechanics to this page under
ADR-0040. Other design contracts and accepted decisions govern their domains; report a conflict and its resolution route.

## Standard applied

| Layer | Document | Version |
|---|---|---|
| Core | [design principles](../core/design-principles.md), [review template](../core/design-review-template.md) | 3.2 (repository-owned edition, ADR-0040/0093) |
| Profile | [code-intelligence principles](../profiles/code-intelligence/principles.md), [review additions](../profiles/code-intelligence/review.md) | 1.3 |
| Binding | this page | — |

The version/path declaration in [`standard.toml`](../standard.toml) is authoritative for loading.
ADR-0040 establishes repository ownership; it replaces ADR-0023's unlocated shared-copy
requirement. ADR-0093 adopts core/template 3.2 and profile guidance 1.3. Other repositories are
not changed by a local revision. Preserve FP/DP/CI IDs and historical version semantics;
FP-01–FP-06 and A1–A3 supply the architecture structure. FP-04/A2 assess whether an adequate,
explicit domain model governs behavior, independently of other architectural and fidelity judgments.
This assessment belongs to the bounded reviews below. Ordinary implementation work does not
itself trigger a domain-model review; investigation depth and optional flow tracing follow the
concrete question being assessed.

## Reviews in this repository

- **Location:** `docs/design_review/reviews/design_review_{slug}_{YYYY-MM-DD}.md`.
- **Evidence method:** static review can suffice, including for library features and fit.
  The reviewing agent judges whether complexity, criticality or unresolved uncertainty warrants
  creating and running probes; no review tier or kind requires them. When chosen, store probes
  under `docs/design_review/evidence/YYYY-MM-DD_<topic>/` with a README. This storage convention
  does not require an evidence folder for every review. Implementation acceptance checks and
  the evidence needed for Tested/Measured claims still apply.
- **Who:** the `design-review` skill with `design-review-code-intelligence`, usually through a
  fresh `design-reviewer` subagent, so the author of a change is not its reviewer.
- **Cadence and purpose** (ADR-0040):
  - Before a substantial stage, changed ownership/dependency boundary or §B decision: a
    **design/target** review, including adjacent consumers and realistic change scenarios.
  - For a bounded slice inside accepted boundaries: **change/conformance**, with the affected
    contract, one relevant extension scenario and its enclosing architectural limit. Pure
    mechanical changes may give an explicit reason the scenario is unchanged.
  - At an integrated stage or increment boundary: **design/target** review of the assembled
    scope and accumulated findings; compare the original scenarios with the resulting code.
    A coincident stage/increment end needs one review. Depth follows impact and uncertainty.
  - Reopen architectural analysis sooner when repeated classifiers, private dependencies,
    inseparable test setup or repeated structural deferrals affect planned work. These are
    reasoning triggers, not new blocking hooks.
  - `compact`, `standard`, `deep` in historical records describe effort. The odd/even increment
    schedule is retired. A local pass never certifies the enclosing architecture.
- **Authority:** the architectural collection is authoritative, so a divergence from an implemented claim is a code defect
  or a stale DESIGN section; distinguish both from an accepted target still awaiting implementation.
- **Outcomes and labels:** check outcomes are `passed`/`failed`/`blocked`/`not_run` with the
  command (AGENTS.md *Reporting*); historical test receipts keep their original date and scope
  and are not fresh runs; a mocked provider is never a pass; outcomes are never turned into a percentage.
- **Reviews are evidence, never authority.** A governed architectural section changes through the ADR that
  responds to a finding.

**Documentation boundary (ADR-0041).** Read the architecture map, relevant section owner and
adjacent consumers. Stable section IDs survive relocation; one resolver supplies ADR lint and the
site directory. Update prose for enduring meaning, boundaries and workflow changes. Derived
navigation/search and link checks serve readers; they do not certify implementation or historical
claims. Internal changes require no additional evidence packet. Current-work selection links to
existing disposition owners and never copies their status. Documentation tooling uses focused
`just docs-test`, with `docs-check` in `just hygiene`, independently of integrated product
qualification.

## 1. Binding decisions and what they bear on

IDs are never reused or renumbered. Changing a §B decision needs an ADR and a design/target review.

| ID | Binding decision (DESIGN.md §2 is authoritative) | Bears on |
|---|---|---|
| §B1 | Independent latest Ruff owns canonical syntax/lexical facts; Pyrefly owns typing within its embedded Ruff family, and ty owns the declared runtime flow view. Provider-local parsing and exact source/role attachment preserve nominal boundaries (ADR-0117) | DP-01, DP-08, DP-14, DP-16, CI-02 · G1, G7 |
| §B2 | `lctx-model` typed domain declarations own facts and derive Arrow/PostgreSQL contracts; codebooks are append-only (ADR-0085–0088); the Phase 5 legacy schema/IPC consumers are retired | DP-01, DP-02, DP-03, DP-16, DP-24 · G1, G2 |
| §B3 | Shared model invariant validators serve tests and publication; DataFusion is in-process compute; normalized derivations return in P3 | DP-03, DP-08, DP-18, DP-23 · G3 |
| §B4 | Graph algorithms have named owners and named consumers | DP-07, DP-11, DP-13, DP-21, CI-05, CI-07, CI-09 · G6, G8 |
| §B5 | Python semantics are custom Rust passes with stated abstractions | DP-05, DP-08, DP-11, DP-22, CI-06 · G2, G7, CI-G1 |
| §B6 | Facts are first-class assertions with run, origin, fidelity and model; disagreement is retained; analysis results carry provenance in-row | DP-01, DP-02, DP-05, DP-21, CI-01 · G1, G2, CI-G1 |
| §B7 | Immutable PostgreSQL generations, attempt-owned seal/validate/publish with admission and pinned reader leases; publication and selection are distinct | DP-04, DP-19 · G5 |
| §B8 | Independent Ruff/ty and Pyrefly link in-process at exact fork revisions, with observational seams and explicitly constructed configurations (ADR-0117/0118) | DP-09, DP-14, DP-15, DP-18, DP-21, CI-10 · G2, G4, G6 |
| §B9 | One pinned compute family and two exact, source-scoped analyzer families (ADR-0118) | DP-09, DP-15, DP-21 · G6, G7 |
| §B10 | The standing exclusions | DP-16 · — |
| §B11 | Insight synthesis is programmatic; no generative model in v1, and never in the query path | DP-02, DP-08, DP-11, DP-22, CI-11 · G2, G7, CI-G2 |
| §B12 | One PostgreSQL relational store; facts generations are implemented, serving views/indexes return in P5 without a second semantic store | DP-01, DP-19, CI-13 · G1, G5 |
| §B13 | The agent interface is a FastMCP server over one pinned generation per process; accepted PG serving uses a Rust repository and pure native executor | DP-10, DP-14, DP-15, CI-13 · G2, G3, G7 |
| §B14 | One hashed embedding spec, cached vectors, conformance-checked Rust and Python clients | DP-09, DP-11, DP-21, CI-13 · G6 |

## 2. Recurring review questions, routed onto the gates

Use these domain questions after reconstructing responsibilities and expected changes. Their
gates constrain supported behavior; A1–A3 independently determine architectural fitness. A
concrete change-propagation or testability defect can require revision without a failed G gate.

| # | Question | Gate | Principles |
|---|---|---|---|
| 1 | Can every fact, finding and assertion be traced to its provider, analyzer revision, run, environment and source span? | G1, CI-G1 | CI-01, DP-21 |
| 2 | Is any semantic fact editable in two places, such as a schema and a hand-written validator, a codebook and a match arm, a family table and a derived view, or the canonical store and the serving bundle? | G1 | DP-01 |
| 3 | Does every brief assertion cite only findings and evidence that exist in the **same** generation, and does every named public symbol and parameter exist there? | CI-G2 | CI-11 |
| 4 | Are unavailable, not-requested, failed and unresolved distinguishable, and does an empty result or an `unresolved` slot stay distinct from "absent"? | CI-G1 | CI-04, DP-02 |
| 5 | Is Pyrefly's inference graph relabelled as runtime dataflow, an `ifCalled` target as a call, or a call edge as "always reached" or "recommended"? | CI-G1 | CI-02 |
| 6 | Can `statistically_derived` output (communities, centrality, kNN) state a control, a limit or a behavioral claim in a brief? | CI-G1 | CI-09 |
| 7 | Can an unvalidated batch, or one with unmapped endpoints, reach generation publication? | G3, G5 | DP-03, DP-19 |
| 8 | Can a reader load a table at "latest" or without a generation lease, or mistake an aborted attempt for a published one? | G5 | DP-19 |
| 9 | Does the serving generation read its canonical generation, and does a server process hold exactly one generation for its lifetime? | G5 | CI-13 |
| 10 | Does a projection, a parallel-arc collapse or a dense index lose the facts that support it, or leak into persistent identity? | G6 | CI-03, CI-05 |
| 11 | Do traversal, community detection and concept analysis give identical output for shuffled input, with seeds, parameters and crate versions recorded? | G6 | CI-09, DP-11 |
| 12 | Can a query-time vector and a compile-time vector come from different embedding specs, or a cached vector be reused after the spec changed? | G6 | CI-13, DP-09 |
| 13 | Does an extractor, decoder or DESIGN claim cover a fact family it only partly extracts? | G7 | DP-15, CI-04 |
| 14 | Does a read, validation or inspection path mutate, fetch, build, or read ambient state, including analyzer config discovery? | G4 | DP-18, CI-10 |
| 15 | Can anything under `.claude/skills/` (the gold reference) reach the compiler's inputs, or can analytics parameters be tuned on the gold? | CI-G3 | CI-12 |
| 16 | Which phenomena and owned operations govern this feature, and do consumers reuse their meaning? Distinguish instances, bindings, compositions, policies, concepts and mechanisms when assessing change. | A1, A2, A3 | FP-01–06, DP-08, DP-16, core §E |
| 17 | Does each added layer or technique have a named consumer in the served model (a tool's output or a brief), and does its ablation change published output (§9.8)? | — | DP-16 |

## 3. Vocabulary and scenario authorities

| Meaning | Authority |
|---|---|
| Design claim strength | Core principles §D; label and date the claim actually established |
| Architectural judgments | Core §E, A1–A3; report separately from gate verdicts |
| Correctness/fidelity gate verdicts | Core §A and profile gates |
| Check execution outcomes | AGENTS.md Reporting; each outcome names its command |
| Domain fidelity, verdicts and boundary reasons | `lctx-model::domain` and DESIGN §15 (ADR-0085–0088); §3/§9 preserve typed provider and finite analysis obligations |

Select scenarios from the active plan. The recurring architectural questions are:

| Scenario | Expected boundary |
|---|---|
| Add a pinned model using existing categories | One model declaration and genuinely new domain behavior; existing validation/persistence/serving contracts follow |
| Add an analytic over existing facts | Declared input/output and invocation contract; no new extraction or publication rule |
| Upgrade an analyzer | Provider integration absorbs private API changes; semantic changes have an explicit contract decision |
| Add a rendering | Consume canonical results/evidence without another semantic interpreter |
| Change an invariant | Update its authoritative definition and mechanical derivations; inspect independent enforcement |
| Test a transformation | Explicit inputs and observable outputs without unrelated acquisition/store/server setup |

These are scenario seeds, not promises about current implementation. The review names its chosen
scenario, expected propagation, observed or proposed route, and evidence strength. Shared Arrow
contracts are intentional under §B2; a wrapper needs a concrete semantic or variation benefit.

## 4. Where findings land

The source review owns the dated finding and its evidence, identified by `review#Fnn`. Current
execution disposition has **one owner**: the active plan's findings table when scheduled, otherwise
an explicit Deferred row in the source review. On transfer to a plan, link the review to that owner;
retain the original assessment. Subsequent reviews and STATUS link to the owner instead of copying
mutable status. Related findings with one structural cause share a remediation row and keep every
source reference. No separate register is introduced.

The plan row records source findings, disposition, responsible component, decision/implementation
links, and closure evidence or revisit trigger. Dispositions are open, in progress, deferred,
closed or superseded. An accepted ADR is a decision, not implementation or closure. Close a finding
only when the named evidence establishes its correction; a traced extension or removed competing
authority can suffice. A closed or superseded row can leave the plan once the commit that
removes the row cites its closure evidence; Git retains it (ADR-0042). Source-review ids stay intelligible.

An action uses the existing route appropriate to it:

- **ADR + DESIGN** for changed architecture, semantics or a meaningful alternative; accepted
  records are immutable except lifecycle metadata. A proposed ADR can carry an open decision.
- **Implementation/refactor** inside an accepted contract, with the affected owner and deletion
  obligations; no ADR is needed solely to report code movement.
- **Test or `rules/` entry** when a meaningful regression is plausible and a cheap check catches
  it. Derive mechanical validators where useful; keep independent semantic controls independent.
- **Deferred disposition** with scope consequence and the event that reopens it. Excluding a
  scenario from a slice does not establish architectural acceptance of the enclosing stage.

| Existing check | Use |
|---|---|
| Compile checks, focused tests/probes | During implementation: validate the scope just implemented; use release-profile Rust reuse |
| `just fmt` | After every turn, by the end-of-turn hook (ADR-0110); agents never run it |
| `just hygiene` | Once at scope end, beside `just test-all`; the agent fixes what fails. The hook never runs or fixes it |
| `just rules-scan`, `just rules-test` | A justified code-shape invariant; run within `hygiene` |
| `just adr-lint`, `just lint-agents` | Decision metadata and agent-facing links/commands; run within `hygiene` |
| `just test-all`, `just pilot` | Integrated functional acceptance at scope end, not every slice or process edit; qualification also cites `just hygiene` passing for the same tree |

Review quality is established by scenario reasoning and calibration, not by a new checker.

## 5. The code-intelligence graph rules, mapped to the design

The operator's graph guidelines live in the code-intelligence profile. These are current
implementation routes, **Implemented / interface-checked, 2026-10-02**; linked receipts establish
only their named Tested boundaries. [§15](../../../design/sections/semantic-model.md) and the
[alignment plan](../../../plans/semantic-model-incremental-alignment-plan_2026-10-02.md#7-current-disposition--sole-execution-owner)
own current contracts and correction acceptance.

| Profile | Governing distinction | Current mechanism / independent control route |
|---|---|---|
| CI-03 | Canonical relation identity, typed endpoints and isolates; endpoint pair is not identity | Model records and declared projections; `normalized_projections`, `projection_hydration` and graph-shape controls |
| CI-02, CI-04 | Attributed facts, normalized resolution, derivation and heuristic remain distinct; unknown is explicit | Typed support/coverage and nominal outcomes; native/normalization/selection controls |
| CI-05, DP-04 | Projection universe versus output selection; graph-local indices never escape | Model projection definitions and lineage; materialized graph/source-bound adapter controls |
| CI-08, CI-04 | Bounded paths and incomplete results never imply complete absence | Finite Summary/native path operations and explicit coverage; independent native/oracle controls |
| CI-06, CI-09 | Exact-under-context versus heuristic; method/settings/run observations explicit | Model policy/definitions and nominal outcomes; `analytics_settings`, `domain_analytics`, `ranking_policy` |
| CI-07 | Compute/graph boundary never acquires store meaning | Completed PostgreSQL sources registered with DataFusion, borrowed model kernels; actual stage/graph adapter controls |
| CI-01, DP-21 | Results retain source, invocation and proof identity | Typed derivation sources/shared invariants and actual stored-row admission; publication and serving tamper controls |
| DP-19 | Original immutable generation, acknowledged epoch receipts and publication after validation | `lctx-postgres` generation lifecycle and guards; `generation_stages`, `stage_reads`, `vocabulary_epochs`, serving lifecycle controls |
| DP-23, DP-22 | Independent known answers and operational measurement boundaries | Model/core/store fixtures and CPython oracles; separate stage/RSS/reservation observations, no current performance claim |


## 6. Where defect shapes tend to land in this codebase

Illustrative places the shapes in the core skill's reference and the profile's calibration table
have landed or would land here. Their absence proves nothing. The last column is the cheap check
to reach for where doubt remains.

| Pattern | Shape | Principles · gate | Cheap check |
|---|---|---|---|
| An extractor/decoder copying model columns or semantics | Second authority | DP-01, DP-16 · G1 | Typed codec/schema equality and independent provider cases |
| A packet reading a relation outside its typed output/child/prepared binding, including an empty read | Undeclared consumption | DP-01, DP-03 · G1/G3 | Scoped packet-reader refusal with a real generation |
| A validator replacing the shared owner operation with test-only SQL | Second authority | DP-01, DP-23 · G1 | Injected invalid-input controls through the production validator |
| An assertion/proof citing an absent or foreign-generation row | Ungrounded claim | CI-11, DP-03, DP-21 · G1/G3/CI-G2 | Actual proof-row membership and damaged-receipt controls |
| Reordered codebook codes or unreviewed schema snapshot change | Silent migration | DP-24 · G2 | Append-only codebook snapshots and reviewed `.snap.new` diff |
| Name-only dependency dedup merging different vocabulary epochs or stream orders | Lost source universe | DP-04, DP-09 · G2/G6 | Exact closure/consumed-source and epoch controls |
| Known default formals disappearing because native assignment is unsupported | Invalid validity inference | DP-02, CI-04 · G2 | Required/default public-formal twin with retained refusal |
| Statistical/heuristic evidence promoted to behavioral proof | Unbacked claim | CI-09, DP-02 · G2/G7/CI-G1 | Typed status/qualification and synthesis replay controls |
| Query relevance used as eligibility or evidence truth | Changed question | CI-11, DP-02 · G2 | Eligibility-before-ranking and original evidence controls |
| Reused vector/spec or graph/parent inventory from another acknowledged source | Invalid reuse | CI-13, DP-09 · G6 | Codec/spec mismatch, source/budget and independent replay controls |
| Serving swaps generation/guard mid-process, or publishes before validation | Inconsistent authority | CI-13, DP-04, DP-19 · G5 | Original-guard loss/replacement and failed-publication controls |
| Borrowed CPU access/charges released while an opaque kernel executes | Invalid lifetime | DP-10, DP-19 · G5/G6 | Heartbeat and cancel/drain controls |
| A failure emits backend detail, acquires a replacement grant or recursively encodes | Unsafe failure boundary | DP-03, DP-19 · G3/G5 | Actual tool/resource complete-envelope and fallback-once controls |
| Result/rendering drift across batches or valid string assumptions strengthened without premises | Nondeterminism / incorrect restriction | DP-24, CI-04 · G2 | Batch-order, shuffled-input and independent native/oracle controls |

## 7. Stack notes and known conflicts

- **Library specifics** from the graph guidelines' §5 (petgraph container choices, graphops
  naming and version caveats, leiden-rs data model, rustworkx-core scope) belong to the
  `rust-graphs` skill; DataFusion operator and provider rules to `datafusion`; PostgreSQL generation and
  receipt/lease rules to `sqlx-postgres`. The profile names no library.

| # | Standard says | Repository text says | Resolution |
|---|---|---|---|
| K1 | Library selection follows an owned capability and its current or agreed planned scenario; total integration burden matters (DP-13, DP-16) | §9.8 selects techniques for product use; §B10 defines exclusions | Evaluate product need and implementation fit separately. Planned use can justify a seam; library availability cannot establish need. Target reviews can propose an ADR changing a binding policy. |
| K2 | Licence is never a reason to reject a library (operator, 2026-09-23) | The graph guidelines' §5 asks for licence approval of rust-igraph and raphtory | The operator's position governs; judge candidates on merit |
| K3 | Principle IDs `DP-nn`, `CI-nn`; labels in principles §D | Accepted ADRs, DESIGN history and earlier reviews cite `DM-nn`, charter §D, ADDENDUM §n and guidelines §n | Read through core principles §I and §8 below; accepted records are not edited |

## 8. Lineage

The superseded sources below are retired from the tree (recover them from Git); the table lets
readers translate ids cited by older material.

| Superseded source | Now |
|---|---|
| Charter DM-01–DM-60, gates G1–G7, §D–§H | Core principles (DP-01–DP-24, G1–G8, §D–§H); mapping in core §I |
| `AGENT_DESIGN_DIRECTIVE.md`, `DESIGN_REVIEW_TEMPLATE.md` | Core principles §0–§1; core review template |
| `ADDENDUM.md` §1, §2, §3 | §1, §2, §3 here |
| `ADDENDUM.md` §4 (every finding names an oracle) | §4 here: landing places, with naming a check optional |
| `ADDENDUM.md` §5 | §5 here |
| `REVIEW_REFERENCE.md` §1 (DM index) | Retired; the core principles index themselves |
| `REVIEW_REFERENCE.md` §2–§4 | The `design-review` skill's `REFERENCE.md` |
| `REVIEW_REFERENCE.md` §5 | §6 here |
| Graph guidelines §1, §6 | CI-07 |
| Graph guidelines §2 | CI-01, CI-02, CI-03, CI-04 |
| Graph guidelines §3, §4 | CI-05 (library specifics: `rust-graphs` skill) |
| Graph guidelines §5 | CI-07 and the `rust-graphs` skill |
| Graph guidelines §7 | CI-08, CI-04 |
| Graph guidelines §8 | CI-06, CI-09 |
| Graph guidelines §9 | CI-07, DP-10, DP-20 (operator rules: `datafusion` skill) |
| Graph guidelines §10 | CI-01, DP-09, DP-21 |
| Graph guidelines §11 | DP-19, DP-04 (PostgreSQL generation rules: `sqlx-postgres` skill) |
| Graph guidelines §12 | Profile review additions (slot 4 analysis record, slot 10 known-answer shapes) |
