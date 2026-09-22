# Design Review — Reference

Companion to `.claude/skills/design-review/SKILL.md`. §1 is a lookup table. Everything after it is a set of lenses and calibration examples — angles that have turned up real defects before, offered because they are useful, not because the review owes them. The normative standard is [`DATA_MODEL_DESIGN_CHARTER.md`](DATA_MODEL_DESIGN_CHARTER.md); the repo-specific mapping is [`ADDENDUM.md`](ADDENDUM.md).

---

## §1 Principle index (DM-01–DM-60)

Titles and requirement levels, for accurate citation and for gate reasoning. A MUST-level gap on in-scope behavior cannot be waived by an exception record — it narrows the supported scope or the design is unresolved against that requirement. A SHOULD-level deviation can be accepted with a §10 record.

### Group 1 — Semantic authority and the modeling boundary

| ID | Level | Title |
|---|---|---|
| DM-01 | MUST | Make meaning—not storage shape—the primary model |
| DM-02 | MUST | Assign one authority to each semantic fact and revision |
| DM-03 | MUST | Unify logical contracts without mandating one physical structure |
| DM-04 | MUST | Declare the semantic boundary and expose opaque behavior |
| DM-05 | SHOULD | Separate intent from mechanisms and incidental technology |

### Group 2 — Semantic types, schemas, and invariants

| ID | Level | Title |
|---|---|---|
| DM-06 | MUST | Type semantic distinctions, not only machine representations |
| DM-07 | MUST | Make validity rules explicit and enforce them at identified boundaries |
| DM-08 | MUST | Represent absence, unknowns, uncertainty, invalidity, and failure distinctly |
| DM-09 | MUST | Model relationships and valid domains explicitly |
| DM-10 | SHOULD | Keep important structure typed and queryable |

### Group 3 — Identity, versions, and consistency

| ID | Level | Title |
|---|---|---|
| DM-11 | MUST | Use stable semantic identity independent of physical location |
| DM-12 | MUST | Distinguish entity, revision, artifact, and execution identity |
| DM-13 | MUST | Separate definitions, specifications, policies, observations, and results |
| DM-14 | MUST | Publish semantically consistent revisions through explicit commit boundaries |
| DM-15 | MUST | Define canonicalization and equivalence before using content identity |

### Group 4 — Declarative composition and reusable structure

| ID | Level | Title |
|---|---|---|
| DM-16 | SHOULD | Represent material structure and policy as declarations |
| DM-17 | SHOULD | Use templates and bindings instead of copied construction logic |
| DM-18 | SHOULD | Preserve high-level structure until expansion is required |
| DM-19 | MUST | Select behavior through declared capabilities and explicit bindings |
| DM-20 | MUST | Make inspection and validation semantically non-mutating |

### Group 5 — Compilation, derivation, and semantic preservation

| ID | Level | Title |
|---|---|---|
| DM-21 | SHOULD | Use explicit intermediate representations and progressive lowering |
| DM-22 | MUST | Give every meaningful transformation a contract |
| DM-23 | MUST | Make derived representations traceable and non-competing |
| DM-24 | MUST | Preserve semantics across rewrites and lowerings |
| DM-25 | SHOULD | Unify operation contracts while allowing specialized implementations |

### Group 6 — Planning, execution, mutable state, and effects

| ID | Level | Title |
|---|---|---|
| DM-26 | SHOULD | Separate preparation from repeated execution |
| DM-27 | SHOULD | Represent important workflows as inspectable plans or state machines |
| DM-28 | MUST | Declare effects, ambient inputs, and nondeterminism |
| DM-29 | MUST | Isolate mutable workspaces and commit their outcomes explicitly |
| DM-30 | MUST | Make partial failure and recovery explicit |

### Group 7 — Dependencies, incrementality, and concurrency

| ID | Level | Title |
|---|---|---|
| DM-31 | MUST | Expose every dependency that can affect meaning or output |
| DM-32 | MUST | Key reuse to semantic dependencies rather than convenience |
| DM-33 | SHOULD | Invalidate at the smallest trustworthy semantic granularity |
| DM-34 | MUST | Keep different relationship structures semantically distinct |
| DM-35 | MUST | Make concurrency respect dependencies, ownership, and declared ordering |

### Group 8 — Execution representations and performance

| ID | Level | Title |
|---|---|---|
| DM-36 | SHOULD | Choose physical layouts for demonstrated access patterns |
| DM-37 | SHOULD | Cross expensive boundaries in coarse, typed units |
| DM-38 | SHOULD | Match the execution mechanism to the operation’s semantics |
| DM-39 | MUST | Evaluate performance end to end and distinguish evidence from expectation |
| DM-40 | MUST | Declare precision, approximation, ordering, and determinism requirements |

### Group 9 — Boundaries, providers, and extensibility

| ID | Level | Title |
|---|---|---|
| DM-41 | MUST | Keep adapters mechanical and domain conversions explicit |
| DM-42 | MUST | Make interchange loss-aware and reject silent semantic degradation |
| DM-43 | MUST | Negotiate capabilities and expose unsupported behavior |
| DM-44 | MUST | Make extensions complete, versioned, and conformance-testable |
| DM-45 | MUST | Treat trust and authority as explicit execution constraints |

### Group 10 — Provenance, reproducibility, and explainability

| ID | Level | Title |
|---|---|---|
| DM-46 | MUST | Preserve source-to-result lineage through transformations |
| DM-47 | MUST | Represent diagnostics as structured evidence |
| DM-48 | MUST | Define and support the required reproducibility contract |
| DM-49 | SHOULD | Make changes understandable at the level of meaning |
| DM-50 | SHOULD | Observe the model lifecycle, not only low-level operations |

### Group 11 — Evolution, generation, and verification

| ID | Level | Title |
|---|---|---|
| DM-51 | MUST | Evolve schemas and semantics through explicit migrations |
| DM-52 | SHOULD | Generate repeated mechanical artifacts from shared contracts |
| DM-53 | MUST | Verify invariants and equivalence across representations |
| DM-54 | MUST | Test adversarial lifecycle and boundary conditions |
| DM-55 | SHOULD | Make contracts and extension paths discoverable to humans and agents |

### Group 12 — Architectural leverage and disciplined improvement

| ID | Level | Title |
|---|---|---|
| DM-56 | SHOULD | Optimize for fewer independent semantic decisions—not fewer lines |
| DM-57 | SHOULD | Prefer a small coherent core with explicit extension mechanisms |
| DM-58 | SHOULD | Scale architectural machinery to demonstrated needs |
| DM-59 | MUST | Make design claims falsifiable and label uncertainty |
| DM-60 | MUST | Turn the principles into change-level review and regression controls |

### Scope-to-group routing

A starting orientation, not an allocation. It exists to help justify the §7 applicability note; the scope decides which groups actually matter.

| If the scope is… | Groups that often carry the findings |
|---|---|
| The fact model, a fact family, a codebook, or `cpg-schema` contracts (DESIGN §3, §B2) | 1, 2, 3, 11 |
| Identity encoding, the run contract, or `provider_node_map` (§3.4.1, §4.0, §4.1 C) | 3, 2, 10 |
| The Pyrefly/Ruff extraction driver, walker or mappers, or the binding recognizer (§4.2, §B1, §B8) | 9, 2, 10 |
| DataFusion construction plans or validators (§4.1 D, §8, §B3) | 5, 2, 8 |
| A graph projection or traversal (§5, §B4) | 7, 5, 8, 10 |
| Analytics: Passes A–C, community detection, centrality, FCA/RCA, analytic embeddings (§9) | 5, 8, 2, 12 |
| Synthesis: findings, assertions, briefs, grounding, usage patterns (§10, §B11) | 2, 5, 10, 11 |
| Delta publication, readers, or the serving generation (§6, §B7, §B12) | 3, 6, 7 |
| Serving and the agent interface: embedding spec, retrieval, FastMCP tools (§11, §B13, §B14) | 9, 7, 2, 8 |
| Evaluation, gold scoring, ablations (§12) | 10, 11, 12 |
| Execution semantics: CFG lowering, dataflow, aliasing (§B5, §13, deferred) | 5, 2, 8, 12 |
| A refactor claiming leverage or simplification | 12, 4, 11 |

---

## §2 Lenses that tend to pay off

Nothing here is a required step. These are the angles that have repeatedly turned up real defects, and the note on what it takes for each to hold up as evidence rather than as a suspicion.

### When the subject is a document

**Reconstructing beats reading.** Building the template's tables yourself from the document — rather than checking whether the document's own version looks complete — tends to surface the gaps quickly, because every cell you have to invent is a decision the design hasn't made:

- The **authority table** (§2): one row per semantic fact, with owner, revision boundary, and permitted update path. The list of cells you had to invent *is* the evidence for the finding.
- The **invariant table** (§3), with an enforcement boundary and a failure behavior per invariant. An invariant with neither is unresolved (DM-07), whatever the surrounding prose claims.
- The **stage graph** (§4), with inputs, output contract, effects, and invalidation per stage. Undeclared effects and ambient inputs land under DM-28.
- The **absence lattice** (DM-08): for each value that can be missing, which of unspecified / unknown / not-applicable / uncertain / invalid / partial / failed does the design distinguish, and which collapse into one representation? States that collapse when callers need to tell them apart are a G2 concern. In this repo the lattice is partly fixed by DESIGN §3.5's coverage statuses (`complete_under_stated_model`, `partial`, `not_requested`, `unavailable`, `failed`) and by resolution sets carrying `has_unresolved_remainder` — check that a design does not add an eighth state informally, or collapse two of them into a null.

**Sorting the load-bearing sentences** is a cheap way to keep claim strength honest:

| Kind | How it reads | What it deserves |
|---|---|---|
| Specification | States what holds, where enforced, what is rejected | Assess it directly |
| Intention | A desirable property with no mechanism — "the model is the single source of truth" | Unresolved until a mechanism is named |
| Assumption | Rests on an external system, library, or later decision | If not stated as an assumption, that's DM-59 |
| Benefit assertion | Performance, simplicity, extensibility | Hypothesis unless evidence is cited (DM-39, DM-59) — label it |

**The divergence sentence.** For a core mechanism, writing the one sentence two implementers would read differently is often the whole finding. Familiar shapes here: "IDs are deterministic" (over which length-delimited encoding, which structural occurrence path, which encoding version — DM-15, DESIGN §3.4); "the call is resolved" (a complete candidate set under a stated model, or known targets plus an unresolved remainder — §3.6, DM-08); "the type of `x`" (which of the fifteen type roles, at which program point — §3.5, DM-06); "the snapshot is consistent" (every table at the version its `snapshots` row names *and* filtered by `snapshot_id`, or each table's latest — §B7, DM-14); "the decoder preserves the provider's data" (every report field mapped or declared lost, or silently dropped — DM-42); "the brief says X" (a template over a `structurally_observed` finding, a verbatim documented sentence, or a `statistically_derived` guess — §10.2, DM-08, DM-59).

At `deep`, the counter-design is worth the time: the smallest alternative delivering the same observable outcome. If the proposal's extra machinery can't pay for itself against it, that's a DM-58 finding that usually outranks the local ones.

### When the subject is code

Six recurring defect shapes. The second column is what it takes for one to be reportable rather than suspected — below that bar, it belongs in the Method note as something you looked at and couldn't settle.

| Shape | What makes it evidence | Gate · principles |
|---|---|---|
| **Second authority** — the same semantic fact independently editable in two places (an Arrow `Schema` in `cpg-schema` and a hand-written column list in an adapter; a codebook and a string `match`; a canonical table and a convenience `HAS_TYPE` view materialized independently; a production validator and a test-only copy of the same SQL) | Both sites cited, plus the absence of a derivation or an assertion linking them. Two representations derived from one source are not a second authority — check for the derivation first (e.g. the adapter importing the table spec from `cpg-schema`) | G1 · DM-02, DM-23 |
| **Unguarded boundary** — an external input, partial construction, or deserialization reaching an operation whose correctness assumes an invariant | The entry point, the operation, and either no check between them or one that is advisory (logs, warns, opt-in) rather than rejecting. Type-level enforcement counts; say so and close it | G3 · DM-07, DM-22 |
| **Hidden effect** — `validate`, `check`, `inspect`, `plan`, `explain`, `search` paths that mutate, populate caches, lazily fetch, register, or read ambient state (clock, env, filesystem, network, global config) | The mutation or ambient read cited, plus a caller that reasonably assumes purity. A validator that repairs rather than rejects, or a read that triggers an adapter run, is the shape to look for here. Memoization that can't change observable semantics isn't a finding — but say why you concluded that | G4 · DM-20, DM-28 |
| **Silent degradation** — default-on-absence, catch-alls flattening distinct failure classes, an inner join silently dropping unmapped endpoints, a report field defaulted when the provider omitted it, a native value reduced to its `Display` string | The branch, what the consumer observes in the degraded versus supported case, and no declared loss, fidelity downgrade, or coverage row. `complete_under_stated_model` means complete *within the declared model*; a coverage row that hides an adapter gap is this shape | G2/G7 · DM-08, DM-42, DM-43 |
| **Incomplete reuse key** — a run, snapshot, or content-derived ID missing something that can change the result: analyzer revision, adapter build, Python environment and search paths, enabled extraction families, codebook or schema version, projection policy | The key construction, the omitted dependency, and a situation where changing it yields a reused result that is wrong. Volatile inputs in a deterministic ID — wall-clock time, tempdir paths, partition arrival order — are the mirror-image defect and also DM-32 | G6 · DM-31, DM-32 |
| **Unbacked capability** — the accepted-operation surface (a node or edge kind in the codebook, a fact family an adapter advertises, a DESIGN claim, a projection's declared edge kinds) wider than the implementation that produces it | The set difference, with the declared variant and the missing or fallback handling both cited. A coverage row saying `not_requested` or `unavailable`, or a check reporting `blocked` with its prerequisite named, is aligned; a silent fallback to different semantics is G7. The codebook entry is the claim, not the proof | G7 · DM-43, DM-44 |

Tracing the path that executes — the `impl` actually selected rather than the trait's doc comment, the branch taken under the real configuration — is what makes these hold. Where dispatch is dynamic or config-dependent, saying which path was traced and which wasn't keeps the coverage note honest.

### When the subject is both

Beyond what each half yields:

| Question | Finding shape |
|---|---|
| Do `DESIGN.md`'s semantics and the implementation's match? | Divergence against the pair; name which is authoritative and how they reconcile (DM-02) |
| Does `DESIGN.md` describe a state the code has passed through? | Stale specification — say whether the section is normative-forward (a target for a later phase) or descriptive (a record); one that is neither is a competing authority |
| Does `DESIGN.md` depart from `Initial_plan.md`? | The plan is research input, not authority, so departing is fine — but DESIGN.md or an ADR must say so. An unrecorded departure is a finding whose correction is a sentence or an ADR, never an edit to the research input |
| Does the code implement semantics `DESIGN.md` omits? | Undocumented surface; DM-04, DM-55 |
| Does the document claim capabilities the code lacks? | G7 — narrow the claim or relabel it Proposed |
| What evidence label does each design claim now deserve? | Re-label per charter §D. A Proposed claim becoming Implemented is often this mode's most useful output |

---

## §3 Finding calibration

Adequate and inadequate versions of the same observation. The difference is consistently the same three things: a concrete consequence, evidence at the right grain, and citations that do work. In this repo there is a fourth: the **Verification** column must name an oracle tier from [`ADDENDUM.md`](ADDENDUM.md) §4 or say that none exists. The examples below use plausible future paths and are illustrative.

### A — Authority

**Inadequate.** "Violates DM-02: the schema definition appears in multiple places, which creates a single-source-of-truth problem. Recommend consolidating."

No sites cited, no demonstration that the two can drift, no consequence, and "consolidating" isn't a direction with a surface area.

**Adequate.** (Illustrative paths.) "The `call_sites` column set is independently editable in two places: the `TableSpec` in `cpg-schema` (`tables/calls.rs:40`) and the Pysa decoder's batch builder (`crates/cpg-extract/src/pysa/emit.rs:112`), which re-declares the fields as literals instead of taking the spec's `SchemaRef`. No test compares them. **Consequence:** adding the nullable `branch_context_id` column to the spec makes every decoder batch fail `RecordBatch::try_new` at the validation boundary, or worse, if the decoder's copy is edited to match by hand, the next nullability change drifts silently. **Correction:** have the decoder build against the spec's schema (one call site). **Verification:** test — a decoder test asserting its output schema equals the spec's, including field metadata; it fails today. DM-02, DM-52, G1."

### B — Absence semantics

**Inadequate.** "DM-08 is not fully satisfied; the design should distinguish absence states more clearly."

**Adequate.** "A module Pyrefly never analyzed, a module whose Pysa report failed to decode, and a module with genuinely no calls all produce zero `CALL_TARGET` rows and no coverage row. **Consequence:** a consumer asking 'what does `f` call?' cannot separate 'nothing' from 'not extracted', so an empty result reads as proof of absence. **Correction:** emit a coverage row per (module, fact family) with `complete_under_stated_model`, `failed` or `not_requested`, a one-table change while the decoder has one consumer. **Verification:** test — a fixture with one failing report asserting the three cases are distinguishable; plus an `ast-grep` rule in `rules/` flagging a decoder `Err` arm that returns an empty batch. DM-08, DM-30, G2."

### C — Over-construction

**Inadequate.** Silence — or praise for extensibility. This is the finding that most often goes unwritten.

**Adequate.** "§X introduces a generic projection DSL with a planner that compiles declarations into DataFusion plans, but stage 1 has two projections (module dependency and source call), each a two-join query. The DSL adds a declaration surface, a compiler, and a conformance obligation (DM-44) nothing funds. **Consequence:** the third projection, the execution CFG, needs entry/exit policies the DSL can't express, so it gets rewritten rather than extended — the cost paid twice. **Correction:** keep the projection manifest (the cheap, justified declaration of kinds and policies) and write the two plans as ordinary functions; revisit the DSL when a fourth projection repeats their shape. **Verification:** none needed — this removes machinery. DM-58, DM-57, charter §F, 'speculative flexibility with no demonstrated consumer'."

### D — Evidence labels

**Inadequate.** "The design is validated end to end — see the integration tests."

**Adequate.** "Delta round-trip guarantee: **Tested** for values and types (the round-trip test, N fixture tables, `FixedSizeBinary(16)` ↔ `Binary` with width checks) and **Proposed** for schema metadata — nothing exercises whether the `cpg.*` field metadata DESIGN §3.3 relies on survives a Delta write and read. Either narrow the claim to value round-trip or add the metadata case before making it. DM-53, DM-59."

Note the three vocabularies stay separate: this is a charter §D label on a *design claim*. The check that backs it has an outcome (`passed`/`failed`/`blocked`/`not_run`), and neither is a fact-graph `fidelity` or coverage value. See [`ADDENDUM.md`](ADDENDUM.md).

---

## §4 How the sections tend to compress

Template sections are `DESIGN_REVIEW_TEMPLATE.md` §1–§11. A sketch of what usually survives, not a rule — the scope decides.

| Section | Document subject | Code subject | At `compact` |
|---|---|---|---|
| §1 Decision and scope | Plus Method and coverage | Plus Method and coverage | Compressed; Method note still earns its place |
| §2 Authority and lifecycle | Reconstructed; invented cells marked | Reconstructed; where each authority lives | Prose, with the authority gaps named |
| §3 Contracts and invariants | Usually the core of the review | Enforcement sites cited | Often merges into §7 |
| §4 Derivation and execution | Full | Tracing real paths | Prose |
| §5 Journeys | Extension and failure at minimum | Same, through real code | One journey, chosen for relevance |
| §6 Gates | Tabular | Tabular | Tabular |
| §7 Findings + applicability | Tabular | Tabular | Tabular |
| §8 Alternatives | Worth the work at standard/deep | Worth the work at standard/deep | Optional; say why omitted |
| §9 Verification plan | Proposed checks | Existing coverage and gaps, named | Top gaps only |
| §10 Exceptions | Only if deviations exist | Only if deviations exist | Only if deviations exist |
| §11 Decision and changes | Required | Required | Required |

---

## §5 Appendix — where these shapes tend to appear in this repository

Illustrative, and mostly forward-looking: at the time of writing only `crates/cpg-schema` and the scripts exist, so module and package names below (the reader module, `python/lctx_mcp`) are the planned ones. The charter is technology-neutral and so is the review; this lists places the shapes in §2 are likely to land given DESIGN.md, and their absence proves nothing. The right-hand column names the oracle tier (ADDENDUM §4) that would catch a regression.

| Pattern | Shape | Principles · gate | Likely oracle |
|---|---|---|---|
| An extractor or decoder re-declaring a table's columns instead of using the `cpg-schema` spec | Second authority | DM-02, DM-52 · G1 | test: extractor output schema equals the spec, metadata included |
| A pass writing `nodes`/`edges` directly instead of its family table, so the derived view and the family disagree | Second authority | DM-02, DM-23 · G1 | test: regenerated views equal the stored ones on a fixture; `ast-grep` rule on writes to view tables |
| A validator's SQL duplicated in a test instead of calling the shared library function | Second authority | DM-02, DM-53 · G1 | `ast-grep` rule flagging inline validation SQL in tests |
| A `HAS_TYPE` edge materialized as its own mutable table instead of a view over `type_observations` | Second authority | DM-23 · G1 | test: view and observations agree on a fixture |
| An assertion citing a finding or evidence id from another snapshot, or a public symbol absent from `exports` | Ungrounded claim | DM-07, DM-46 · G1/G3 | test: the §10.4 grounding validator on a fixture with a stale id |
| A codebook code reassigned or reordered | Silent migration | DM-51 · G1/G2 | test: codebook append-only snapshot |
| A schema change without a reviewed snapshot diff | Silent migration | DM-51 · G2 | test: insta schema snapshots under `INSTA_UPDATE=no` |
| An inner join in stage C/D dropping unmapped provider endpoints | Silent degradation | DM-08, DM-42 · G2 | test: fixture with an unresolvable target yields a `boundaries` row |
| A module an extractor never reached having no `coverage` row | Silent degradation | DM-08, DM-30 · G2 | validator: coverage completeness per declared family × module |
| A Pyrefly type kept only as its display string where Pysa gave structure | Silent degradation | DM-42, DM-10 · G2 | test: `fidelity` is `display_only` whenever structure is absent |
| Pysa `ifCalled` targets emitted as call targets, or `artificial-call` sites without `synthetic_model` origin | Silent reinterpretation | DM-13, DM-24 · G2 | test on a higher-order-call fixture |
| Glean caller→callee pairs used for `calls` (they drop unresolved calls) | Silent degradation | DM-08, DM-42 · G2 | test: an unresolved call on a fixture yields a `resolutions` row with a reason |
| A Pass B guard or forwarding site promoted despite an earlier binding of the parameter's name | Silent reinterpretation | DM-24, DM-59 · G2 | test: the "guard after parameter rebinding" fixture yields `ambiguous_binding` |
| A template emitting a control, limit or behavioral claim from a `statistically_derived` finding | Unbacked claim | DM-08, DM-59 · G2/G7 | test over the assertion builder; validator query rejecting statistical status on control/limit assertions |
| A brief's conditions or limits hydrated by a second semantic search instead of deterministic joins on `(snapshot_id, brief_id)` | Optional warning | DM-08, DM-23 · G2 | test: every limit of a selected brief is returned regardless of query wording |
| A deterministic ID built from a span alone, or including a tempdir path or timestamp | Incomplete / volatile key | DM-15, DM-32 · G6 | proptest: same-range syntax nodes get distinct IDs; IDs stable across two runs |
| A run identity missing the analyzer revision, adapter build, search paths or config digests | Incomplete reuse key | DM-31, DM-48 · G6 | test: changing one input changes `run_id` |
| Community detection, FCA or witness selection depending on input row order, an unrecorded seed, or an unpinned `rand` | Nondeterminism | DM-28, DM-40 · G6 | test: shuffled-input fixture gives byte-identical findings |
| A traversal relying on `Bfs`/`Dfs` sibling order or `edges_directed` order instead of sorting by canonical key | Nondeterminism | DM-40 · G6 | test: same projection built in two insertion orders yields identical witnesses |
| Dense petgraph indices persisted as, or joined to, canonical IDs | Leaked temporary coordinate | DM-11, DM-40 · G6 | `ast-grep` rule on projection-index writes outside the projection module |
| A parallel-arc collapse that drops the arc-to-fact mapping | Lost lineage | DM-46 · G6 | test on a parallel-arcs fixture |
| A query-time embedding produced or accepted without checking the generation's `spec_hash`, or a cached vector reused after the spec changed | Invalid reuse | DM-31, DM-32 · G6 | test: conformance vectors across the Rust and Python clients; a spec-mismatch fixture is rejected |
| A reader loading a Delta table at "latest", building a provider on an already-loaded handle, or omitting the `snapshot_id` filter | Inconsistent revision | DM-14 · G5 | `ast-grep` rule on `DeltaTable` loads outside the reader module; test with an unpublished attempt present |
| A `snapshots` row appended before validation passes, or an attempt retried under the same `snapshot_id` after a write error | Unguarded commit | DM-14, DM-30 · G5 | test: an injected validation or write failure publishes nothing |
| The MCP server swapping generations mid-process, or a generation manifest not naming its snapshot | Inconsistent revision | DM-12, DM-14 · G5 | test: `Client(mcp)` sees one generation key across calls; manifest schema test |
| A raw Parquet directory scan of a Delta table | Hidden semantics change | DM-20, DM-24 · G4 | `ast-grep` rule on `read_parquet` over table paths |
| An analyzer run that discovers its own config or reads an ambient environment | Hidden input | DM-28, DM-31 · G4 | test: the Pyrefly invocation carries an explicit config; `context_id` changes when the config changes |
| Gold-reference paths (`.claude/skills/**`) read by the compiler, or analytics parameters tuned against the gold | Hidden input; circular evaluation | DM-28, DM-59 · G4 | `ast-grep` rule on skill paths in compiler code; test that the acquisition manifest lists no skill paths |
| `unwrap` / `expect` in a report decoder or at a bundle or IPC boundary | Unguarded boundary | DM-07, DM-42 · G3 | test with a malformed report; clippy |
| A FastMCP tool returning a bare list, or anything printing to stdout under stdio | Protocol degradation | DM-42 · G2 | test: `Client(mcp)` asserts object `structured_content` in both protocol modes; `ast-grep` rule on `print(` in `python/lctx_mcp` |
| A second Arrow, DataFusion or delta-rs version entering the lockfile | Dependency drift | DM-31 · G6 | `just deps` (exists) |
| An extractor advertising a fact family it only partly extracts | Unbacked capability | DM-43, DM-44 · G7 | test: coverage rows per family on the fixture corpus |
| An analytic technique kept although its ablation changes no published output | Unearned machinery | DM-58 · — | the §9.8 ablation diff |
| A check reported `passed` without its command having run, or a mocked provider counted as a pass | Unbacked capability | DM-59 · G7 | prose (`AGENTS.md`); no mechanical oracle |
| A benchmark measuring one stage while the claim is end-to-end | — | DM-39, DM-59 | name the conditions, or relabel the claim |
| A finding with no oracle at any tier | — | DM-60 | **say so.** That absence is the most valuable output; it becomes a new test or `rules/` entry |
