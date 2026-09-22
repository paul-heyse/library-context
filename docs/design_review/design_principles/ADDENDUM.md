# Charter addendum — library-context

The charter, the directive and the review template are technology-neutral and are carried here
**verbatim**, so a `DM-nn` or `Gn` citation means the same thing here as anywhere else the charter
is used. This file is the only repository-specific layer, and it covers four things:

1. which binding decisions the charter's gates bear on (the **§B IDs**);
2. the recurring review questions for this project, routed onto **G1–G7**;
3. how the three grading vocabularies stay distinct;
4. which oracles a finding can name.

`docs/design/DESIGN.md` is the **single source** for what each §B decision says. This file only
maps them onto the charter. If the two ever disagree, DESIGN.md wins and this table is stale.

---

## 1. Binding decisions and what they bear on

IDs are never reused or renumbered. Changing a §B decision needs an ADR and a `standard` review
(ADR-0001).

| ID | Binding decision (DESIGN.md is authoritative) | DESIGN | Bears on |
|---|---|---|---|
| §B1 | Ruff and Pyrefly are the only semantic front ends; no second parser or type checker | §2 | DM-04, DM-41, DM-57, DM-58 · G7 |
| §B2 | Arrow schemas in `cpg-schema` are the authoritative data contract; codebooks are append-only | §2, §3 | DM-02, DM-06, DM-09, DM-51, DM-52 · G1, G2 |
| §B3 | DataFusion constructs and validates relations; the same validators run in tests and before publication | §2, §4, §8 | DM-07, DM-22, DM-38, DM-53 · G3 |
| §B4 | petgraph only for topology over small, declared projections | §2, §5 | DM-34, DM-38, DM-40, DM-46 · G6 |
| §B5 | Python semantics (CFG, dataflow, aliasing) are custom Rust passes with stated abstractions | §2 | DM-13, DM-24, DM-40, DM-59 · G2, G7 |
| §B6 | Facts are first-class assertions with run, origin, fidelity and model; disagreement is retained | §2, §3 | DM-08, DM-13, DM-23, DM-46, DM-47 · G1, G2 |
| §B7 | Delta persistence behind an immutable snapshot manifest; readers never pick "latest" per table | §2, §6 | DM-12, DM-14, DM-29, DM-30 · G5 |
| §B8 | Pinned adapters run as separate processes and emit Arrow IPC | §2, §4.2 | DM-31, DM-37, DM-41, DM-42, DM-48 · G2, G6 |
| §B9 | One pinned Rust dependency family (DataFusion 55.1 / Arrow 59.3 / delta-rs git) | §2, §7 | DM-31, DM-48 · G6 |
| §B10 | Exclusions: no graph DB, embeddings, workflow engine, JSON-inferred schemas, or whole-ontology graph | §2 | DM-57, DM-58 · — |

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
| 1 | Can every fact be traced to its provider, analyzer revision, run, environment and source span? | G1 | DM-46, DM-48, DM-31 |
| 2 | Is any semantic fact editable in two places, such as a schema and a hand-written validator, a codebook and a match arm, or a canonical table and a convenience view? | G1 | DM-02, DM-23 |
| 3 | Are unavailable, not-requested, failed and unresolved distinguishable, and does an empty result stay distinct from "absent"? | G2 | DM-08, DM-30 |
| 4 | Is Pyrefly's inference graph or `Phi` relabelled as runtime dataflow or SSA, or an `ifCalled` target as a call? | G2 | DM-13, DM-24, DM-34 |
| 5 | Can an unvalidated batch, or one with unmapped endpoints, reach Delta publication? | G3, G5 | DM-07, DM-14, DM-22 |
| 6 | Can a reader assemble an inconsistent snapshot from independently "latest" tables, or mistake a partial run for a published one? | G5 | DM-14, DM-29, DM-30 |
| 7 | Does a projection, a parallel-arc collapse or a dense index lose the facts that support it, or leak into persistent identity? | G6 | DM-23, DM-40, DM-46 |
| 8 | Does an adapter, report decoder or DESIGN claim cover a fact family it only partly extracts? | G7 | DM-43, DM-44, DM-59 |
| 9 | Does a read, validation or inspection path mutate, fetch, build, or read ambient state? | G4 | DM-20, DM-28 |
| 10 | Does a new fact family need one declaration plus focused tests, or edits across several subsystems? | — | DM-56, charter §E |
| 11 | Does each added layer (registry, generator, IR, service) have a demonstrated consumer? | — | DM-57, DM-58 |

---

## 3. Three vocabularies, kept distinct

| Vocabulary | Grades… | Values | Defined in |
|---|---|---|---|
| **Charter §D claim labels** | the strength of a **design claim** | `Proposed` · `Interface-checked` · `Implemented` · `Tested` · `Measured` · `Formally established` | charter §D |
| **Check outcomes** | the result of **running a check** (`just` recipe, test) | `passed` · `failed` · `blocked` · `not_run` | `AGENTS.md` |
| **Fact-graph data vocabularies** | **data inside the product**, not claims about the design | `origin`, `fidelity`, coverage status, type role | DESIGN §3.5 |

These are never interchangeable:
- A check outcome is never `Measured`, and a design claim is never `not_run`.
- `complete_under_stated_model` is a value in a table, not evidence that the design is complete.
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
