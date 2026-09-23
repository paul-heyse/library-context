# Design review: ADR-0011 (analytics algorithms) at its increment-2 spike, and slice 2.3 (communities)

**Depth:** standard · **Date:** 2026-09-23 · **Target:** commit `8b27dc0` on `main` ·
**Reviewer:** design-reviewer subagent (fresh context), not the author of the slice ·
**Standard:** `docs/design_review/design_principles/` (charter, ADDENDUM, REVIEW_REFERENCE, graph guidelines)

## 1. Decision and scope

**Proposal.** ADR-0011 (`status: proposed`, `evidence: Interface-checked`) says who owns each graph
algorithm: petgraph for traversal and SCCs, our own weighted PageRank, leiden-rs communities fed a
normalized sorted input, and our own FCA and RCA. The record says it is accepted at the increment-2
Leiden spike. Slice 2.3 builds the community half:
- the relations `cpg_schema::communities` (co-use occurrences and public callables);
- the kernel `lctx_analytics::communities`: integer `(min, max)` pair counts per layer, hub
  down-weighting, unit-total layers summed 0.5/0.5, RBER at γ ∈ {0.5, 1, 2, 4} × seeds 0–9, and the
  non-degenerate γ with the highest mean pairwise ARI. Communities are reported with max-Jaccard
  agreement and projected onto public APIs;
- the Stage E wiring (40 `leiden` invocations and one `community_consensus`);
- the `analysis_invocations.diagnostics` migration and the codebook appends;
- the `build.rs` library record, which includes the rand 0.9 line.

**Status of the claims.** The slice is Implemented. Its determinism and LFR claims are **Tested**.
The pilot claims are **Measured**, except where F1 says otherwise.

**Observable outcome.** Each attempt publishes `statistically_derived` `community` findings with their
public members, supporting sites and agreement. The planned consumers are seed selection and the
brief's **Related** field (§10.3). Neither exists yet.

**Baseline.** Before `8b27dc0` there were no communities. ADR-0011 option 2 (no community detection)
is still the §9.8 ablation baseline.

**Supported scope and non-goals.** In scope: ADR-0011 as a whole record (whether to accept it now),
and slice 2.3's code, tests, docs and pilot evidence. Out of scope: PageRank (2.4) and FCA (2.5),
which are unimplemented and judged only as decisions; Stage F (no community consumer exists); and
serving, apart from the coverage count the Python test pins.

**Constraints and uncertainty.** The pilot evidence is one read-only snapshot (`115a9bec`, store
`build/store-next`). Another session keeps working in the tree, so every check here ran against an
export of `8b27dc0`.

### Method and coverage

**Read myself:**
- ADR-0011 in full, and DESIGN at `8b27dc0`: §B4, §5, the §9 intro, and §9.3 to §9.8;
- the deviation log D1–D28 (D3, D6, D21 and D28 in detail);
- the full diff of `8b27dc0`, including every file the prompt lists;
- both `communities.rs` files end to end, and the analyze.rs wiring together with `member()`
  (the fail-closed MRO walk);
- `build.rs`, and `Cargo.lock` for leiden-rs, rand, rand_chacha and rand_core;
- the codebook, contract and rules snapshots, and the new tests with their insta snapshots;
- `scripts/check_gold.py`, `eval/gold/analytics-freeze.json` and `scripts/adr.py` (the immutability
  rules).

**leiden-rs 0.8.1 source.** I read the registry copy that is actually compiled, which is identical to
the skill corpus under `.claude/skills/rust-graphs/content/corpus/leiden-rs/source/` (`diff -q` on 7
files, 2026-09-23). I read:
- `leiden.rs`: `run_core` and `local_moving_dispatch`, where the rayon path is behind `cfg`;
- `algorithm.rs`: aggregation (sorted at :401) and refinement (shuffles only, :453 and :550);
- `quality.rs`: RBER at :237–309;
- `metrics.rs`: the special cases of NMI (:116) and ARI (:222).

**Checks run (2026-09-23, on `git archive 8b27dc0` in a scratch directory):**

| Command | Outcome |
|---|---|
| `INSTA_UPDATE=no cargo nextest run --workspace --no-tests=pass --no-fail-fast` (`CARGO_TARGET_DIR` in scratch, offline) | **passed**, 164/164 |
| `cargo nextest run --workspace --status-level pass -E 'test(communit) \| test(lfr) \| test(shuffled) \| test(hub) \| test(trivial) \| test(handoffs_and_usage_patterns_come_from_official_code) \| test(pass_a_is_identical_across_module_order_and_location) \| test(synthesis_ids_ignore)'` | **passed**, 8/8 (the four kernel tests, `communities_are_stable_and_projected_onto_public_apis`, the docs_shapes co-use assertion, the module-order/location test and the platform-id test) |
| `python3 scripts/adr.py lint` (system python3, not `uv run`; the export has no `.git`, so the check on edits after acceptance probably did not run) | **passed**: `adr lint: ok (19 records)` |
| pytest, `just deps`, `just gold`, rules scan and rule tests, clippy | **not_run**: not needed for the claims examined |
| `just pilot`, GPU service | **not_run**: the brief forbids them |

**Probes (scratch only, never committed).**
- **Pilot evidence.** Read-only `target/release/lctx query` against snapshot `115a9bec`.
- **Input rebuilt from the store.** I rebuilt the kernel's input with SQL: the §5 projection's arcs
  between subsystem functions, and the co-use relation. I re-ran `lctx_analytics::communities` on it
  from a scratch test file. The rebuilt input gives **the published consensus bit-for-bit**:
  - 516 vertices, 712 + 598 pairs, 1,298 combined;
  - thresholds 10 and 250;
  - every per-γ float in the diagnostics.

  The follow-up probes (seed blocks, definite-only arcs, public-member stability) therefore measure
  the real pilot input.
- **LFR.** 15 graphs (μ ∈ {0.1 … 0.5} × 3 generator seeds), the committed test's configuration.

**Not inspected, or not attacked (asserted only):**
- cross-platform float identity (Linux only; NMI uses `ln`, see Q2);
- publication, interruption and the old store's refusal of the migration (G5, inherited);
- the claim that Stage E still takes 2.2 s;
- the served tool's semantics beyond the count its test pins;
- PageRank and FCA as code (none exists).

The scratch target directory was deleted after the run.

## 2. Authority and lifecycle map

| Concept | Identity | Authority / owner | Boundary | Update path | Derived |
|---|---|---|---|---|---|
| Community parameters (γ grid, seeds, hub percentile, layer weights, thresholds) | `Params` JSON digest | `Params::preregistered` (`lctx-analytics/src/communities.rs:61-74`); D28 | the compiler digest (`attempt.rs:91-96`) and every consensus's `parameters` | a code edit. **Not frozen** (F4) | the per-run parameters JSON |
| Co-use and public-callables relations | `communities::digest()` | `cpg-schema/src/communities.rs:20-94` | the compiler digest; the community invocations' `projection_digest` | a code edit | Input layers, member labels |
| Invocation layer | none of its own | `projection::invocation()` (§5), reinterpreted in `Input::build` (`communities.rs:166-171`): undirected, counted, every arc kind and modality | its spec digest is in the compiler digest, **not** in the community `projection_digest` (F3) | a code edit to either | pair counts and least sites |
| A class's member by name along the MRO | node id | **two derivations:** `member()` (`analyze.rs:288-400`, fail-closed) and `public_callables_sql` (`cpg-schema/src/communities.rs:59-78`, nearest release definition) | per snapshot | independently editable (F6) | seed subjects; community member labels |
| Leiden runs | `invocation(leiden, run digest, community digest, seed)` | `analysis_invocations`, method 3 | the snapshot | the attempt | — |
| Consensus | `invocation(community_consensus, parameters digest, community digest)` | `analysis_invocations`, method 4, `diagnostics` | the snapshot | the attempt | findings cite it |
| Community finding | `FindingKey` (kind, subject, status, omitted flag, member keys) | `findings` kind 11, `finding_members` roles 9 and 10 | the snapshot | the attempt | Related (future) |
| Status policy | — | `FINDING_STATUS` (11 → `statistically_derived`) and the rule `semantic:finding-status-policy` | — | an ADR | — |
| Library versions | string | `build.rs` from `Cargo.lock` | the compiler digest; every invocation | the lock | — |

**Deliberately opaque behaviour.** leiden-rs's optimisation is opaque by design. Its contract here, as
read in its source:
- sequential local moving (no `rayon` in `Cargo.lock:3493-3497`);
- `StdRng::seed_from_u64(seed)`, used only for shuffles;
- aggregated edges sorted before rebuilding (`algorithm.rs:401`);
- `quality_history` pushed once per changed level (`leiden.rs:392-403`).

**Identity behaviour.**
- The dense index is the sorted node ids of the subsystem functions that some pair touches. It is
  never persisted.
- Labels are made canonical by first appearance.
- Finding ids contain node ids and access paths, never a float. Floats only choose the subject and
  the three cited pairs, and they are computed in canonical order.
- Adding one function to the library reorders the dense index and can reshape every community. This
  is inherent to seeded Leiden, and nothing claims stability across versions.

## 3. Semantic contracts and invariants

| Invariant | Representation | Enforcement | Failure behaviour | Evidence |
|---|---|---|---|---|
| Community findings are `statistically_derived` | `FINDING_STATUS` (11, 2) | the `semantic:finding-status-policy` rule | publication refused | Tested (rules snapshot; injected-violation tests) |
| Every run is seeded | `seed: Some(seed)` (`communities.rs:345`) | construction | — | Tested (`community_runs`: 40 runs, 10 seeds, 40 parameter sets) |
| Input normal form | `BTreeMap<(min, max), (u64, Id)>` (`:89-101`) | construction | — | Tested (`shuffled_and_flipped_input_gives_the_identical_partition`) |
| Lineage is order-independent | least site by `min` (`:100`) | construction | — | Tested for the invocation layer (the module-order/location test); **no fixture exercises co-use lineage** (Deferred) |
| One rand 0.9 line, recorded | `build.rs:40-44` | build | the build fails | Implemented |
| The parameters don't change after the gold freeze | — | **none** | — | F4 |
| Public callables agree with `member()` | — | **none** | — | F6 |
| The invocation layer's weight policy | — | **none** | — | F3 |

**Absence.** Three states need to stay apart:
- an empty layer gives `completion = complete_under_stated_model` and no findings;
- "every γ degenerate" gives `partial` with `chosen_gamma: null` (docs_shapes);
- a leiden-rs error fails the attempt (`CoreError::Analysis`).

The second state is arguably not "partial" (Observation O3).

**Equivalence.** Byte-identical for fixed inputs and parameters. ARI is arithmetic only. NMI uses
`ln` and appears only in `diagnostics`.

## 4. Derivation and execution

Stage E, on the attempt's session:
1. two `collect` queries (co-use, public callables);
2. `Input::build` over the §5 projection and the subsystem mask;
3. `combined` (hub scaling, normalization, the weighted sum, all in BTreeMap order);
4. `consensus`: one `GraphDataBuilder` graph, then 40 `Leiden::run`s; ARI and NMI per seed pair;
   choice; agreement;
5. `run`, which projects onto public members and builds findings.

The rows go through the same attempt and validators as Passes A–C. `lctx-analytics` stays free of
DataFusion and Delta (guidelines §9).

## 5. Representative journeys

- **Ordinary extension (the increment-3 layers).** Each new layer needs:
  - a relation in `cpg_schema::communities`, which joins the digest;
  - a `Layer` field and a `Params` weight;
  - edits to `combined` and to the supporting-site loop (`communities.rs:545`);
  - its diagnostics.

  That is one file plus one relation. The layer list is repeated in three places in that file:
  acceptable, and not a finding at this size.
- **A meaningful change (editing `Params`).** The compiler digest and the content digest change, and
  the analysis_shapes snapshots probably change. **Nothing requires an ADR-0004 amendment after the
  gold freeze** (F4).
- **Failure.** A leiden-rs error fails the attempt, so nothing is published. An all-degenerate grid
  publishes no communities with `chosen_gamma: null`.

## 6. Acceptance gates

| Gate | Result | Evidence or rationale | Required action |
|---|---|---|---|
| G1 Authority | **fail (latent)** | Two derivations of "the member a class path names" can disagree with no reconciliation: `member()` refuses where an outside or unresolved ancestor, or a non-def binding, comes first. `public_callables_sql` silently reports a later release definition. No pilot instance (0 by SQL) | F6 |
| G2 Semantic fidelity | **pass** | Status is fixed at `statistically_derived` and rule-checked. No template consumes communities, so none can state a control or a limit (Q6). "Nothing chosen" is distinguishable in diagnostics. Candidate and definition arcs are collapsed into the layer weight, but no call is claimed anywhere. That the collapse is undeclared is F3 | — |
| G3 Validity | **pass** | Codebook, ref, key, finite and status-policy rules cover the new rows. The contract and codebook snapshots changed with a declared migration. The old store refuses the new CHECK set, as the open-time verify should | Deferred: a kind↔method rule |
| G4 Hidden behavior | **pass** (for the code as it executes) | Seeds are always set. The leiden-rs `seed: None` path is never used. There is no rayon, and the kernel makes no ambient reads. Parameters are recorded. **Q15's guard (the D21 freeze) does not cover `Params`** | F4, before 3.3's first gold scoring |
| G5 Consistency and recovery | **pass** (inherited, not attacked) | Rows are written through the attempt into one snapshot | — |
| G6 Transformation and reuse | **pass** | Q11: identical output under shuffled and flipped input, module order and location (Tested); an independent recomputation from the store is bit-identical (Measured); seeds, parameters and crate versions (including rand 0.9.5, rand_chacha 0.9.0 and rand_core 0.9.5) are recorded. Q10: no dense index persists, and the least site is order-independent. The projection-digest gap is in F3 | F3 |
| G7 Truthful capability claims | **fail** | DESIGN §9.4 says ADR-0011 is accepted, but it is `proposed`. It says "all five seeds share one community", but four do. Stale §9.4 bullets contradict the implemented text | F1 |

## 7. Principle findings

Ordered by severity. F2 and F3 rank above F6's G1 failure because they change the published pilot
output today, while F6 has no pilot instance.

| # | Finding | Principle IDs | Evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | **DESIGN §9.4 states things the commit does not bear out.** | DM-59, DM-02 · G7 | **(a)** `DESIGN.md:1972`: "ADR-0011 accepted at its spike". But `docs/adr/0011-…md` has `status: proposed`, as do `docs/adr/README.md:15` and `STATUS.md:43`. **(b)** `DESIGN.md:2054`: "All five seeds share one community, the `FastMCP` server surface". In snapshot `115a9bec` the 64-member community lists `FastMCP.tool`, `.mount`, `.resource` and `.prompt`. **No** community lists `custom_route` (`… m.label LIKE '%custom_route%'` returns 0 rows). The rebuilt seed-0 partition puts `TransportMixin.custom_route` in a 4-vertex community with one public member, so it is not reported. **(c)** Bullets now stale in the same section: `:1983` "γ … in the analytics config" (it is D28's code); `:1984` "from §5's dense index" (the kernel builds its own, `communities.rs:191-198`); `:1989` "DataFusion aggregates … under a named weight policy" (Rust, with no named policy); `:1992-1994` "keeps its contributing arcs" (it keeps the least site); `:2001` "public and private callables in the subsystem" (only functions some pair touches); `:2008` "degree above the configured percentile" (strength, a code constant). Also the §9 intro at `:1792` ("parameters … live in one … analytics config") | DESIGN is the single authority. A reader plans the Related field around a shared seed community that `custom_route` is not in, and treats ADR-0011 as closed. A second implementer following the bullets would include isolates, which changes RBER's density term `p = 2m/(N(N−1))` (`quality.rs:279`). That gives a different partition | Change the label to "ADR-0011 proposed" until the status is flipped. Change "all five" to "four of the five (`custom_route` is in a 4-vertex community with one public member)". Replace the stale bullets and the §9 intro sentence with pointers to the implemented text and D28. About 15 lines | (a) `just adr lint`: extend `design_decisions` in `scripts/adr.py` so a DESIGN sentence naming "ADR-NNNN accepted" must match that record's status. (b) and (c): **no mechanical oracle exists**. Prose: copy pilot claims from `lctx query` output |
| **F2** | **The chosen γ depends on the seed block, not on the data.** | DM-40, DM-59 | `communities.rs:403-414` takes the highest mean ARI. `total_cmp` treats only exact equality as a tie, so "ties to γ nearest 1" acts only when two ARIs are identical. On the rebuilt pilot input (bit-identical to the published grid), per seed block: **0–9 → γ 2** (0.793 against γ 1's 0.749); 10–19 → γ 1 (0.795/0.782); 20–29 → γ 1 (0.811/0.772); 30–39 → γ 1 (0.847/0.746); 40–49 → γ 2 (0.777/0.765). With seeds 0–19 → γ 2 (0.790/0.770); with 0–49 → γ 1 (0.791/0.775). The standard deviation of the pairwise ARIs is 0.036–0.092, larger than the margins. The diagnostics record no spread. On LFR (the committed test and 15 probe graphs), γ 1 is chosen every time, and for μ ≤ 0.3 only by the tie-break (ARI 1.000 at both γ 0.5 and γ 1). So the committed fixture exercises the tie-break, not the maximisation | The published partition (59 communities, the largest 78) exists because the seeds are 0–9. Most other blocks give γ 1 (45 communities, the largest about 101). Changing the seed count from 10 to 20 or 50 flips it too. A library change that reorders the dense index can flip γ, and every community moves with it, for reasons that have nothing to do with the library. The §9.8 ablation would then judge a technique whose granularity is partly a coin flip | Decide **before acceptance**, because ADR-0011's Decision names the grid. Either (a) fix γ = 1 (RBER's own density scale) and keep the grid as recorded diagnostics; or (b) treat any γ within a pre-registered margin of the best mean ARI (for example one pair-ARI SD) as tied, with ties going to the γ nearest 1. On the pilot, (b) picks γ 1 in every block I ran. Record the ARI spread in diagnostics either way, and move the rule into one pure function | **test**, new: unit-test the choice over `Stability` rows (margin and ties), and add a kernel test asserting the same choice for seed blocks 0–9 and 10–19 on a near-tie fixture. None exists |
| **F3** | **The invocation layer has no named weight policy. It counts definition and candidate arcs as calls, and neither its lineage nor its projection digest says so.** | DM-22, DM-46; guidelines §3 MUSTs (edge selection, weights) · G6 | `communities.rs:166-171` pushes every projection arc between two subsystem functions, whatever its `arc_kind` or `modality`. `graph.rs:27-43` carries both, and Pass A uses `is_definite_call`. ADR-0011 `:95-96` and `DESIGN.md:1989` require "a named weight policy", and `:2004` says "calls". **Pilot, by SQL over the snapshot's projection:** of the 712 invocation pairs, 315 have a definite call, 370 come from candidate-modality arcs and 33 from definition arcs. 415 of the 419 candidate sites have exactly one target. **Probe:** with definite calls only, the pilot has 376 vertices instead of 516, γ 0.5 is chosen instead of 2, and NMI is 0.768 (ARI 0.473) against the published partition on the common vertices. Of the 59 `supporting_site` rows labelled `invocation`, 36 cite sites whose only subsystem arcs are candidates; the row carries no modality or `edge_id`. The community invocations' `projection_digest` is `communities::digest()` (`cpg-schema/src/communities.rs:89-94`; `analyze.rs:715, 772, 801`), which leaves out `projection::invocation()`'s digest | The layer's meaning is fixed by code that no document states, and it carries real weight: the partition changes wholesale. Two implementers of "calls" would build different graphs. An auditor following a community's strongest pair lands on a nesting, or on an override-dispatch candidate, labelled "invocation". If §5's projection changes, as it did when D5 added definition arcs, the community invocations' `projection_digest` does not show it (only the snapshot's compiler digest does) | Name the policy as a `Params` field, so it is recorded and digested: which arc kinds and modalities count, and at what weight. Keeping all three at 1 is defensible for a statistical layer, since single-target candidates are the call's own target, but say so in §9.4. Fold `projection::invocation().digest()` into `communities::digest()`. Optionally put the modality in the supporting-site label | **test**, new: `communities::digest()` changes when the invocation spec changes, and an analysis_shapes assertion counts, or refuses, a definition-only pair according to the policy. None exists |
| **F6** | **The public-callables relation re-derives MRO member lookup with rules that differ from the fail-closed `member()`.** | DM-02, DM-23 · G1 | `cpg-schema/src/communities.rs:59-78`: `lineage` drops unresolved ancestors (`WHERE t.ancestor_node_id IS NOT NULL`). `candidates` sees only `declarations`, and the pilot has 0 declarations under an outside ancestor, so an outside base's member is invisible. `nearest` ignores class-level non-def bindings. `analyze.rs:288-400` refuses in all three cases (`:380`, `:384`, `:394`; slice 1.4 review F2, D6) | Take `class Sub(ext.Base, pkg.Mixin)` where both bases define `run`. Python resolves `Sub.run` to `ext.Base.run`. `member()` refuses. The relation reports `pkg.Sub.run` → `Mixin.run`, so a community lists that path for the wrong node, and the future Related field would print it. `run = helper` in `Sub` fails the same way. **Latent:** the pilot has no instance (by SQL, 0 inherited methods pass an outside ancestor, 0 are shadowed by a class-level rebinding, and there are 0 null targets). A second library (ADR-0013) could have one | Exclude, rather than refuse, any name whose walk passes an outside or unresolved ancestor or a non-def binding. Mirror `member()`'s rules, ideally from one shared SQL fragment. Add one fixture case per shape | **test**, new: classes of both shapes in `fixtures/python/analysis_shapes`, asserting the relation's rows. None exists |
| **F4** | **D28's pre-registration is asserted, and the parameters sit outside the gold freeze.** | DM-28, DM-31, DM-59 · G4 (Q15) | `git log -S "fn preregistered"` finds only `8b27dc0`, which also carries the pilot output (the DESIGN §9.4 pilot paragraph and the commit message). D28 says "the commit before any community output is the pre-registration". `eval/gold/analytics-freeze.json` hashes only `libraries/fastmcp/analytics.toml`, and `scripts/check_gold.py:62-68` checks only those paths. No test or snapshot pins `Params::preregistered().json()`: the analysis_shapes snapshots record diagnostics, not parameters | After 3.3's first gold scoring, the γ grid, thresholds or layer weights can be edited with `just gold` still passing. Only insta snapshots move, and those are routinely accepted. That reopens the tuning-on-gold path D21 closed for `analytics.toml`. "Fixed before any output" cannot be checked from the history | Add a `community_parameters` digest to the freeze file, and a test in `lctx-analytics` that compares `Params::preregistered().digest()` with it. A change then needs the ADR-0004 amendment D21 requires. Reword D28 to "committed with its first output; frozen from `8b27dc0`". The operator rules on D28 at the end-of-run review | **test** / `just gold`, new. None exists |
| **F5** | **A community's score measures the whole community, but the finding publishes only its public members.** | DM-59, DM-46 | `communities.rs:421-443`: agreement is the mean best Jaccard over **all** of the reference community's vertices. `run` keeps only the public members (`:509-515`), and uses agreement as both the threshold (`:520`) and the score (`:598`). **Probe** (γ 2, seed 0 against seeds 1–9), for the 32 reported communities: in 7, the public members' mean pairwise co-assignment is below the score; in four it is 0.50–0.62 (scores 0.616–0.743), with 0 or 1 of 9 seeds keeping them together; one (score 0.627) is at 0.496, under the 0.5 threshold. The 64-member server community (score 0.777) stays whole in **0 of 9** seeds | When §10.3's Related field says "A and B share a community (0.63)", A and B are co-assigned in about half of the seeds. The score overstates the stability of exactly the claim the consumer makes. Statistical status keeps it out of controls and limits, but Related has no other stability signal | Score the published projection: the mean pairwise co-assignment of the public members across seeds. Apply the threshold to it, or record both, and have the consumer read the projection's score. Owed before the first Related consumer | **test**, new: a hand-built graph with a stable private core whose public members alternate between two cores. None exists |
| **F7** | **Accepting ADR-0011 as written would freeze a Decision section that its own amendments contradict.** | DM-59, DM-49 | ADR-0011 `:95-98` ("DataFusion aggregates … under a named weight policy"; "γ … pre-registered in the analytics config") is superseded only by the trailing amendments. `scripts/adr.py:46-48`: after acceptance only `status` and `superseded-by` may change, plus a trailing `## Amendments` section for **factual corrections**. `evidence` cannot be raised. PageRank (2.4) and FCA (2.5) are unimplemented | Once accepted, the record would state two superseded decisions in its Decision body, correctable only by supersession. Its header would stay at a label that understates traversal and communities, or, if raised to Tested, overstates PageRank and FCA | While the record is still `proposed`: fold the amendments into the Decision body; record F2's and F3's decisions; give each part its label in the body (traversal Tested, 1.4; communities Tested, 2.3; petgraph `page_rank` rejected, Tested by probe; our own PageRank Proposed; FCA Interface-checked); keep `evidence: Interface-checked` as the header's honest floor; then flip the status. Start a fresh trailing `## Amendments` section for facts from 2.4 and 2.5 | `just adr lint` enforces immutability after acceptance (exists). **Nothing checks a Decision body against its amendments**: no oracle |

**Observations (not findings: none has a consequence yet):**
- **O1.** Four of the five seeds sit in one community of 64 public members (78 in all). As a
  seed-selection or Related signal it barely discriminates among them. §9.8's keep rule will judge
  this in increment 3.
- **O2.** The served coverage now counts 41 statistical invocations alongside Passes A–C: 56
  `complete_under_stated_model` (`python/lctx_mcp/tests/test_server.py`). A non-converged Leiden run
  would show `partial: 1` to agents.
- **O3.** An all-degenerate grid reports `completion = partial`, which reads as "cut short".
  `complete_under_stated_model` with `chosen_gamma: null` is closer to the truth.

**Applicability.**
- **Applied:** groups 2, 3, 5, 7, 8, 10 and 12 (analytics routing), plus 1 for F6 and 11 for the
  migration.
- **Group 4** (declarative composition): not engaged. The kernel is a specialised algorithm behind
  declared relations, which charter §F places in ordinary code.
- **Group 6:** nothing new. Stage E's effects are the attempt's.
- **Group 9** (providers): only through leiden-rs, read at source level (§2).
- **Group 11:** the migration and the codebook appends (G3).

### The five focus questions

1. **Acceptance.**
   - The evidence supports **Tested** for the Leiden decision. Shuffled and flipped input,
     module order and location all give identical output (tests passed 2026-09-23). An independent
     recomputation from the store is bit-identical (Measured). LFR recovery holds for the committed
     cases and for 15 probe graphs (chosen γ 1, truth NMI ≥ 0.981 up to μ 0.5). Seeds and crate
     versions are recorded, and the run is sequential (no rayon in the lock).
   - The revisit trigger's first two conditions did not fire. The ablation condition (after
     increment 3) and the PageRank condition (2.4) cannot be evaluated yet.
   - Accept the **whole** record rather than splitting it. The record's own acceptance criterion is
     this spike. The PageRank and FCA choices rest on probe and interface evidence, and their open
     details are owed to DESIGN §9.5 and §9.6, not to the ADR. A split would add a record with no
     consumer (ADR-0001).
   - **Not today, though.** G7 fails, and F2, F3 and F7 are cheaper to settle while the record can
     still be edited (§11).
2. **Determinism and identity.**
   - Every float is computed in canonical order: BTreeMap sums, strength in dense order, and ARI in
     seed-pair order. leiden-rs is sequential, shuffles with a seeded `StdRng`, and sorts its
     aggregated edges. ARI is arithmetic only. NMI's `ln` affects only `diagnostics`, which is in no
     id; cross-platform identity was not attacked.
   - Module order and location change nothing (tested).
   - Finding ids are content-derived and contain no float. Floats choose the subject and the three
     cited pairs, deterministically.
   - The least site per pair is order-independent (it is a `min`). It is tested for the invocation
     layer. The committed shuffle test cannot test it, because its sites derive from edge positions
     and `Consensus` excludes them. No fixture exercises co-use lineage (Deferred).
3. **The selection rule.**
   - The degeneracy guard is sound in intent. leiden-rs's ARI and NMI return 1.0 for identical
     trivial partitions (`metrics.rs:116, 222`), so without the guard, all-in-one or all-singletons
     would win. It checks only seed 0, which is acceptable.
   - The rule itself fails on the pilot (**F2**). γ 2 against γ 1 is a near-tie that the seed block
     decides. That is not a bias toward either end, but the choice is arbitrary.
   - The 0.5 agreement threshold is honestly stated, but it is measured on the whole community
     rather than on the published projection (**F5**).
4. **Truthful claims.**
   - Every pilot number I checked matches the snapshot (vertices, pairs, per-γ ARI and NMI,
     convergence, 59, 78, 32, 7 and 20, the 0.616–1.0 score range with mean 0.886), **except** the
     "five seeds" claim (**F1**). The 2.2 s timing is unverified.
   - Public callables:
     - For release-only chains, the nearest definition along the MRO wins, overrides win, and
       private segments are filtered as stated.
     - The relation diverges from `member()` (**F6**).
     - 22 property getters count as "callables", 4 of them as subjects, and 28 access paths map to
       several nodes (overloads and setters). No published finding repeats a path (Deferred).
5. **Scope.**
   - Hub down-weighting and layer normalization are the design's: §9.4's Graph bullets and ADR-0011's
     Decision predate the slice.
   - The refinements (strength instead of degree, threshold/strength applied to each end, the 0.95
     percentile, weights 0.5/0.5) are recorded in the new §9.4 text and in D28, so they are not
     silent. The old "degree … configured percentile" bullet now contradicts them (F1).
   - Co-use scopes are small on the pilot (a median of 2 and a maximum of 10 subsystem targets per
     scope; the 10 largest scopes give 3.4% of the co-use contributions), so per-scope normalization
     is not needed there.

## 8. Alternatives and architectural leverage

| Alternative | Duplication and extension locality | Risks | Cost | Performance evidence | Why selected or rejected |
|---|---|---|---|---|---|
| Baseline: no communities (ADR-0011 option 2) | none | no Related grouping | none | — | Stays the §9.8 ablation baseline |
| As built: two layers, 4 γ × 10 seeds, the highest mean ARI, a degeneracy guard, a seed-0 reference, Jaccard agreement | layers repeated in 3 places in one file; parameters in code | F2: the choice is decided by the seed block. F5: the score does not measure the projection | 40 runs; about 750 lines including tests | Stage E said to be 2.2 s (not verified) | Built |
| **Simpler: a fixed γ = 1 (RBER's own density scale), 10 seeds, the same layers** | removes the selection rule, its tie-break and the degeneracy heuristics (kept only as a stated "no community" outcome) | if γ 1 is degenerate for some library, that library reports no communities, with a visible reason, instead of silently switching γ | 10 runs, about 40 fewer lines | on 15 LFR graphs γ 1 is chosen, or tied for best against the truth, every time; on the pilot, 3 of 5 seed blocks already choose it | **Recommended**, or F2(b) if the operator wants the grid to decide. Either way it removes one independent semantic decision (DM-56) |

**Abstractions justified by current needs.** `Params` as one serializable struct, recorded and digested,
is the right size. The declared SQL relations (digested) match the §5 pattern.

**What remains ordinary code.** The kernel: counting, scaling, the Leiden calls and agreement.
Nothing here should become declarative (charter §F).

## 9. Verification and measurement plan

| Claim or risk | Evidence label | Test or analysis | Conditions and expected result | Current result or gap |
|---|---|---|---|---|
| Identical output under shuffled and flipped input | **Tested** | `shuffled_and_flipped_input_gives_the_identical_partition` | LFR μ 0.5; `Consensus` equal | passed 2026-09-23. Does not cover lineage |
| Identical output across module order and location | **Tested** | `pass_a_is_identical_across_module_order_and_location` | analysis_shapes; invocations, findings and members equal | passed. Co-use is empty there |
| Reproducible from the store | **Measured** | scratch recomputation from snapshot SQL | the grid equals the published diagnostics bit-for-bit | matched, 2026-09-23 |
| LFR recovery | **Tested** | `lfr_planted_partitions_are_recovered` (plus a 15-graph probe) | NMI ≥ 0.9 | passed; the probe gave ≥ 0.981 at the chosen γ 1 |
| Seeds and versions recorded | **Tested** | `community_runs` snapshot; pilot `library_versions` | 40 runs and 10 seeds; leiden-rs 0.8.1 and rand 0.9.5 | matched |
| The choice is robust to the seed set | **Measured: fails** | scratch seed-block probe | the same γ for seed blocks 0–9 through 40–49 | γ 2, 1, 1, 1, 2 (F2) |
| The score reflects the published projection | **Measured: does not** | scratch co-assignment probe | public co-assignment ≥ score | 7 of 32 below (F5) |
| Public callables agree with `member()` | Implemented, untested | none | fixture shapes of F6 | gap (F6) |
| Parameters frozen after the gold | none | none | the freeze covers `Params` | gap (F4) |
| Pilot figures in §9.4 | **Measured**, except the five-seeds claim and the timing | `lctx query` | as stated | F1(b); timing not verified |

**Cost accounting.** 40 sequential Leiden runs on 516 vertices, each converging in at most 5 levels.
The author reports no measurable change in Stage E (not verified). The simpler alternative runs 10.

## 10. Exceptions and unresolved decisions

**D28, as an exception record.**
- **Principle IDs.** DM-31, DM-59; §9's single analytics config.
- **Scope.** The community parameters.
- **Reason.** `analytics.toml` has been frozen since D21, and adding keys to it needs an ADR-0004
  amendment.
- **Consequence.** The parameters escape the freeze (F4).
- **Compensating control.** They are recorded in every invocation and in the compiler digest. The
  missing control is a digest in the freeze file (F4).
- **Owner.** The author. The operator rules at the end-of-run review.
- **Revisit trigger.** 3.3's first gold scoring, or any edit to `Params`.

**Unresolved decisions for the author:**
- F2: a fixed γ, or a margin rule;
- F3: which arcs count, and at what weight.

Both belong in ADR-0011 before it is accepted.

## 11. Decision and implementation changes

**Decision: Revise (small), then accept ADR-0011.** G7 fails on two DESIGN claims (F1), and G1 fails
latently (F6). The Leiden decision itself is well evidenced: determinism, LFR recovery and recorded
seeds are all Tested, and the revisit trigger has not fired.

**Recommendation on ADR-0011's status:**
- Keep it `proposed` today.
- Flip it to `accepted` as a **whole record** once F1, F2, F3 and F7 are done. None of them needs
  another review if it lands as described.
- Leave the header `evidence: Interface-checked`, with per-part labels in the body.
- F4, F5 and F6 do not block acceptance. Each has a deadline, listed below.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 | F1: correct the §9.4 label and the five-seeds sentence; replace the stale §9.4 bullets and the §9 intro sentence | DM-59 | the DESIGN diff; `lctx query` output quoted | an `adr lint` check on status claims |
| 2 | F2: a fixed γ = 1, or a margin rule; record the ARI spread | DM-40, DM-59 | the pilot diagnostics show the rule's result | a unit test for the choice; a seed-block kernel test |
| 3 | F3: name the arc policy in `Params`; add the invocation spec digest to `communities::digest()` | DM-22, DM-46 | the policy is recorded in the consensus parameters | digest and analysis_shapes tests |
| 4 | F7: consolidate ADR-0011's Decision body and per-part labels, then flip the status | DM-59 | `just adr lint` passes | `adr lint` |
| 5 | F6: public callables exclude what `member()` refuses | DM-02 | fixture rows | a fixture test (before a second library) |
| 6 | F4: freeze the `Params` digest | DM-28, DM-31 | `just gold` fails on a `Params` edit | a test or `just gold` (before 3.3) |
| 7 | F5: score and threshold on the public projection | DM-59 | the pilot score distribution | a kernel test (before the Related consumer) |

**Deferred (deliberately not acted on here):**

| Item | Why deferred | Trigger that reopens it |
|---|---|---|
| A guard that leiden-rs stays feature-free (no `rayon` in its locked dependencies), and rand in `check_family.py` | Today's lock is correct (`Cargo.lock:3493-3497`), and versions are recorded | Any lock change touching leiden-rs or rand, or a second crate depending on leiden-rs |
| Property getters counted as public "callables" (22 on the pilot); overload stubs and setters giving several nodes per path (28 paths) | No published finding repeats a path, and no consumer renders "call X" | The Related template (§10.3), or any finding listing a path twice |
| Co-use lineage has no fixture (analysis_shapes has no co-use pairs; docs_shapes chooses no γ) | Order-independent by construction (`min`) | The next change to `Input::build` or `co_use_sql` |
| O3: `partial` for an all-degenerate grid | Cosmetic until agents read the coverage | Coverage semantics reviewed for serving (increment 4) |
| A rule that a `community` finding cites a `community_consensus` invocation, and that `diagnostics` is valid JSON | One producer | 2.4 adds a second statistical method with diagnostics |
| Cross-platform last-bit NMI in `diagnostics` | Not in any id; Linux only | A second build platform, or bundle byte identity across platforms |
| O1: community discrimination for seed selection | Decided by the §9.8 keep rule | The increment-3 ablation |
| `witnesses_omitted` reused to mean "more supporting pairs than cited" | No consumer reads it for communities | A consumer reading `witnesses_omitted` on kind 11 |

**Final check.**
- **Claims against evidence.** They match, except F1's two sentences.
- **Scope against guarantees.** Matched for determinism. For the selection rule and the projection's
  score, the guarantee is weaker than the prose implies (F2, F5).
- **The extension path.** Clear for the increment-3 layers once the weight policy is named (F3).
