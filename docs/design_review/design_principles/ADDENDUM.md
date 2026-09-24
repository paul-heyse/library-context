# Charter addendum — library-context

> **Superseded 2026-09-24** by the [library-context binding](binding/library-context.md), which carries §1–§3 and §5 forward re-keyed to the new IDs. Retained so reviews citing `ADDENDUM §n` stay readable; the binding's §8 maps each section.

The charter, the directive and the review template are technology-neutral and are carried here
**verbatim**, so a `DM-nn` or `Gn` citation means the same thing here as anywhere else the charter
is used. This file is the only repository-specific layer, and it covers four things:

1. which binding decisions the charter's gates bear on (the **§B IDs**);
2. the recurring review questions for this project, routed onto **G1–G7**;
3. how the grading vocabularies stay distinct;
4. which oracles a finding can name.

`docs/design/DESIGN.md` is the **single source** for what each §B decision says. This file only
maps them onto the charter. If the two ever disagree, DESIGN.md wins and this table is stale.

---

## 1. Binding decisions and what they bear on

IDs are never reused or renumbered. Changing a §B decision needs an ADR and a `standard` review
(ADR-0001).

| ID | Binding decision (DESIGN.md is authoritative) | DESIGN | Bears on |
|---|---|---|---|
| §B1 | Pyrefly (in-process, pinned patched fork) for semantics and Ruff 0.0.11 crates for syntax, both over one parse; our own recognizer for binding history; no second parser or type checker, except `cpg-flow`'s flow facts from ty over a second parse, joined by range under two-way parity rules (ADR-0012 amendment) | §2, §4.2 | DM-02, DM-04, DM-41, DM-57, DM-58 · G1, G7 |
| §B2 | Arrow schemas in `cpg-schema` are the authoritative data contract; codebooks are append-only; nothing is inferred | §2, §3 | DM-02, DM-06, DM-09, DM-51, DM-52 · G1, G2 |
| §B3 | DataFusion constructs and validates relations; one query per rule, shared by tests and publication | §2, §4, §8 | DM-07, DM-20, DM-22, DM-53 · G3 |
| §B4 | Graph algorithms have named owners (petgraph traversal and SCCs, our own weighted PageRank, leiden-rs communities on a normalized input, own FCA/RCA; ADR-0011 as amended), each with a named consumer | §2, §5, §9 | DM-34, DM-38, DM-40, DM-46 · G6 |
| §B5 | Python semantics are custom Rust passes with stated abstractions | §2, §9 | DM-13, DM-24, DM-40, DM-59 · G2, G7 |
| §B6 | Facts are first-class assertions with run, origin, fidelity and model; disagreement is retained. Analysis results (findings, assertions, briefs) carry that provenance in-row and cite facts, edges and nodes; operator review verdicts are facts (ADR-0019) | §2, §3 | DM-08, DM-13, DM-23, DM-46, DM-47 · G1, G2 |
| §B7 | Delta canonical store, published by one `snapshots` append; readers pin versions and filter by `snapshot_id` | §2, §6 | DM-12, DM-14, DM-29, DM-30 · G5 |
| §B8 | Pyrefly and Ruff link in-process from a pinned fork with a constructed, explicit config; one workspace, one process; the Pyrefly CLI is only a parity-test oracle | §2, §4.2 | DM-28, DM-31, DM-41, DM-42, DM-43, DM-48 · G2, G4, G6 |
| §B9 | One pinned Rust dependency family (DataFusion 55.1 / Arrow 59.3 / delta-rs git) | §2, §7 | DM-31, DM-48 · G6 |
| §B10 | Exclusions: no graph DB, workflow engine, JSON-inferred schemas, whole-ontology graph, composition planner, constraint solver, neural reranking or graph embeddings | §2 | DM-57, DM-58 · — |
| §B11 | Insight synthesis is programmatic (templates, extractive selection); no generative model in v1, and never in the query path | §2, §10 | DM-08, DM-22, DM-40, DM-59 · G2, G7 |
| §B12 | Canonical Delta store vs rebuildable serving projections; no cross-store transactions | §2, §6.4 | DM-02, DM-14, DM-23 · G1, G5 |
| §B13 | The agent interface is a FastMCP server over one pinned file-based generation per process | §2, §11.3 | DM-37, DM-41, DM-43 · G2, G3, G7 |
| §B14 | One hashed embedding spec, cached vectors, conformance-checked Rust and Python clients | §2, §11.1 | DM-31, DM-32, DM-40, DM-48 · G6 |

`docs/initial_plan/Initial_plan.md` is **research input**, not authority. DESIGN.md may depart
from it. A departure is legitimate when DESIGN.md or an ADR says so, and a finding when neither
does.

---

## 2. Recurring review questions, routed onto the gates

A review that settles the gates has answered these. Questions with no gate are proportionality
and ergonomics tests: they belong in §7 findings and §8 alternatives, and can never rescue or
sink a gate.

| # | Question | Gate | Principles |
|---|---|---|---|
| 1 | Can every fact, finding and assertion be traced to its provider, analyzer revision, run, environment and source span? | G1 | DM-46, DM-48, DM-31 |
| 2 | Is any semantic fact editable in two places, such as a schema and a hand-written validator, a codebook and a match arm, a family table and a derived view, or the canonical store and the serving bundle? | G1 | DM-02, DM-23 |
| 3 | Does every brief assertion cite only findings and evidence that exist in the **same** snapshot, and does every named public symbol and parameter exist there? | G1, G3 | DM-07, DM-46 |
| 4 | Are unavailable, not-requested, failed and unresolved distinguishable, and does an empty result or an `unresolved` slot stay distinct from "absent"? | G2 | DM-08, DM-30 |
| 5 | Is Pyrefly's inference graph relabelled as runtime dataflow, an `ifCalled` target as a call, or a call edge as "always reached" or "recommended"? | G2 | DM-13, DM-24, DM-34 |
| 6 | Can `statistically_derived` output (communities, centrality, kNN) state a control, a limit or a behavioral claim in a brief? | G2, G7 | DM-08, DM-59 |
| 7 | Can an unvalidated batch, or one with unmapped endpoints, reach the `snapshots` append? | G3, G5 | DM-07, DM-14, DM-22 |
| 8 | Can a reader load a table at "latest" or without the `snapshot_id` filter, or mistake an aborted attempt for a published one? | G5 | DM-14, DM-29, DM-30 |
| 9 | Does the serving generation name its canonical snapshot, and does a server process hold exactly one generation for its lifetime? | G5 | DM-12, DM-14 |
| 10 | Does a projection, a parallel-arc collapse or a dense index lose the facts that support it, or leak into persistent identity? | G6 | DM-23, DM-40, DM-46 |
| 11 | Do traversal, community detection and concept analysis give identical output for shuffled input, with seeds, parameters and crate versions recorded? | G6 | DM-28, DM-40, DM-48 |
| 12 | Can a query-time vector and a compile-time vector come from different embedding specs, or a cached vector be reused after the spec changed? | G6 | DM-31, DM-32 |
| 13 | Does an extractor, decoder or DESIGN claim cover a fact family it only partly extracts? | G7 | DM-43, DM-44, DM-59 |
| 14 | Does a read, validation or inspection path mutate, fetch, build, or read ambient state, including analyzer config discovery? | G4 | DM-20, DM-28 |
| 15 | Can anything under `.claude/skills/` (the gold reference) reach the compiler's inputs, or can analytics parameters be tuned on the gold? This is an undeclared input that changes output, and it makes the §12 evaluation circular. | G4 | DM-28, DM-31, DM-59 |
| 16 | Does a new fact family or technique need one declaration plus focused tests, or edits across several subsystems? | — | DM-56, charter §E |
| 17 | Does each added layer or technique (registry, generator, service, analytic) have a named consumer in the brief, and does its ablation change published output? | — | DM-57, DM-58 |

---

## 3. Five vocabularies, kept distinct

| Vocabulary | Grades… | Values | Defined in |
|---|---|---|---|
| **Charter §D claim labels** | the strength of a **design claim** | `Proposed` · `Interface-checked` · `Implemented` · `Tested` · `Measured` · `Formally established` | charter §D |
| **Check outcomes** | the result of **running a check** (`just` recipe, test) | `passed` · `failed` · `blocked` · `not_run` | `AGENTS.md` |
| **Fact-graph data vocabularies** | **data inside the product**, not claims about the design | `origin`, `fidelity`, `modality`, coverage status, type role | DESIGN §3.5 |
| **Assertion evidence status** | the **support behind one brief assertion**, as published to agents | `structurally_observed` · `documented` · `statistically_derived` · `fixture_checked` · `unresolved` | DESIGN §10.2 |
| **Behavior verdict** | **one behavioral answer in the product** (a fate, a delegation, a negative claim), under the stated runtime model | `established` · `conditional` · `refuted_under_model` · `unknown` · `not_analyzed` | DESIGN §3.9, ADR-0022 |

These are never interchangeable:
- A check outcome is never `Measured`, and a design claim is never `not_run`.
- `complete_under_stated_model` is a value in a table, not evidence that the design is complete.
- An assertion's `evidence_status` grades that assertion's support inside the product. It is not a
  §D label: a `fixture_checked` assertion says nothing about how well the design is established,
  and a `Tested` design claim says nothing about any one brief.
- A behavior verdict grades one answer under the stated model. `established` is not `Tested`, and
  `refuted_under_model` is not a proof of absence outside the model. A brief rendering a behavior
  maps it to an evidence status (DESIGN §3.9); `modality` stays the call graph's input fact.
- A check is `passed` only when a command ran and its output was read. A missing tool is
  `blocked`, with the prerequisite named.
- A mocked provider is never a pass.
- Outcomes are never converted into a percentage.

Design claims in DESIGN.md, in an ADR's `evidence:` field and in a review carry a §D label.
`Proposed` is not a failure state, but **an unlabelled claim is**.

---

## 4. Findings must name an oracle

Choose the cheapest reproducible oracle:

| Tier | Where | When |
|---|---|---|
| **test** | `cargo nextest` / pytest, including insta schema snapshots and the shared DataFusion validators | the behavior or invariant can be exercised |
| **ast-grep rule** | `rules/`, with fixtures in `rule-tests/`, run by `just rules-scan` and `just rules-test` | the defect is a code shape |
| **`just` recipe** | e.g. `just deps`, `just adr lint`, `just lint-agents` | the check needs the whole repo or its metadata |
| **prose** | `AGENTS.md` | only when no mechanical oracle exists; say so |

There is no blocking hook tier. The only hook is format-on-edit, and nothing is sealed from the
agent (ADR-0001). A §7 finding's *Verification* column names one tier, or says explicitly that no
oracle exists. That absence is often the most useful output, because it is where a new test or
rule should land.

---

## 5. The operator's graph guidelines, mapped to the design

[`rust_code_intelligence_data_graph_guidelines.md`](rust_code_intelligence_data_graph_guidelines.md)
(operator, 2026-09-22) adds MUSTs for the CPG and the analyses that read it. A review checks the
design against them as it does against the charter. This table routes each MUST to the mechanism
that meets it and the oracle that would catch a regression (ADR-0014). Rows marked *consumer* are
met when the first projection or analysis lands (increment 1, slice 4 onward). Until then the CPG
supplies their inputs.

| Guideline MUST | Mechanism | DESIGN | Oracle |
|---|---|---|---|
| §2 relationships have a persistent `edge_id`, typed endpoints, a relation kind, evidence and snapshot scope; `(src, dst)` is not an identity | `edges` from the registry; content-derived `edge_id` with a discriminator; `edge_kind` codebook | §3.4.1, §3.8 | `key:edges`; the parallel-sites test; the endpoint and evidence rules |
| §2 isolates survive through an explicit node relation | `nodes` from per-kind existence sources, independent of edges | §3.1, §3.8 | the `graph_shapes` isolate; the graph-readiness reader |
| §2 extracted, resolved, derived and heuristic stay distinguishable; unknown targets explicit | a derivation class per edge kind, published in `edge_kinds`; `facts` provenance; candidate sets kept (name resolution, lexical mentions); external and synthetic nodes; unresolved remainders and provider reasons, no catch-all; `types` boundaries for what Pyrefly does not type; `graph_gaps` for rows not yet in the graph (empty since C3) | §3.2, §3.5, §3.6, §3.7, §3.8 | lineage rules (the edit guards among them declared, §8); `typed:*` rules and their injected-violation tests (`every_rule_kind_rejects_its_violation`, `the_types_rules_reject_their_violations`, `the_corpus_rules_reject_their_violations`, `each_graph_rule_rejects_a_doctored_catalog`); `partition:pysa_calls-remainders` and its case; `every_rule_is_exercised_or_declared_an_edit_guard`; `edge_kinds` in the derivations snapshot |
| §3 a projection's spec; separate output selector and universe; declared direction; weight semantics | `cpg_schema::projection::ProjectionSpec` (slice 1.4): vertex universe apart from arcs, edge kinds, accepted modalities, origins and fidelities, candidate, unknown-target and weight policies, generated SQL, a digest on every invocation | §3.7, §3.8, §5 | `pass_a_finds_the_known_answers_on_analysis_shapes`; the adapter test |
| §4 graph-local indices never become identities; simplification preserves the question  | §5's three identities; the catalogs hold no indices | §3.8, §5 | the graph-readiness reader; `the_adapter_keeps_parallel_arcs_isolates_and_refuses_disorder`; the first projection's tests |
| §7 no eager closures or path enumeration; partial is not complete | the catalogs hold direct relations only; coverage, boundaries, external and synthetic nodes and `graph_gaps` state where the graph stops | §3.7, §3.8 | review question 13; `partition:pysa_calls-remainders` (the gaps partition is an edit guard over an empty table, §8) |
| §8 exact, conservative and heuristic results stay apart; method, parameters, seed, convergence recorded or reported unavailable | `FINDING_STATUS` per finding kind; `analysis_invocations` records seed, iterations, residual, convergence and quality history, null meaning unavailable | §9 | `semantic:finding-status-policy` |
| §9 an explicit DataFusion ↔ graph boundary | projection SQL on the attempt's session, cast to declared schemas, then `lctx-analytics` (Arrow in, Arrow out; no DataFusion or Delta), rows written back through the attempt | §4.1, §5 | `lctx-analytics` tests run with no store |
| §10 results carry run and projection lineage and evidence | `analysis_invocations` (method, parameters JSON, projection digest, library versions, diagnostics), `findings`, `finding_members`, `witnesses` with `edge_id` lineage; provenance in-row (ADR-0019) | §3.2, §9 | `semantic:invocation-model-producer`, `semantic:witness-chain`, `semantic:finding-status-policy`, `semantic:invocation-run-is-compiler`, each with an injected case |
| §11 Delta read through a pinned snapshot; a manifest after validation; retention for files live snapshots need | `snapshots` append; pinned `with_version` reads with a `snapshot_id` filter; cleanup off, retention verified at open. An attempt holds one run per release (the library, its corpus); what both assert is one node | §3.4, §6.1, §6.2 | the reader tests; the retention test; `unique:release-paths`, `unique:type_terms` |
| §12 known-answer shapes (isolates, parallel edges, self-loops, reconvergence, cross-file cycles, unresolved, mixed configuration); lineage; per-stage instrumentation | the `graph_shapes`, `syntax_shapes`, `lexical_shapes`, `type_shapes` and `docs_shapes` fixtures; generated lineage rules; `lctx compile` stage metrics, with validation's slowest rules | §3.8, §4.3 | insta catalog snapshots; `just pilot` and the C6 measurement, with its allocator setting stated (§4.3) |
