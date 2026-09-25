# library-context binding

The repository layer of the design standard, and the successor to `ADDENDUM.md`. It defines no
principles; it maps the [core design principles](../core/design-principles.md) and the
[code-intelligence profile](../profiles/code-intelligence/principles.md) onto this repository's
binding decisions, review cadence, vocabularies and code. `docs/design/DESIGN.md` plus `docs/design/sections/` is the architectural collection; DESIGN §2 delegates review mechanics to this page under
ADR-0040. Other design contracts and accepted decisions govern their domains; report a conflict and its resolution route.

## Standard applied

| Layer | Document | Version |
|---|---|---|
| Core | [design principles](../core/design-principles.md), [review template](../core/design-review-template.md) | 3.0 (repository-owned edition, ADR-0040) |
| Profile | [code-intelligence principles](../profiles/code-intelligence/principles.md), [review additions](../profiles/code-intelligence/review.md) | 1.1 |
| Binding | this page | — |

The version/path declaration in [`standard.toml`](../standard.toml) is authoritative for loading.
ADR-0040 gives this repository ownership of core 3.0; it replaces ADR-0023's unlocated shared-copy
requirement. Other repositories are not changed by a local revision. Preserve DP/CI IDs and historical
version semantics; FP-01–FP-06 and A1–A3 supply the architecture structure.

## Reviews in this repository

- **Location:** `docs/design_review/reviews/design_review_{slug}_{YYYY-MM-DD}.md`.
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
  or a stale DESIGN section; distinguish both from an accepted target still awaiting implementation. `docs/initial_plan/Initial_plan.md` is
  research input: the collection may depart from it when its owner section or an ADR says so, and an
  unrecorded departure is a finding.
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
`just docs-test` / `just docs-check`, independently of integrated product qualification.

## 1. Binding decisions and what they bear on

IDs are never reused or renumbered. Changing a §B decision needs an ADR and a design/target review.

| ID | Binding decision (DESIGN.md §2 is authoritative) | Bears on |
|---|---|---|
| §B1 | Ruff and Pyrefly are the only semantic front ends, over one parse; `cpg-flow`'s flow facts from ty are the declared exception, joined by range under parity rules | DP-01, DP-08, DP-14, DP-16, CI-02 · G1, G7 |
| §B2 | Arrow schemas in `cpg-schema` are the authoritative data contract; codebooks are append-only | DP-01, DP-02, DP-03, DP-16, DP-24 · G1, G2 |
| §B3 | DataFusion constructs and validates relations; one query per rule, shared by tests and publication | DP-03, DP-08, DP-18, DP-23 · G3 |
| §B4 | Graph algorithms have named owners and named consumers | DP-07, DP-11, DP-13, DP-21, CI-05, CI-07, CI-09 · G6, G8 |
| §B5 | Python semantics are custom Rust passes with stated abstractions | DP-05, DP-08, DP-11, DP-22, CI-06 · G2, G7, CI-G1 |
| §B6 | Facts are first-class assertions with run, origin, fidelity and model; disagreement is retained; analysis results carry provenance in-row | DP-01, DP-02, DP-05, DP-21, CI-01 · G1, G2, CI-G1 |
| §B7 | Delta canonical store, published by one `snapshots` append; readers pin versions and filter by `snapshot_id` | DP-04, DP-19 · G5 |
| §B8 | Pyrefly and Ruff linked in-process from a pinned fork with a constructed, explicit config | DP-09, DP-14, DP-15, DP-18, DP-21, CI-10 · G2, G4, G6 |
| §B9 | One pinned Rust dependency family | DP-09, DP-15, DP-21 · G6, G7 |
| §B10 | The standing exclusions | DP-16 · — |
| §B11 | Insight synthesis is programmatic; no generative model in v1, and never in the query path | DP-02, DP-08, DP-11, DP-22, CI-11 · G2, G7, CI-G2 |
| §B12 | Canonical Delta store vs rebuildable serving projections; no cross-store transactions | DP-01, DP-19, CI-13 · G1, G5 |
| §B13 | The agent interface is a FastMCP server over one pinned file-based generation per process | DP-10, DP-14, DP-15, CI-13 · G2, G3, G7 |
| §B14 | One hashed embedding spec, cached vectors, conformance-checked Rust and Python clients | DP-09, DP-11, DP-21, CI-13 · G6 |

## 2. Recurring review questions, routed onto the gates

Use these domain questions after reconstructing responsibilities and expected changes. Their
gates constrain supported behavior; A1–A3 independently determine architectural fitness. A
concrete change-propagation or testability defect can require revision without a failed G gate.

| # | Question | Gate | Principles |
|---|---|---|---|
| 1 | Can every fact, finding and assertion be traced to its provider, analyzer revision, run, environment and source span? | G1, CI-G1 | CI-01, DP-21 |
| 2 | Is any semantic fact editable in two places, such as a schema and a hand-written validator, a codebook and a match arm, a family table and a derived view, or the canonical store and the serving bundle? | G1 | DP-01 |
| 3 | Does every brief assertion cite only findings and evidence that exist in the **same** snapshot, and does every named public symbol and parameter exist there? | CI-G2 | CI-11 |
| 4 | Are unavailable, not-requested, failed and unresolved distinguishable, and does an empty result or an `unresolved` slot stay distinct from "absent"? | CI-G1 | CI-04, DP-02 |
| 5 | Is Pyrefly's inference graph relabelled as runtime dataflow, an `ifCalled` target as a call, or a call edge as "always reached" or "recommended"? | CI-G1 | CI-02 |
| 6 | Can `statistically_derived` output (communities, centrality, kNN) state a control, a limit or a behavioral claim in a brief? | CI-G1 | CI-09 |
| 7 | Can an unvalidated batch, or one with unmapped endpoints, reach the `snapshots` append? | G3, G5 | DP-03, DP-19 |
| 8 | Can a reader load a table at "latest" or without the `snapshot_id` filter, or mistake an aborted attempt for a published one? | G5 | DP-19 |
| 9 | Does the serving generation name its canonical snapshot, and does a server process hold exactly one generation for its lifetime? | G5 | CI-13 |
| 10 | Does a projection, a parallel-arc collapse or a dense index lose the facts that support it, or leak into persistent identity? | G6 | CI-03, CI-05 |
| 11 | Do traversal, community detection and concept analysis give identical output for shuffled input, with seeds, parameters and crate versions recorded? | G6 | CI-09, DP-11 |
| 12 | Can a query-time vector and a compile-time vector come from different embedding specs, or a cached vector be reused after the spec changed? | G6 | CI-13, DP-09 |
| 13 | Does an extractor, decoder or DESIGN claim cover a fact family it only partly extracts? | G7 | DP-15, CI-04 |
| 14 | Does a read, validation or inspection path mutate, fetch, build, or read ambient state, including analyzer config discovery? | G4 | DP-18, CI-10 |
| 15 | Can anything under `.claude/skills/` (the gold reference) reach the compiler's inputs, or can analytics parameters be tuned on the gold? | CI-G3 | CI-12 |
| 16 | Does an ordinary model, analytic or rendering reuse contracts, and where is semantic meaning independently re-expressed? Distinguish a new concept from an instance. | A1, A2, A3 | FP-01–06, DP-16, core §E |
| 17 | Does each added layer or technique have a named consumer in the brief, and does its ablation change published output (DESIGN §9.8)? | — | DP-16 |

## 3. Vocabulary and scenario authorities

| Meaning | Authority |
|---|---|
| Design claim strength | Core principles §D; label and date the claim actually established |
| Architectural judgments | Core §E, A1–A3; report separately from gate verdicts |
| Correctness/fidelity gate verdicts | Core §A and profile gates |
| Check execution outcomes | AGENTS.md Reporting; each outcome names its command |
| Domain fidelity, verdicts and boundary reasons | `cpg-schema` contracts/codebooks and DESIGN §3/§9; never infer them from review labels |

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
authority can suffice. Preserve closed/superseded rows for traceability.

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
| Focused tests/probes | Resolve material implementation uncertainty; use release-profile Rust reuse |
| `just rules-scan`, `just rules-test` | A justified code-shape invariant |
| `just adr lint`, `just lint-agents` | Decision metadata and agent-facing links/commands |
| `just test-all`, `just pilot` | Integrated functional acceptance at scope end, not every slice or process edit |

Review quality is established by scenario reasoning and calibration, not by a new checker.

## 5. The operator's graph guidelines, mapped to the design

The guidelines' domain rules now live in the code-intelligence profile (lineage in §8). This
table keeps the mechanism that meets each rule in this repository and the checks already in
place (ADR-0014).

| Profile | Guideline rule | Mechanism | DESIGN | Checks in place |
|---|---|---|---|---|
| CI-03 | §2 relationships have a persistent `edge_id`, typed endpoints, a relation kind, evidence and snapshot scope; `(src, dst)` is not an identity | `edges` from the registry; content-derived `edge_id` with a discriminator; `edge_kind` codebook | §3.4.1, §3.8 | `key:edges`; the parallel-sites test; the endpoint and evidence rules |
| CI-03 | §2 isolates survive through an explicit node relation | `nodes` from per-kind existence sources, independent of edges | §3.1, §3.8 | the `graph_shapes` isolate; the graph-readiness reader |
| CI-02, CI-04 | §2 extracted, resolved, derived and heuristic stay distinguishable; unknown targets explicit | a derivation class per edge kind, published in `edge_kinds`; `facts` provenance; candidate sets kept (name resolution, lexical mentions); external and synthetic nodes; unresolved remainders and provider reasons, no catch-all; `types` boundaries for what Pyrefly does not type; `graph_gaps` for rows not yet in the graph (empty since C3) | §3.2, §3.5, §3.6, §3.7, §3.8 | lineage rules (the edit guards among them declared, §8); `typed:*` rules and their injected-violation tests (`every_rule_kind_rejects_its_violation`, `the_types_rules_reject_their_violations`, `the_corpus_rules_reject_their_violations`, `each_graph_rule_rejects_a_doctored_catalog`); `partition:pysa_calls-remainders` and its case; `every_rule_is_exercised_or_declared_an_edit_guard`; `edge_kinds` in the derivations snapshot |
| CI-05 | §3 a projection's spec; separate output selector and universe; declared direction; weight semantics | `cpg_schema::projection::ProjectionSpec` (slice 1.4): vertex universe apart from arcs, edge kinds, accepted modalities, origins and fidelities, candidate, unknown-target and weight policies, generated SQL, a digest on every invocation | §3.7, §3.8, §5 | `pass_a_finds_the_known_answers_on_analysis_shapes`; the adapter test |
| CI-05, DP-04 | §4 graph-local indices never become identities; simplification preserves the question  | §5's three identities; the catalogs hold no indices | §3.8, §5 | the graph-readiness reader; `the_adapter_keeps_parallel_arcs_isolates_and_refuses_disorder`; the first projection's tests |
| CI-08, CI-04 | §7 no eager closures or path enumeration; partial is not complete | the catalogs hold direct relations only; coverage, boundaries, external and synthetic nodes and `graph_gaps` state where the graph stops | §3.7, §3.8 | review question 13; `partition:pysa_calls-remainders` (the gaps partition is an edit guard over an empty table, §8) |
| CI-06, CI-09 | §8 exact, conservative and heuristic results stay apart; method, parameters, seed, convergence recorded or reported unavailable | `FINDING_STATUS` per finding kind; `analysis_invocations` records seed, iterations, residual, convergence and quality history, null meaning unavailable | §9 | `semantic:finding-status-policy` |
| CI-07 | §9 an explicit DataFusion ↔ graph boundary | projection SQL on the attempt's session, cast to declared schemas, then `lctx-analytics` (Arrow in, Arrow out; no DataFusion or Delta), rows written back through the attempt | §4.1, §5 | `lctx-analytics` tests run with no store |
| CI-01, DP-21 | §10 results carry run and projection lineage and evidence | `analysis_invocations` (method, parameters JSON, projection digest, library versions, diagnostics), `findings`, `finding_members`, `witnesses` with `edge_id` lineage; provenance in-row (ADR-0019) | §3.2, §9 | `semantic:invocation-model-producer`, `semantic:witness-chain`, `semantic:finding-status-policy`, `semantic:invocation-run-is-compiler`, each with an injected case |
| DP-19 | §11 Delta read through a pinned snapshot; a manifest after validation; retention for files live snapshots need | `snapshots` append; pinned `with_version` reads with a `snapshot_id` filter; cleanup off, retention verified at open. An attempt holds one run per release (the library, its corpus); what both assert is one node | §3.4, §6.1, §6.2 | the reader tests; the retention test; `unique:release-paths`, `unique:type_terms` |
| DP-23, DP-22 | §12 known-answer shapes (isolates, parallel edges, self-loops, reconvergence, cross-file cycles, unresolved, mixed configuration); lineage; per-stage instrumentation | the `graph_shapes`, `syntax_shapes`, `lexical_shapes`, `type_shapes` and `docs_shapes` fixtures; generated lineage rules; `lctx compile` stage metrics, with validation's slowest rules | §3.8, §4.3 | insta catalog snapshots; `just pilot` and the C6 measurement, with its allocator setting stated (§4.3) |

## 6. Where defect shapes tend to land in this codebase

Illustrative places the shapes in the core skill's reference and the profile's calibration table
have landed or would land here. Their absence proves nothing. The last column is the cheap check
to reach for where doubt remains.

| Pattern | Shape | Principles · gate | Cheap check |
|---|---|---|---|
| An extractor or decoder re-declaring a table's columns instead of using the `cpg-schema` spec | Second authority | DP-01, DP-16 · G1 | test: extractor output schema equals the spec, metadata included |
| A pass writing `nodes`/`edges` directly instead of its family table, so the derived view and the family disagree | Second authority | DP-01 · G1 | test: regenerated views equal the stored ones on a fixture; `ast-grep` rule on writes to view tables |
| A validator's SQL duplicated in a test instead of calling the shared library function | Second authority | DP-01, DP-23 · G1 | `ast-grep` rule flagging inline validation SQL in tests |
| A `HAS_TYPE` edge materialized as its own mutable table instead of a view over `type_observations` | Second authority | DP-01 · G1 | test: view and observations agree on a fixture |
| An assertion citing a finding or evidence id from another snapshot, or a public symbol absent from `exports` | Ungrounded claim | CI-11, DP-03, DP-21 · G1/G3/CI-G2 | test: the §10.4 grounding validator on a fixture with a stale id |
| A codebook code reassigned or reordered | Silent migration | DP-24 · G1/G2 | test: codebook append-only snapshot |
| A schema change without a reviewed snapshot diff | Silent migration | DP-24 · G2 | test: insta schema snapshots under `INSTA_UPDATE=no` |
| An inner join in stage C/D dropping unmapped provider endpoints | Silent degradation | CI-04, DP-02, DP-15 · G2/CI-G1 | test: fixture with an unresolvable target yields a `boundaries` row |
| A module an extractor never reached having no `coverage` row | Silent degradation | CI-04, DP-02, DP-19 · G2/CI-G1 | validator: coverage completeness per declared family × module |
| A Pyrefly type kept only as its display string where Pysa gave structure | Silent degradation | CI-02, DP-15, DP-02 · G2/CI-G1 | test: `fidelity` is `display_only` whenever structure is absent |
| Pysa `ifCalled` targets emitted as call targets, or `artificial-call` sites without `synthetic_model` origin | Silent reinterpretation | CI-02, DP-05, DP-08 · G2/CI-G1 | test on a higher-order-call fixture |
| Glean caller→callee pairs used for `calls` (they drop unresolved calls) | Silent degradation | CI-04, DP-02, DP-15 · G2/CI-G1 | test: an unresolved call on a fixture yields a `resolutions` row with a reason |
| A Pass B guard or forwarding site promoted despite an earlier binding of the parameter's name | Silent reinterpretation | CI-06, DP-08, DP-22 · G2/CI-G1 | test: the "guard after parameter rebinding" fixture yields `ambiguous_binding` |
| A template emitting a control, limit or behavioral claim from a `statistically_derived` finding | Unbacked claim | CI-09, DP-02, DP-22 · G2/G7/CI-G1 | test over the assertion builder; validator query rejecting statistical status on control/limit assertions |
| A brief's conditions or limits hydrated by a second semantic search instead of deterministic joins on `(snapshot_id, brief_id)` | Optional warning | CI-11, DP-02, DP-01 · G2 | test: every limit of a selected brief is returned regardless of query wording |
| A deterministic ID built from a span alone, or including a tempdir path or timestamp | Incomplete / volatile key | DP-04, DP-09 · G6 | proptest: same-range syntax nodes get distinct IDs; IDs stable across two runs |
| A run identity missing the analyzer revision, adapter build, search paths or config digests | Incomplete reuse key | DP-09, DP-21 · G6 | test: changing one input changes `run_id` |
| Community detection, FCA or witness selection depending on input row order, an unrecorded seed, or an unpinned `rand` | Nondeterminism | CI-09, DP-18, DP-11 · G6 | test: shuffled-input fixture gives byte-identical findings |
| A traversal relying on `Bfs`/`Dfs` sibling order or `edges_directed` order instead of sorting by canonical key | Nondeterminism | DP-11 · G6 | test: same projection built in two insertion orders yields identical witnesses |
| Dense petgraph indices persisted as, or joined to, canonical IDs | Leaked temporary coordinate | CI-05, DP-04, DP-11 · G6 | `ast-grep` rule on projection-index writes outside the projection module |
| A parallel-arc collapse that drops the arc-to-fact mapping | Lost lineage | CI-03, DP-21 · G6 | test on a parallel-arcs fixture |
| A query-time embedding produced or accepted without checking the generation's `spec_hash`, or a cached vector reused after the spec changed | Invalid reuse | CI-13, DP-09 · G6 | test: conformance vectors across the Rust and Python clients; a spec-mismatch fixture is rejected |
| A reader loading a Delta table at "latest", building a provider on an already-loaded handle, or omitting the `snapshot_id` filter | Inconsistent revision | DP-19 · G5 | `ast-grep` rule on `DeltaTable` loads outside the reader module; test with an unpublished attempt present |
| A `snapshots` row appended before validation passes, or an attempt retried under the same `snapshot_id` after a write error | Unguarded commit | DP-19 · G5 | test: an injected validation or write failure publishes nothing |
| The MCP server swapping generations mid-process, or a generation manifest not naming its snapshot | Inconsistent revision | CI-13, DP-04, DP-19 · G5 | test: `Client(mcp)` sees one generation key across calls; manifest schema test |
| A raw Parquet directory scan of a Delta table | Hidden semantics change | DP-18, DP-08 · G4 | `ast-grep` rule on `read_parquet` over table paths |
| An analyzer run that discovers its own config or reads an ambient environment | Hidden input | CI-10, DP-18, DP-09 · G4 | test: the Pyrefly invocation carries an explicit config; `context_id` changes when the config changes |
| Gold-reference paths (`.claude/skills/**`) read by the compiler, or analytics parameters tuned against the gold | Hidden input; circular evaluation | CI-12, DP-18, DP-22 · G4/CI-G3 | `ast-grep` rule on skill paths in compiler code; test that no library definition (`libraries/*/pyproject.toml`) or environment path points into `.claude/skills/` |
| `unwrap` / `expect` in a report decoder or at a bundle or IPC boundary | Unguarded boundary | DP-03, DP-15 · G3 | test with a malformed report; clippy |
| A FastMCP tool returning a bare list, or anything printing to stdout under stdio | Protocol degradation | DP-15 · G2 | test: `Client(mcp)` asserts object `structured_content` in both protocol modes; `ast-grep` rule on `print(` in `python/lctx_mcp` |
| A second Arrow, DataFusion or delta-rs version entering the lockfile | Dependency drift | DP-09 · G6 | `just deps` (exists) |
| An extractor advertising a fact family it only partly extracts | Unbacked capability | CI-04, CI-04, DP-15 · G7 | test: coverage rows per family on the fixture corpus |
| An analytic technique kept although its ablation changes no published output | Unearned machinery | DP-16 · — | the §9.8 ablation diff |
| A check reported `passed` without its command having run, or a mocked provider counted as a pass | Unbacked capability | DP-22 · G7 | prose (`AGENTS.md`); no mechanical oracle |
| A benchmark measuring one stage while the claim is end-to-end | — | DP-22 | name the conditions, or relabel the claim |

## 7. Stack notes and known conflicts

- **Library specifics** from the graph guidelines' §5 (petgraph container choices, graphops
  naming and version caveats, leiden-rs data model, rustworkx-core scope) belong to the
  `rust-graphs` skill; DataFusion operator and provider rules to `datafusion`; Delta snapshot and
  retention rules to `deltalake`. The profile names no library.

| # | Standard says | Repository text says | Resolution |
|---|---|---|---|
| K1 | Library selection follows an owned capability and its current or agreed planned scenario; total integration burden matters (DP-13, DP-16) | §9.8 selects techniques for product use; §B10 defines exclusions | Evaluate product need and implementation fit separately. Planned use can justify a seam; library availability cannot establish need. Target reviews can propose an ADR changing a binding policy. |
| K2 | Licence is never a reason to reject a library (operator, 2026-09-23) | The graph guidelines' §5 asks for licence approval of rust-igraph and raphtory | The operator's position governs; judge candidates on merit |
| K3 | Principle IDs `DP-nn`, `CI-nn`; labels in principles §D | Accepted ADRs, DESIGN history and earlier reviews cite `DM-nn`, charter §D, ADDENDUM §n and guidelines §n | Read through core principles §I and §8 below; accepted records are not edited |

## 8. Lineage

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
| Graph guidelines §11 | DP-19, DP-04 (Delta rules: `deltalake` skill) |
| Graph guidelines §12 | Profile review additions (slot 4 analysis record, slot 10 known-answer shapes) |
