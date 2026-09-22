# Design review: design spine baseline (compact)

**Date:** 2026-09-22 · **Depth:** compact · **Mode:** document-stage, plus the placeholder code
**Reviewer:** `design-reviewer` subagent (fresh context) · **Author:** the seeding session (ADR-0001..0003)

## 1. Decision and scope

**Proposal:** `docs/design/DESIGN.md` as seeded on 2026-09-22 from `docs/initial_plan/Initial_plan.md`,
with ADR-0002 (dependency family) and ADR-0003 (workspace topology, `proposed`), plus the only
code, `crates/cpg-schema` (an empty `lib.rs` and `tests/family_smoke.rs`).

**Status:** every DESIGN claim is **Proposed** by the file's own default (DESIGN L9-13), except
§7, which is labelled **Tested**.

**Observable outcome being judged:** is the spine *decidable* for increment 1 (the fact substrate,
DESIGN §1.2 item 1)? That is, would two competent implementers reading only DESIGN.md build the
same semantics for the schemas, codebooks, identity mapping, validation and Delta publication?

**Baseline:** there is nothing to regress from. No table family exists, `cpg-schema` has no
schema (`src/lib.rs` is doc comments only), and there are no adapters (`adapters/README.md`).

**Supported scope and non-goals:** the scope is increment 1 only. Projections and topology
(§5, increment 2) and execution semantics (§B5, increment 3) were read for consistency but not
judged for decidability. The non-goals are DESIGN §1.3.

**Constraints:** single operator; proportionality per ADR-0001 and DM-58. The spine uses 272 of
its 600-line budget, so it has room for every correction below without an ADR per sentence.

### Method and coverage

- **Read in full:** DESIGN.md, ADR-0001/0002/0003, `crates/cpg-schema/{Cargo.toml,src/lib.rs,tests/family_smoke.rs}`,
  the workspace `Cargo.toml`, `justfile`, `scripts/check_family.py`, `docs/pins.md`, `STATUS.md`
  and `adapters/README.md`.
- **Read at section grain in `Initial_plan.md`:** Conclusion, §1, §2.3, §3.7-3.8, §4.1-4.7, §5,
  "Recommendation", and part two §1-§3 (Stages A-E), §7.1-7.4 and the implementation sequence.
  Not read: part one §2.1-2.2, §2.5, §3.1-3.6 and §6.2, and part two §4-§6 (petgraph/CFG,
  increments 2-3).
- **Condensation check:** for each increment-1 rule in the plan, I looked for its counterpart in
  DESIGN.md. Dropped rules are recorded in F2. Omissions of plan material for increments 2-3 were
  not assessed.
- **Ran:** `just test` → **passed**. `cargo nextest run --workspace`: 2 tests, 2 passed,
  0 skipped (`delta_write_then_datafusion_query_through_provider`, `fixed_size_binary_ids_are_available`).
- **Not run:** `just deps`, `just check`, `just test-all`. DESIGN §7's claim that "`just deps`
  checks for single versions" is therefore **asserted** here. I read `check_family.py`, whose
  shape matches the claim (Interface-checked), but did not execute it.
- **Not available:** the plan's "53-table / 94-edge-kind" JSON specification and its
  "66-schema reference package" are not in the repository (STATUS.md already says so). Every
  statement below about what they contained rests only on the plan's prose.
- **Attacked:**
  - interrupted or failed publication against the manifest (F3);
  - a rerun under a new analyzer revision reusing IDs (F4);
  - unmapped endpoints under the "never an inner join" rule (F5);
  - an adapter partly extracting a family (F5);
  - Pyrefly inference being relabelled as dataflow (§B5 and §3.6 close this; not a finding).
- **Not attacked, so asserted:**
  - `record_fields` "loss-free" preservation;
  - `Require::Everything` retention being sufficient;
  - the Ruff adapter being buildable at all, which depends on a Ruff hook (F2);
  - the MSRV and toolchain claims.

## 2. Authority and lifecycle (reconstructed)

Reconstructing the authority table from DESIGN.md alone gives the following. Cells marked
*invented* are ones the spine does not decide.

| Concept | Authority named | Revision boundary | Gap |
|---|---|---|---|
| Table columns, types, nullability | `cpg-schema` (§B2, §3.2) | *invented*: nothing ties a schema version to a snapshot | the crate is empty; the only concrete column lists are in the research input, whose two parts disagree (L469ff `edge_kind: utf8` vs L1066ff `Int16`) (F1) |
| Category codebooks | `cpg-schema`, append-only (§B2, §3.4) | *invented* | `extraction_mode`, `modality`, `model`, resolution `status`/`domain` have no values anywhere in the repo (F1) |
| Edge kinds and permitted endpoints | `cpg-schema` (§B2) | *invented* | the plan's 94-kind registry is absent; DESIGN names about 8 kinds (F1) |
| Run and environment | `runs`, `contexts`, `producers` tables (§4.1 A) | *invented* | what a run's identity comprises is undefined (F4) |
| Node and fact identity | "deterministic, versioned, length-delimited" (§3.4) | *invented*: snapshot-scoped or cross-snapshot? | inputs per ID kind undeclared (F4) |
| Published snapshot | manifest (§B7, §6.1) | manifest | manifest storage, atomicity, and scoping of rows within shared family tables undefined (F3) |
| Coverage and resolution issues | vocabulary only (§3.5, §4.1) | none | no table, no grain (F5) |

**Identity behaviour:** DESIGN §3.4 answers rename and duplication well: a qualified name is a
label, and a span is not an ID. It leaves regeneration and rerun open (F4).

## 3. Contracts and invariants

These are merged into §7 at compact depth. What the spine *does* decide well:

- two validation levels, each with a named enforcement boundary and a rejection outcome (§8);
- "never dropped by an inner join" (§4.1);
- Delta reads through the log, not by Parquet directory scan (§6.1);
- checked conversions in both directions at the Delta boundary (§3.3);
- `ifCalled` becomes `POTENTIAL_CALL_TARGET`, and synthetic shims carry `synthetic_model` (§3.6);
- Pyrefly inference is not dataflow (§B5).

Each of these would break without the sentence that states it, because the plan argues each
against an obvious wrong default. The spine kept them.

## 4. Derivation and execution

The stage table (§4.1 A-E) keeps the plan's ownership split, and its outputs are named. For
increment 1, three stage contracts are missing:

- **Stage A:** what a run and a context record, which is the plan's environment manifest with
  ordered search paths (F4).
- **Stage C:** where unmapped or ambiguous rows go (F5).
- **Publication:** the protocol after validation (F3).

Effects and ambient inputs are declared nowhere. Analyzer config discovery and the Python
interpreter and search paths are ambient inputs to both adapters (F4).

## 5. Journey: interrupted or failed publication

1. Snapshot S2's Pyrefly adapter succeeds, and its facts are appended to the relation-family
   Delta tables (§6.1: batched across modules, keyed by `snapshot_id`).
2. The `edges` Delta table advances to version 7.
3. Cross-table validation (§8) then finds dangling `dst_id`s, so no manifest is written for S2.

The next snapshot, S3, appends and advances `edges` to version 8, validates, and publishes a
manifest naming `edges@8`. Version 8 still contains S2's invalid rows. DESIGN §B7 guarantees
that readers use the manifest's *version*, but it never says the reader also scopes to the
manifest's `snapshot_id`, or that failed rows are removed. One implementer reads `edges@8`
whole and gets S2's dangling edges. Another filters by `snapshot_id` and is correct.

§8 also doesn't say whether validation runs on in-memory batches before any Delta commit, or on
committed versions. Nor does it say whether an adapter crash, which ADR-0003 says is "contained",
yields an unpublished snapshot or a published one with `failed` coverage rows. **G5 is
unresolved** (F3).

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 Authority | **unresolved** | Declared correctly: `cpg-schema` is the single authority, and ADR-0003 has adapters depend on it by path. What the authority contains is undecided: the crate is empty, and the research input holds two conflicting column profiles (F1). Question 1 (trace a fact to its revision, environment and run) has no run contract (F4). The plan's rule that `HAS_TYPE` is a non-competing view over `type_observations` was dropped (F2b). | Decide the F1 contents in slice 1; restore F2b; define the run contract (F4). |
| G2 Semantic fidelity | **unresolved** | The binding `facts` columns `extraction_mode`, `modality` and `model` (§B6) have no domain (F1). Coverage has a vocabulary but no table or grain (F5). The §3.3 physical profile dropped the plan's "nullable Boolean: unknown ≠ false" row, which the `ResolutionSet` flags depend on (F2i). The coordinate-normalization rule was dropped (F2a). Question 4 (inference relabelled as dataflow) **is** closed by §B5 and §3.6. | F1, F2 and F5 corrections. |
| G3 Validity | **pass** (Proposed) | Every invariant class has a named boundary and a rejection outcome: local checks at every materialization boundary, `provider_node_map` uniqueness "checked before use" (§4.1 C), and cross-table checks gating publication (§8). *What* counts as valid depends on the F1 declarations, which is a G1/G2 matter, not a missing enforcement path. | none beyond F1 |
| G4 Hidden behavior | **unresolved** | Both analyzers read ambient state: config discovery, interpreter and site-packages, search paths. DESIGN says "environment-qualified" (§1.1) but never requires these to be explicit, recorded inputs (F4). Whether validators only reject or may repair is implied by §8 ("publishes only when both pass") but not stated. | F4, plus one sentence in §8 that validators are read-only. |
| G5 Consistency and recovery | **unresolved** | See §5: rows of a failed snapshot persist in later family-table versions; the manifest's storage and atomicity are undefined; crash versus `failed` coverage is undecided (F3). | One ADR on the publication protocol (F3). |
| G6 Transformation and reuse | **unresolved** | The Delta-boundary conversion is decided (§3.3). ID derivation inputs are not, so a rerun under a new analyzer revision can reuse or collide `fact_id`s, or node IDs can be snapshot-scoped and so unusable across snapshots (F4). Projections are out of scope (increment 2). | F4 |
| G7 Truthful capability claims | **pass** | The only non-Proposed claim is §7 **Tested**. `just test` → passed (2/2) in this session. The claim is scoped to "write Delta and query through DataFusion", which is what `delta_write_then_datafusion_query_through_provider` does, on an `Int64` column. `FixedSizeBinary` is only constructed in memory, and the test comment says so. The "`just deps` single versions" sub-claim was not run here (asserted, Interface-checked by reading `check_family.py`). Everything else carries the file-level Proposed label. | none |

## 7. Findings

These are in severity order. F1-F5 are correctness or authority gaps on in-scope behavior. F6 is
extension locality. O1 and O2 are observations.

| # | Finding | Principle IDs | Evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| F1 | **The spine hands column-level authority to an empty crate, and the only concrete source is research input that contradicts itself.** Several binding columns have no domain anywhere. | DM-02, DM-06, DM-09, DM-59 · G1, G2 | DESIGN §3.2: "Column-level contracts live in `cpg-schema`, not here." `crates/cpg-schema/src/lib.rs` is empty. Initial_plan L469ff declares `edges.edge_kind: utf8`; L1066ff declares `Int16`, "a versioned physical migration". DESIGN §3.3 follows the second without saying the first is superseded. §B6 names `extraction_mode`, `modality` and "model" as required `facts` columns; §3.5 gives no values for them, nor for resolution `status` and `domain` (§3.6). The 94-kind edge registry and endpoint table that §B2 and §8 validate against are absent from the repo. | Two implementers of slice 1 (`nodes`/`facts`/`edges`/`spans`) choose different `extraction_mode`/`modality` domains, or `model_id` as free `Utf8` versus a codebook. Once data is persisted, the append-only codebook rule (§3.4) freezes whichever came first. Endpoint-kind validation (§8) cannot be written at all, so it gets skipped, silently. | For slice 1, make the `cpg-schema` declarations the answer, and add one line to DESIGN §3.2 naming which plan listing seeds them (part two, L1066ff) and that the part-one `Utf8` profile is superseded. Add the missing vocabularies (`extraction_mode`, `modality`, resolution `status`/`domain`, what "model" identifies) to §3.5: a few table rows. Declare only the edge kinds increment 1 emits; do not recreate all 94. | **None exists today.** Oracle to add: test, insta snapshots of each `Schema` including field metadata under `INSTA_UPDATE=no`, plus a codebook append-only snapshot test in `cpg-schema`. |
| F2 | **The condensation dropped increment-1 normative rules from the research input without recording the departure.** Each instance is a sentence two implementers would read differently. | DM-24, DM-42, DM-23, DM-40, DM-59 · G1, G2 | The dropped rules, each absent from DESIGN §3-§4, are listed after this table as F2a-F2i. | Case (a): Glean spans on a file containing `é` attach Pyrefly facts to the wrong `SyntaxNode`, or none, with no error. Case (b): a type answer becomes editable in two tables. Case (c): Ruff and Glean asserting the same binding get merged, losing disagreement. Case (e): computed types get labelled `annotation`. | Restore (a)-(g) and (i) as about 12 lines across §3.3, §3.4, §4.1 and §4.2. Record (h) as an explicit assumption in §4.2; it also bears on the pending analyzer-revision ADR. This is a sentence-level fix, not an ADR, except that (h) belongs in that revision ADR. | **None exists today.** Oracles to add as the slices land: tests on the plan's L1599 fixtures (non-ASCII coordinates, same-range syntax nodes, distinct binders named `T`, known-plus-unknown targets); a test that `HAS_TYPE` agrees with `type_observations`. |
| F3 | **The publication protocol is not decidable.** The manifest's storage, how atomic it is, and how rows of failed snapshots are scoped out of shared family tables are all undefined. | DM-14, DM-29, DM-30 · G5 | §B7 and §6.1 say the manifest names "the exact Delta version of every table" and is written after validation, and that tables are shared across snapshots (keyed by `snapshot_id`). Absent: where the manifest lives; whether validation runs before or after the Delta commit; whether readers also filter by the manifest's `snapshot_id`; what happens to rows of a failed snapshot; whether an adapter crash means no snapshot, or a published one with `failed` coverage. | §5 journey: a reader of a valid manifest sees a failed snapshot's dangling edges. Under a crash, one implementer publishes a partial snapshot with no coverage rows, which a reader cannot distinguish from a complete one. | One ADR (a real choice between alternatives, refining §B7). §8's simpler row is the suggested answer: an append-only `snapshots` Delta table whose single commit *is* publication, holding `(snapshot_id, table, version, schema digest)`. Readers always filter by that `snapshot_id`. A failed snapshot gets no row, and its orphan rows are harmless. Crashes become a published snapshot with `failed` coverage rows for the affected families, or no snapshot: pick one. | **None exists today.** Oracle to add: test, an injected validation failure leaves no `snapshots` row, and a reader of the next published snapshot sees zero rows with the failed `snapshot_id`. Later: an `ast-grep` rule on direct `DeltaTable` loads outside the manifest reader (REVIEW_REFERENCE §5). |
| F4 | **The run and ID-derivation contract is undeclared.** What a run's identity includes, which inputs each ID kind hashes, and whether node IDs are stable across snapshots are all open. | DM-12, DM-15, DM-28, DM-31, DM-32, DM-48 · G1, G4, G6 | Initial_plan L536: "supporting tables connect `run_id` to the producer revision, adapter build, Python environment, import/search-path configuration, and enabled extraction families". Stage A (L1146ff) says to preserve ordered search paths. DESIGN §4.1 lists `contexts`/`producers`/`runs` but keeps none of this. §3.4 says "deterministic IDs … length-delimited encoding" without the inputs per ID kind (node, fact, run, snapshot) or whether `snapshot_id` is among them. Analyzer config discovery is not declared as an input. | A rerun with a new Pyrefly revision but the same `(module, local key)` yields the same `fact_id` under one reading, silently replacing old-revision assertions. Under another reading, node IDs include `snapshot_id`, and no cross-snapshot comparison (DM-49) is ever possible. A `pyproject.toml` found by upward search changes Pyrefly's answers with no recorded difference in `run`. | Add a short §3.4.1 listing each ID kind's derivation inputs, and a `run` contract: analyzer revision, adapter build, interpreter and ordered search paths, config file digests, enabled families. Require adapters to take config and environment only as explicit arguments recorded in `contexts`. | **None exists today.** Oracles to add: test, changing any one run input changes `run_id`; proptest, same inputs twice give identical node IDs, and same-range syntax nodes get distinct IDs. |
| F5 | **Absence and issue destinations are declared as vocabulary with no table or grain,** so the "never dropped" rule has nowhere to put what it keeps. | DM-08, DM-30, DM-43 · G2, G7 | §4.1: unmapped rows "become resolution or coverage issues". §3.5: "recorded separately". But §3.2's table list has no coverage table and no `resolution_issues`, which the plan lists at L1111. The plan's grain, "by fact family and module/function" (§4.7), was dropped. "Fact family" is undefined, and differs from §6.1's "relation family". | Pyrefly never analyzes module M, or its Pysa report fails to decode, or M has no calls: all three give zero `CALL_TARGET` rows. A consumer takes the empty result as absence (Question 3). An adapter that implements half of a family reports nothing that says so (Question 8). | Add `coverage(snapshot_id, run_id, module_id, fact_family, status)` and `resolution_issues` to §3.2, and define "fact family" in one line. This is the smallest shape that separates the cases; function-level grain can wait for a consumer. | **None exists today.** Oracle to add: test, a fixture with one failing report and one call-free module gives distinguishable coverage rows, and an unmapped endpoint gives an issue row rather than disappearing. |
| F6 | **Where the validators and batch builders live is undeclared, and one natural placement breaks ADR-0003's isolation.** | DM-57, DM-31 · G6 (no gate failure today) | §B2's list of `cpg-schema` contents dropped the plan's "validation rules" and "batch builders" (L977ff). §B3 and §8 say validators are "library code" but name no crate. ADR-0003 limits adapters to `arrow-*` plus `cpg-schema`. | If validators land in `cpg-schema`, as the plan suggests, it gains a `datafusion` dependency, and every adapter workspace must then resolve DataFusion next to Ruff or Pyrefly: the conflict ADR-0003 exists to avoid. | One sentence in §B2 or §8 placing DataFusion validators in a core-only crate, and batch builders (Arrow-only) in `cpg-schema`. | Oracle to add: `just deps` (`check_family.py`), extended to assert that `cpg-schema`'s normal dependencies contain no `datafusion`/`deltalake`. It currently checks only version uniqueness. |
| O1 | Observation: §B8 reads as settled, but its deciding ADR-0003 is `proposed`. | DM-59 | DESIGN L91-97 against ADR-0003 `status: proposed` | A later session treats the adapter topology as fixed and skips the ADR's own revisit test. | Add "(provisional, ADR-0003 proposed)" to the §B8 heading. | No mechanical oracle; prose. |
| O2 | Observation: the workspace `Cargo.toml` comment says "`just deps` enforces that through deny.toml". ADR-0002 says the family rule lives in `check_family.py` *because* cargo-deny misses dev-only duplicates. | DM-02 (documentation only) | `Cargo.toml` `[workspace.dependencies]` header comment | A future pin change trusts `deny.toml` and weakens `check_family.py`. | Fix the comment. | No mechanical oracle; prose. |

**F2 instances.** Each rule below is in the research input and absent from DESIGN §3-§4:

- **(a) Source coordinates.** Coordinates need normalization: keep the exact UTF-8 parser input
  and explicit source maps (plan §4.7). DESIGN keeps `source_maps` as a Stage A table name only.
- **(b) `HAS_TYPE`.** It is a view over `type_observations` and "should not become a separately
  mutable competing source of truth" (L1257ff).
- **(c) Deduplication.** Deduplicate repeated ingestion, not independent evidence (L1282). §B6
  keeps the second half; the first half has no dedup key.
- **(d) Omitted report fields.** An omitted field follows the report's own default and is not
  "unknown" (L1186).
- **(e) Annotations.** Annotation-role types come from syntax and native annotation, not from TSP
  `getDeclaredType`, which returns computed types (L416-419).
- **(f) Type variables.** They keep binder identity: two unrelated `T`s are distinct (L621).
- **(g) MRO.** Pyrefly's reported MRO order is kept, not re-derived by topological sort (L1288).
- **(h) The Ruff hook.** Ruff's `Checker` is crate-private, so the adapter needs a narrow hook
  inside Ruff (L156-166). §4.2 states the timing but not that a patch or fork is assumed.
- **(i) Physical profile rows.** §3.3 dropped the `Utf8` row and the "nullable Boolean, unknown ≠
  false" row (L994ff). It also dropped the Delta-boundary line for authoritative schema metadata
  (L1603ff).

**Applicability.**

- **Bore on this scope:** groups 1-3 (authority, types, identity); 6 (publication and failure);
  7 (dependencies and reuse keys); 9 (adapter boundary); 10 (lineage and reproducibility);
  11 (schema evolution and verification); 12 (proportionality).
- **Group 4 (declarative composition):** only DM-20 bore, through G4. No template or binding
  mechanism is proposed for increment 1, and none is needed.
- **Group 5 (derivation):** bore only through DM-23/DM-24 in F2. Construction plans don't exist
  yet.
- **Group 8 (performance):** not applicable. The spine makes no performance claim. §4.1's
  batching and §6.1's "never one table per module" are layout choices without a quantitative
  claim, so DM-39 has nothing to test.
- **Increments 2-3 material (§5, §B4, §B5):** read for consistency only.

**Over-construction check.** I found no machinery without a consumer. The spine excludes the
usual suspects (§B10). It adopts neither the plan's 66 tables nor its 94-kind registry. It has no
projection DSL, and `record_fields` is a declared loss-free fallback with a DM-42 consumer. The
proportionality risk for increment 1 is breadth: §3.2 lists seven families of dedicated tables
before any fixture query needs them. F1's correction ("declare only what increment 1 emits")
covers it.

## 8. Alternatives

| Alternative | Duplication and locality | Risk | Cost | Selected? |
|---|---|---|---|---|
| Current baseline | Nothing implemented | n/a | n/a | n/a |
| DESIGN as written | One authority designated; content undecided | F1-F5 leave divergent readings | Low now; high once codebooks are persisted | Keep the direction |
| **Simpler viable: decide by construction in slice 1** | `cpg-schema` holds only `nodes`/`facts`/`edges`/`spans`/`record_fields`/`coverage`/`resolution_issues` and the codebooks increment 1 emits. Publication is an append-only `snapshots` Delta table whose commit is the manifest, with every read filtered by `snapshot_id`. | Uses Delta's single-table atomic commit rather than a bespoke manifest-file protocol, so no new component. Dedicated tables (signatures, classes, …) are added when a fixture query needs them. | About 30 DESIGN lines, one ADR (F3), schema snapshot tests | **Recommended** |

**What remains ordinary code:** the identity encoding, the report decoders and the validators.
None of them warrants a generator or DSL at this scale.

## 9. Verification plan (top gaps)

| Claim or risk | Label | Oracle | Current result |
|---|---|---|---|
| Family links; Delta write → DataFusion query through the provider | Tested | `just test` (`family_smoke`, 2 tests) | passed, 2026-09-22, this session |
| Single-version family | Interface-checked | `just deps` | not_run in this review |
| `FixedSizeBinary(16)` ↔ `Binary` at the Delta boundary | Proposed | a round-trip test with width rejection | none exists (the smoke test comment defers it to increment 1) |
| Schema and codebook stability | Proposed | insta snapshots plus an append-only codebook test | none exists (F1) |
| Failed snapshot invisible to readers | Proposed | injected-failure publication test | none exists (F3) |
| Run and ID derivation completeness | Proposed | `run_id` sensitivity test, ID proptest | none exists (F4) |
| Absence distinguishable | Proposed | coverage and issue fixture test | none exists (F5) |

## 10. Exceptions

None. No SHOULD deviation is claimed.

## 11. Decision

**Decision: Not Accept, as a decidable specification for increment 1.** The spine's direction
(§B1-§B10, the stage split, the validation levels) is sound and should stand.

**Reason:** G1, G2, G4, G5 and G6 are unresolved on in-scope behavior. G3 and G7 pass. None of
the gaps is a defect in a decision that was made. Each is a decision not yet made, or one that
was dropped in condensation. The author has to decide:

1. the slice-1 column and codebook contents, including the `extraction_mode`/`modality`/`model`
   domains (F1);
2. the publication protocol (F3, one ADR);
3. the run contract and the ID-derivation inputs (F4);
4. the coverage and issue tables and their grain (F5).

The F2 rules only need restoring.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 | Publication protocol ADR, refining §B7 | DM-14, DM-30 | the ADR, plus a DESIGN §6 update | injected-failure test |
| 1 | Slice-1 schemas and vocabularies in `cpg-schema`; §3.2 and §3.5 lines | DM-02, DM-06 | schemas exist | insta snapshots, codebook append-only test |
| 1 | Run and ID-derivation contract (§3.4.1) | DM-15, DM-31, DM-48 | DESIGN lines | `run_id` sensitivity test, ID proptest |
| 2 | Coverage and `resolution_issues` tables | DM-08, DM-30 | DESIGN §3.2 | coverage fixture test |
| 2 | Restore F2a-F2i (about 12 lines) | DM-23, DM-24, DM-42 | DESIGN §3-§4 | the L1599 fixture tests as slices land |
| 3 | Place validators and builders (F6); O1, O2 wording | DM-57 | DESIGN §B2 or §8 | `check_family.py` dependency assertion |

### Deferred

| Item | Why deferred | Reopen when |
|---|---|---|
| Function-level coverage grain | No consumer yet | An agent query needs per-function completeness |
| Recreating the plan's 94-kind edge registry | No increment-1 emitter for most kinds | A slice emits a kind that isn't declared |
| Decidability review of §5 (projections) and §B5 (CFG) | Out of scope (increments 2-3) | Start of increment 2 |
