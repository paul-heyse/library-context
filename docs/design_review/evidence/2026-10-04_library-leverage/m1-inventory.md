# M1 inventory: generic mechanisms in lctx-model execution, analysis, conditions, transfer and analytics

This is supporting evidence for the
[library-leverage review](../../reviews/design_review_library-leverage_2026-10-04.md). It is a
repository-fact inventory from a code-mapper and gives no verdict. The coordinator published it
as returned, with only formatting changed.

- **Baseline:** `main` at `948b2a88`. HEAD was still `948b2a88` at check time. `git log 948b2a88..HEAD` shows no commits touching `crates/lctx-model`, `crates/lctx-analytics`, `docs/adr` or `docs/design`.
- **Method:** read-only; nothing was built or run.
- **Paths:** relative to `crates/lctx-model/src/domain/` unless stated otherwise.
- **Purity confirmed:** `crates/lctx-model/Cargo.toml` production dependencies are serde_arrow, arrow-{array,schema,row,select}, blake3, sha2, serde/serde_json, schemars, biodivine-lib-bdd, petgraph, fixedbitset, leiden-rs, postcard, toml and unicode-segmentation. There is no DataFusion, SQLx or I/O.

## 1. Candidate inventory

**M1-01: Pareto frontier with dominance-based admission**
- **Location:** `execution/summary_worklist.rs:10-151`, `Frontier<K,W>` and `ProofCost`, about 140 lines.
- **Capability:** a keyed set of non-dominated representatives under a two-dimensional cost order (depth, steps). Insertion returns Advanced, Existing (dominated) or Refused(limit). There is a member cap, and memory is charged.
- **Consumers:** `execution/summary_production.rs:911-950` (`Progress`); admission calls at 1665, 1841 and 2047.
- **Invariants:**
  - Explicit refusal (`Admission::Refused(ObligationKind)`).
  - Memory charged through `StateCharge` ("summary-pareto-frontier"); refusal happens before any mutation.
  - Deterministic: rows sorted by (depth, steps, witness) in a BTreeMap.
  - Keys are semantic ContentHash values.
- **Libraries inside:** none (std BTreeMap).
- **Stated reason:** ADR-0053 option 4 says a bounded worklist inside the petgraph SCC schedule "requires bespoke delta propagation". `behavioral-analysis.md` §Transfer summaries specifies "nondominated depth/cost representatives".
- **Repetition:** none found in scope.
- **Tests:** four in-module tests, `summary_worklist.rs:206-329`.
- **A library would need:** a dominance relation per key, refusal rather than eviction, a charge hook at admission, and stable witness order.

**M1-02: Charged, deduplicating work queue**
- **Location:** `execution/summary_worklist.rs:153-204`, `WorkQueue<T>`, about 50 lines.
- **Capability:** an ordered set-queue (BTreeSet with `pop_first`). Admission and pop each consume one unit of work; a duplicate pending item is free. When work runs out it returns `Err(SummaryPairWorkLimit)`.
- **Consumers:** `summary_production.rs:1658-1720`. When work is exhausted, every pending item becomes a `residual(...)` (lines 1702-1720).
- **Invariants:** explicit refusal, with residuals for pending items; a work bound plus a memory charge; canonical pop order.
- **Libraries inside:** none.
- **Stated reason:** ADR-0053 options 2-4 (Ascent and datafrog not adopted for one value relation; there is a revisit trigger).
- **Repetition:** the only work-charged queue. The uncharged traversal stacks are covered under M1-06 and M1-07.
- **Tests:** `summary_worklist.rs:290-329`; the summary production test at `summary_production.rs:2203`.
- **A library would need:** semi-naive or worklist iteration with a per-step charge callback, and access to the pending set when work is exhausted.

**M1-03: SCC-local fixpoint driver (the summary engine)**
- **Location:** `execution/summary_production.rs:1571-2160`, `produce`, about 590 lines.
- **Capability:** for each SCC in callee-first order, seed a frontier, drain the queue, and on exhaustion turn the remainder into residuals and boundaries.
- **Domain parts:** composition, BDD conditions, proofs and publication (`enqueue` 1257, `residual` 1135, `publish` 1346 and 1408).
- **Invariants:**
  - Base-free cycles stay open.
  - A missing component becomes an `IncompleteDomain` boundary (1629-1645).
  - Depth, proof and work caps.
  - Canonical schedule and queue order; nominal identity.
- **Libraries inside:** petgraph (through M1-04) and biodivine (through the condition kernel).
- **Stated reason:**
  - ADR-0053 decision. It is revisited when "two or more … summary channels need a shared recursive rule … or the pilot shows SCC-local composition dominates".
  - ADR-0044 Decision, §"SCCs and summary order" (the W12 comparison).
- **Repetition:** the round-robin fixpoint in M1-08.
- **Tests:** `summary_production.rs:2203` (one in-module); `tests/summary_schedule.rs` (two); `tests/symbolic_summary_synthesis.rs`; `tests/upper_frontier_closures.rs`.
- **A library would need:** recursive rules over keyed relations, a lattice or antichain merge (as in M1-01), external BDD conjunction and proof identity per derivation, and a refusal path that publishes pending tuples rather than dropping them.

**M1-04: Canonical callee-first SCC schedule**
- **Location:** `execution/summary_schedule.rs:22-108`, about 85 lines. It is a Kahn sort over the condensation, ordered lexicographically.
- **How it works:** `invocation_sccs` calls `petgraph::algo::kosaraju_scc` (line 41). `canonical_order` (59-108) then runs a Kahn topological sort whose ready queue is a BTreeSet keyed by each component's minimum member.
- **Consumers:** `summary_production.rs:1613`; re-exported as `crates/lctx-analytics/src/native_schedule.rs`.
- **Invariants:**
  - Determinism does not depend on the library's component order.
  - Graph tokens never escape (doc at line 11).
  - The partition is validated; a malformed condensation refuses (60).
  - Memory is reserved.
- **Libraries inside:** petgraph `kosaraju_scc`.
- **Stated reason:**
  - ADR-0052: "graph library owns traversal; the repository owns identity and tie order". The recursive `tarjan_scc` was rejected for stack depth.
  - ADR-0044 D3 (2026-09-23 review): petgraph `condensation` merges parallel edges.
- **Note:** ADR-0044 §"SCCs and summary order" still reads "`tarjan_scc` order … (Implemented)", but the code uses `kosaraju_scc` per ADR-0052. The internal condensation keeps callee and caller sets as BTreeSets (76-86), so parallel arcs collapse. That suffices for ordering only. No evidence-preserving condensation is published; ADR-0044 still lists that as Proposed.
- **Repetition:** a second lexicographic Kahn sort in M1-05. petgraph `toposort` is already used at `derivation.rs:113`, but only for acyclicity.
- **Tests:** `summary_schedule.rs:110-235`, three in-module tests: canonical ties, reversed edges, malformed partition.
- **A library would need:** SCC membership plus a min-key lexicographic topological order over the condensation. petgraph `toposort` has no tie-break key.

**M1-05: Lexicographic topological sort of the stage graph**
- **Location:** `stages.rs:864-877`, in `Schedule::build_with_publications`, about 15 lines.
- **Capability:** repeatedly pick the minimum-name stage that has no remaining dependencies; return an error on "stage cycle". It is O(n²).
- **Consumers:** every `Schedule::build`.
- **Invariants:** deterministic order, which is then digested into the schedule identity (878-893); explicit refusal on a cycle.
- **Libraries inside:** none.
- **Stated reason:** none found.
- **Repetition:** M1-04 `canonical_order`.
- **Tests:** `tests/domain_stages.rs:761,770` (the cycle cases).
- **A library would need:** a lexicographic topological sort with a name tie-break and cycle detection.

**M1-06: Transitive closure of stage-input relation dependencies, duplicated**
- **Capability:** starting from the requested relations, follow `field.target()` references and `invariant.inputs` transitively. Facts and vocabulary relations get special handling. A dependency on the stage's own unfinished output is refused.
- **Canonical owner:** `dependency_closure.rs:14-175`, `DependencyClosure::build` (pop loop at 68). Its users: `catalog/evidence/build.rs:1071`, `selection/build.rs:855`, `synthesis/production.rs:151`, `analytics/build.rs:788`, `crates/lctx-postgres/tests/stage_validation.rs:204`.
- **Bespoke copies (each `pub fn stage(...)` hand-rolls the loop):**
  - `local_semantics.rs:899` (fn at 836). This copy uses `panic!("Local predecessor depends on its own output")` (~915) where the others return `Err`.
  - `execution/enriched_production.rs:893` (fn 823)
  - `execution/model_production.rs:932` (fn 837)
  - `execution/completion_production.rs:713` (fn 648)
  - `execution/production.rs:549` (fn 485)
  - `analysis/frontier.rs:424` (fn 397)
  - `execution/summary_replay.rs:254` (fn 191)
  - `execution/source_call_records.rs:877` (fn 814)
  - `analysis/preparation.rs:164` (`native_stage`)
  - Outside M1 scope, the same shape at `structural/build.rs:613`, `retrieval/build.rs:961` and `embedding/text.rs:192`.
- **Invariants:** explicit refusal on reading the stage's own output (one copy panics instead); deterministic grants via BTreeMap; model purity unaffected.
- **Libraries inside:** none.
- **Stated reason:** none found for the duplication.
- **Tests:** `tests/dependency_closure.rs` has three tests, covering only the canonical owner.
- **A library would need:** graph reachability over the static relation model plus per-copy policy (facts omission, epoch merge). This is mostly reuse of the existing owner rather than a library question; a petgraph DFS over a model graph would also cover it.

**M1-07: Bounded node-closure traversals over persisted DAGs**
- **Capability:** a DFS stack with a visited set and a node or atom cap.
- **Instances:**
  - `conditions/kernel.rs:840-886` (`closure`): a reduced ordered BDD with an order check and MAX_NODES/MAX_ATOMS caps. Used by `conditions/mod.rs:96` CatalogCheck, `transfer.rs:556` and `composition.rs:1581`.
  - `conditions/graph.rs:202-215`: live-node marking.
  - `local_semantics.rs:800-830`: a charged visited set with a 100,000 work cap, used to test whether an atom is mentioned. This duplicates a support query on the BDD.
  - Outside M1 scope: `normalized/contract_comparison.rs:70` (cap 4096) and `normalized/dispatch.rs:535`.
- **Invariants:** a cap with refusal (`Err`, or `false` meaning "open" in contract_comparison); charged visited sets in local_semantics.
- **Stated reason:** none found.
- **A library would need:** a capped reachability walk. For local_semantics, `Diagram::support()` already exists (`kernel.rs:381`).

**M1-08: Round-robin "until no progress" fixpoint**
- **Location:** `execution/enriched_production.rs:330-423` (`for _ in 0..64 { progress … }`) and `:427-550` (`for _ in 0..frame.headers().len()`). The shared work counter `step()` is at 194-207 (`ENRICHED_WORK_LIMIT` → `ModelError::Resource`).
- **Capability:** naive chaotic iteration over with-statements and headers until nothing new is proved.
- **Invariants:** a work bound through `step()`, which errors; a round cap.
- **Observed:** if the 64-round cap ends the loop while `progress` is still true, there is no boundary or refusal at loop exit (421-423). Whether unproved occurrences are refused downstream was not traced.
- **Libraries inside:** none.
- **Stated reason:** none found; the comment at 425-426 is a semantic note.
- **Repetition:** M1-03, in worklist form.
- **Tests:** not specifically located.
- **A library would need:** a dependency-driven worklist or semi-naive evaluation with explicit non-convergence reporting.

**M1-09: Bounded BFS with parent pointers, caps and a stop reason**
- **Location:** `analysis/delegation.rs:395-470`, about 80 lines. A VecDeque with BTreeMaps for depths and parents. Adjacency comes from borrowed petgraph views (360-393), sorted canonically by (target, site, arc).
- **Invariants:** explicit `Stop::{Arcs, Vertices}` and `depth_limited`; boundaries per target; canonical sibling order; nominal IDs only.
- **Libraries inside:** petgraph, for adjacency only (`IntoEdges`, `EdgeRef`).
- **Stated reason:**
  - ADR-0044 Decision, §Traversal: "Explicit BFS with parent pointers … `visit::Bfs` … not used".
  - 2026-09-23 review D4 gives the ordering facts: newest-edge-first adjacency, and Bfs and Dfs visit siblings in opposite orders.
- **Tests:** `tests/domain_analysis.rs` and related; not individually located.
- **A library would need:** a BFS with a predecessor map, deterministic neighbour order, caps and an early-stop report.

**M1-10: Weighted PageRank power iteration**
- **Location:** `analytics/ranking.rs:73-213`, about 140 lines.
- **Capability:**
  - Interns nominal vertices to dense indices (sort plus `binary_search`).
  - Aggregates parallel pairs with checked u64 weights; dangling and teleport mass are uniform.
  - Uses an L1 residual and reports `Stop::{Converged, Empty, IterationLimit, WorkLimit}` plus a work count.
- **Consumers:** `analytics/build.rs`; re-exported as `crates/lctx-analytics/src/native_ranking.rs`.
- **Invariants:**
  - An explicit empty case and stop reason; never silent.
  - Work, iteration and memory bounds.
  - Canonical accumulation order and input-order independence (a test reverses the inputs).
  - Nominal IDs in and out.
- **Libraries inside:** none.
- **Stated reason:**
  - ADR-0044 Options 3/4, and 2026-09-23 review D1: petgraph `page_rank` is unweighted, credits parallel edges incorrectly, is O(it·V·E) and reports no convergence; graphops pulls in petgraph 0.6.
  - `analytics.md` §9.5: "an unweighted library routine is not an interchangeable definition".
- **Not covered by those records:**
  - The oracle test at `crates/lctx-analytics/tests/native_ranking.rs:40-48` compares against `leiden_rs::infomap::compute_flow`, a weighted directed PageRank power iteration. leiden-rs is already a production dependency of lctx-model.
  - The pinned source (`.claude/skills/rust-graphs/build/acquired/corpus/leiden-rs/source/infomap.rs:191-260`) shows `compute_flow(graph, teleport_rate, tolerance, max_iter) -> Vec<FlowData>`.
  - It returns no iteration count, residual or converged flag. Its residual is L∞ where ours is L1. It has no work bound, and dangling updates are O(n) per dangling node.
  - ADR-0044 and D1 evaluated petgraph, graphops and rustworkx-core, but not leiden-rs `infomap`.
- **Repetition:** the nominal-to-dense interning pattern (M1-14).
- **Tests:** `crates/lctx-analytics/tests/native_ranking.rs` (three tests, with the leiden-rs oracle); `tests/ranking_policy.rs`.
- **A library would need:** a weighted directed power iteration over a dense graph with a convergence and residual report, a work cap and deterministic accumulation order.

**M1-11: NextClosure / Duquenne–Guigues kernel with implication closure**
- **Location:** `analytics/concepts.rs:1-247`.
- **Capability:**
  - A FixedBitSet formal context (rows and columns).
  - NextClosure with an iceberg `min_support`; pseudo-intents form the canonical basis.
  - `implication_closure` (228-247) is a naive repeat-until-unchanged loop.
  - A `max_examined` cap sets `budget_reached`.
- **Consumers:** `analytics/attributes.rs` (FCA, and the RCA one-step scaling at 582-620); re-exported as `crates/lctx-analytics/src/native_concepts.rs`.
- **Invariants:**
  - `budget_reached` is explicit; partial results stay exact.
  - Memory is charged (scratch reservation at line 140).
  - Objects and attributes are sorted, so output is deterministic.
  - Dense indices stay private (doc at 1-2).
- **Observed:** `implication_closure` and the `(0..m).rev()` candidate scan are bounded only by `max_examined` (counted per concept or pseudo-intent) and memory. They are not charged per closure step.
- **Libraries inside:** fixedbitset.
- **Stated reason:** ADR-0044 ("No usable concept-analysis crate": odis AGPL, fcars without support or implications, dci unmaintained); 2026-09-23 review D5; `analytics.md` §9.6.
- **Not covered by those records:**
  - odis `=2026.9.1` is now pinned (root `Cargo.toml:45`) as a development oracle. `crates/lctx-analytics/tests/implication_oracle.rs:5-9` uses its `NextClosure`, `CanonicalBasis` and `Titanic` (iceberg).
  - ADR-0121 (options at 31-33, consequences at 61-63) records that licensing is no rejection criterion. It also records that odis does not fit production because its "batch allocation/work contracts do not preserve the charged production attempt".
  - D5 recommended RCA ∃-scaling as a DataFusion join; the code does it inside the model (`attributes.rs:582ff`), as purity requires.
- **Tests:** `crates/lctx-analytics/tests/native_concepts.rs` (fcars oracle, input-order permutations); `implication_oracle.rs` (odis oracle, 477 lines); `tests/domain_analytics.rs`.
- **A library would need:** iceberg concept enumeration plus the canonical basis, a per-step budget hook or a cap with partial-result semantics, and an object/attribute generic or dense-index mapping.

**M1-12: Leiden adapter and stability profile**
- **Location:** `analytics/communities.rs:72-213`, plus `normalize` (214-273) and `analytics/partitions.rs:1-120`.
- **Capability:**
  - Normalises pairs to (min, max) and sums them in a BTreeMap.
  - Runs seeded RBER across resolutions × seeds, using leiden-rs `metrics::try_ari`/`try_nmi`.
  - Computes the ARI mean and standard deviation by hand (160-162) and applies a degeneracy rule.
  - `normalize` does percentile hub scaling and unit-total normalisation.
  - `partitions.rs` computes pairwise co-membership agreement with a nested loop (82-95) and canonical labels (`communities.rs:59-68`).
- **Invariants:**
  - Work is preflighted against the declared bound (79-85); explicit refusal, then a reservation.
  - Fixed seeds.
  - Orientation normal form, per ADR-0044 and D2 ("undirected builder does not normalize orientation").
  - Nominal IDs only.
- **Libraries inside:** leiden-rs (the algorithm and the metrics).
- **Stated reason:** ADR-0044 §Communities; D2; ADR-0020/0106; `analytics.md` §9.4.
- **Tests:** `tests/domain_analytics.rs:83` (`seeded_leiden_profiles_compare_nominal_membership_under_permutation`).
- **Observation:** the library already supplies the algorithm and the ARI/NMI metrics. Bespoke code remains for the aggregation statistics, the normalisation and the agreement loops.

**M1-13: Exact best-window cosine kNN and centroid**
- **Location:** `analytics/neighbours.rs:158-340` (`nearest`) and `:353-489` (`centroid`).
- **Capability:**
  - An exhaustive O(Q·T·Wq·Wt) f64 dot product, with work preflight (`max_window_pairs`).
  - Canonical tie-break tuples; top-k by a full sort.
  - A missing-window `incomplete` flag; centroid normalisation at 453-458.
- **Invariants:** `Error::WorkLimit` and `incomplete` (never zero vectors); reserved memory; canonical ties; nominal IDs.
- **Libraries inside:** none.
- **Stated reason:** none found in scope beyond the doc ("Bounded exact window similarity … Scores remain heuristic"). The Lance/LanceDB trigger in forward plan §7 and the pgvector exact ranks in serving are outside the model.
- **Repetition:** `analytics/vectors.rs` consumes this (1-411; outlined only).
- **Tests:** `crates/lctx-analytics/tests/native_neighbours.rs` (four tests).
- **A library would need:** exact (not approximate) max-over-windows similarity with work preflight and deterministic tie order, pure and in memory.

**M1-14: Nominal-to-dense interning (sort, dedup check, `binary_search`)**
- **Instances:**
  - `analytics/ranking.rs:98-122`
  - `analytics/concepts.rs:75-96`
  - `analytics/communities.rs:92-110`
  - `analytics/neighbours.rs:184-186` and `383-385`
  - The condition kernel's atom-to-variable support vector, `conditions/kernel.rs:226-262` (`binary_search` at 255 and 492).
- **Invariants:** duplicates refuse; foreign IDs refuse; dense indices never escape; canonical order.
- **Libraries inside:** none.
- **Stated reason:** ADR-0044 D4 ("the sorted domain ids are the dense index").
- **Repetition:** five call sites in analytics, plus the kernel.
- **A library would need:** a typed dense index map with canonical order and refusal of foreign lookups.

**M1-15: Bounded BDD operations beyond the library calls**
- **Location:** `conditions/kernel.rs` (886 lines) and `conditions/substitution.rs` (185 lines).
- **What it adds on top of biodivine:**
  - A node- and work-capped `apply` (103-116, wrapping `Bdd::binary_op_with_limit`).
  - `exists` implemented as restrict plus a capped `or` per variable (723-748).
  - Simultaneous `compose` substitution under a `Budget{work, nodes}` (`substitution.rs:14-60` onward).
  - Admission allowance arithmetic (608-722).
  - Persistence from nodes to a BDD (430-530) via `Bdd::from_nodes` and `validate`.
  - `compatible` and `implies` via `Bdd::check_binary_op` (825-845).
  - `render_terms` via `sat_clauses` (798).
- **Library surface not used:** pinned biodivine 0.6.3 offers `Bdd::exists(&[BddVariable])`, `var_exists`, `substitute(var, &Bdd)`, `binary_op_with_exists` and `ternary_op` (`.claude/skills/rust-reasoning/content/api/biodivine_lib_bdd.md:62,123,138`). None of these takes a node or work limit.
- **Invariants:**
  - `KernelBoundary` errors mean unknown, never `false` (doc at 600-601); `compatible` and `implies` return `Err` for unknown.
  - Node, atom and work caps, plus a reservation.
  - Canonical atom order; identity via the reduced node records; nominal atoms.
- **Stated reason:**
  - In-file docs at `kernel.rs:28-30,50-51,118-119` and `substitution.rs:1` (DESIGN §15.6-§15.7).
  - ADR-0045 is cited at `DESIGN.md:470` ("bounded BDD compatibility screen").
  - ADR-0024, 0032 and 0082 are retired and absent from `docs/adr`.
  - Nothing in scope explains why the library's unbounded `exists`/`substitute` are not used.
- **Tests:** `tests/domain_conditions.rs` (eight tests), `tests/domain_guard_rebase.rs`, `tests/domain_stability.rs`; no in-module tests.
- **A library would need:** node- and work-limited quantification and composition that refuse rather than blow up.

**M1-16: Memory-charged collections and the budget abstraction**
- **Location:**
  - `charged.rs:1-215`: `StateCharge`, `ChargedMap`, `ChargedSet`, `ChargedVec`, each with an admission charge before growth.
  - `resources.rs`: the `ResourcePool` and `Reservation` traits, `FixedPool` and `ScopedPool`.
  - `record.rs:102`: the `HeapSize` trait, with 62 `impl HeapSize for` in lctx-model.
  - Production backing: the DataFusion `MemoryPool` adapter at `crates/cpg-core/src/model_runtime.rs:309-365` (`ComputePool`, `ComputeReservation`).
- **Invariants:** the charge precedes mutation; an unbound default refuses (`charged.rs:12-13`); model purity ("library-neutral allocation reservations", `resources.rs:1`).
- **Libraries inside:** none in the model. DataFusion sits behind the trait in cpg-core.
- **Stated reason:** purity (ADR-0085, the Cargo comment).
- **Repetition:**
  - `batching.rs:68-160` hand-charges HashMap growth (`grow_index`, ×2 for hash overhead).
  - `summary_worklist.rs:100-118` charges per entry with a +64 allowance.
  - `concepts.rs:12-16` charges the bitset block allowance.
  - `ranking.rs:84-99` and `communities.rs:86-91` use constant-factor workspace estimates.
- **Tests:** `tests/domain_resources.rs` (ten), `tests/resource_scopes.rs`, and budget-zero assertions in each kernel test.
- **A library would need:** a size-estimation trait (a heap-size derive) plus an allocation-admission pool that works without DataFusion.

**M1-17: In-memory relational validator**
- **Location:** `memory.rs:1-707`. It does sort, uniqueness, foreign-key semijoin and digest.
- **Mechanics:**
  - `order` (547-579): `arrow_row::RowConverter` plus a stable `Vec::sort_by` and `take_record_batch`, emulating ORDER BY … COLLATE "C".
  - `unique` (666-707): deduplicates by row equality and detects conflicts.
  - `Keys::collect` and `Keys::references` (595-660): key uniqueness, and foreign-key or subtype-tag membership, through ChargedSet and ChargedMap. This is a hand-written semijoin.
  - Chunking and the content digest.
- **Consumers:** conformance and fixture paths (`MemoryGeneration::conformance` and `bind`, the `StageSink` implementation).
- **Invariants:** must equal the PostgreSQL digest (doc at line 1); charged; stable-tie determinism; conflicts refuse.
- **Libraries inside:** arrow-row and arrow-select. arrow-ord (`lexsort_to_indices`) is not a dependency.
- **Stated reason:** purity (no DataFusion), plus "validates them the same way" as PostgreSQL.
- **Tests:** `tests/domain_memory.rs` (three).
- **A library would need:** a pure Arrow sort with null and collation options, distinct-on-id with full-row comparison, and anti-join presence checks, all under a charge.

**M1-18: Validator framework**
- **Location:** `model.rs:606-617`, the `InvariantCheck` trait (`visit_input`, `visit`, `finish`), with factories at 623-629. There are 110 `impl … InvariantCheck for` across 82 files in lctx-model. Examples in scope: `conditions/mod.rs:79-124`, `derivation.rs:117-160`, `analysis/frontier.rs:357-395`.
- **Invariants:** batches are streamed in declared input order; state is charged; failure is explicit.
- **Libraries inside:** none.
- **Stated reason:** `memory.rs:1-4` (the same checks feed both PostgreSQL and memory).
- **Observation:** a streaming fold over typed batches; the domain rules live in each implementation.

**M1-19: Transfer-batch writer with identity deduplication**
- **Location:** `batching.rs:1-192`.
- **Capability:** accumulates rows up to row and byte targets; deduplicates by Id → ContentHash, with a conflict on payload mismatch; poisons itself after the first error; hands the reservation over to the flushed batch.
- **Invariants:** explicit poison and conflict; charged; nominal IDs.
- **Libraries inside:** std HashMap.
- **Stated reason:** the doc comment at 1-2.
- **Tests:** `tests/domain_resources.rs` and the `BatchWriter` users.

**M1-20: Finite-float newtype with canonical bits**
- **Location:** `finite.rs:1-57`, `FiniteF64` (finite values only, with a single zero), plus serde and the Key encoding.
- **Stated reason:** none beyond the doc.
- **Library space:** ordered-float or decorum. The forward plan §7 "nutype/garde/bon" trigger fires on a "repeated checked scalar". This is an observation only.
- **Tests:** `tests/finite_metrics.rs`.

## 2. Trigger evidence (forward plan §7, `docs/plans/behavioral-model-forward-plan_2026-09-24.md:1127ff`)

- **Ascent / named recursive analysis, and the ADR-0053 revisit:**
  - The triggers are "genuine supported recursive multi-relation rules outgrow a clear worklist" and, for ADR-0053, "two or more effect, exception or role summary channels need a shared recursive rule".
  - One work-charged SCC worklist engine exists (M1-01 to M1-03), in `summary_production.rs`.
  - `summary_exceptions::derive` runs before it (`summary_production.rs:1609`) as a separate pass. Whether it recurses was not read.
  - A second fixpoint style is the round-robin in M1-08 (`enriched_production.rs:330`, `:427`).
  - "worklist" mentions in `crates/`: 15 in `summary_production.rs`, and one each in `summary_replay`, `summary_proof`, `summary_capture`, `execution.rs`, `configuration.rs`, `obligation.rs`, `structural/controls.rs` and `synthesis/summary.rs`.
  - `domain/` has 22 `while let Some(..pop..)` loops:
    - 12 are the stage-input closure (M1-06).
    - 5 are DAG closures (M1-07).
    - One each: the delegation BFS (M1-09), the Kahn sort (M1-04), `catalog/access_routes.rs:532` route search, `embedding/text.rs`, and `kernel.rs:134` `CondExpr::atoms`.
  - No multi-relation recursive rule set was found outside the summary engine.
- **Typed dense collections ("multiple dense index domains are actually needed"):** five separate nominal-to-dense interning sites in analytics, plus the BDD atom support vector (M1-14). Each is private and local to its function; no dense index crosses a module.
- **lasso / roaring:** fixedbitset is used only in `concepts.rs`. There is no string interner in scope, and no measurement of allocation or sparse-set cost was found.
- **imbl / rpds / small vectors ("measured cloning/branching"):** `summary_production.rs:1617-1632` deep-copies the Vocabulary (`data.vocabulary.copy`) and re-inserts roots, places and paths. No measurement was found.
- **egg / ena / Z3 / OxiDD ("reasoning gap beyond current bounded models"):** the BDD kernel is self-contained on biodivine. Its unbounded `exists` and `substitute` are bypassed in favour of capped bespoke versions (M1-15). No in-code note records a rewrite or unification gap.
- **Workspace counts:** `BTreeMap<` appears 321 times in `crates/**/*.rs` and 110 times in `lctx-model/src/domain` (`grep -rn 'BTreeMap<'`); the lead's 205 likely used a different count method. `macro_rules!` appears 119 times in M1 scope files. The heaviest are `summary_production` (13), `summary_replay` (9), `analysis/frontier` (9), `enriched_production` (7), `local_theory` (7), `model_production` (6), `local_semantics` (6) and `local_fields` (5). These are left for M2.

## 3. Domain, not a candidate

- **`execution/`:**
  - `summary_consequences`, `summary_path`, `summary_proof`, `summary_alias`, `summary_capture`, `summary_control`, `summary_exceptions`, `summary_replay` (except the closure copy at line 254), `summary_symbolic`, `summary_terminal`: claims, proofs and channels.
  - `completion*`, `enriched*` (except M1-08), `model_*`, `production` (except M1-06), `evaluation`, `protocol_interpretation`, `closed_targets`, `context_*`, `capture_bridge`, `body*`, `definition`, `source_call*`, `modeled_call`, `builtin_read`, `read_dynamic`, `read_channels`, `read_fields`, `records`, `outcome`: Python execution semantics and their records.
- **`analysis/`:** `frontier` (except M1-06), `family/*`, `expected`, `findings`, `policy`, `settings`, `config`, `sources`, `support`, `usage`, `native`, `coverage` and `obligation_support`: owner coverage and refusal vocabulary.
- **`conditions/`:** the `rebase`, `stability` and `entry` modules and the substitution policy (guard restatement, binding witnesses, entry-value proofs). Only the bounded BDD mechanics are M1-15.
- **`transfer.rs` and `transfer/witness.rs`:** transfer keys, alternatives and witness citation.
- **`composition.rs` and `place_composition.rs`:** call composition and access-path algebra.
- **`local_theory`, `local_semantics` (except M1-06 and M1-07), `local_symbolic`, `local_fields`:** bounded structural domains under the provider typing model.
- **`flow.rs`, `flow_inventory.rs`, `flow_capture.rs`:** raw flow observations and per-use closure authority.
- **`assumptions.rs`, `assumptions_universe.rs`, `obligation.rs`:** premises, refusal vocabulary and verdict policy.
- **`derivation.rs`:** proof declarations. Its cycle check is already library-backed (petgraph `DiGraphMap` and `toposort`, lines 4 and 113).
- **`stages.rs`:** stage, permit and publication authority (except M1-05).
- **`analytics/`:** `attributes` and `native_attributes` (incidence semantics; the RCA loop is domain scaling), `build`, `records`, `conclusions`, `frames`, `policy`, `vectors` (E1 consumption) and `partitions` (presentation, except the agreement loop noted in M1-12).
- **`crates/lctx-analytics`:**
  - 23 source lines, all re-exports of model kernels (`native_concepts`, `native_neighbours`, `native_ranking`, `native_schedule`, `delegation`, `program_projection`).
  - Its weight is in its oracle tests (`tests/`, about 1,170 lines).
  - Dev-dependencies: fcars, odis, bit-set, bitvec, leiden-rs and serde_json.
  - It owns no mechanism.

## 4. Search coverage and limits

- **Read fully:**
  - `summary_worklist`, `summary_schedule`, `charged`, `finite`, `batching`, `dependency_closure`, `derivation`.
  - The `memory.rs` mechanics.
  - `analytics/{ranking,concepts,communities,partitions,mod}` and the core of `analytics/neighbours`.
  - The `conditions/kernel` operations, the head of `conditions/substitution` and the `conditions/mod` catalog check.
  - The `summary_production` driver (1571-1720), the `stages.rs` sort, the `delegation` BFS and all twelve closure-loop sites.
- **Outlined or grepped only:**
  - The remaining execution files.
  - `local_*`, `composition`, `flow*`, `transfer`, `assumptions*`, `obligation`.
  - `analytics/{vectors,attributes,native_attributes,build,records,frames}`.
  - `conditions/{rebase,stability,entry,graph}`.
- **Searches (domain and crates):** `worklist`, `while let Some(.*pop`, `let mut changed|progress`, `toposort|topological|cycle`, `windows(2).any`, `NodeIndex|DiGraphMap|Graph<`, `impl.*InvariantCheck`, `impl.*ResourcePool`, `HeapSize for`, `BTreeMap<`, `macro_rules!`, `fcars|odis|NextClosure`.
- **Unresolved edges:**
  1. Whether the 64-round cap in M1-08 is surfaced downstream as a refusal.
  2. Whether `summary_exceptions::derive` holds its own fixpoint.
  3. The canonical encoding (`identity.rs` KeySink, blake3) and the pool internals in `resources.rs` are outside the named scope and were noted only as dependencies.
  4. ADR-0044's `tarjan_scc` text is superseded in effect by ADR-0052, but ADR-0044 is not marked superseded.
  5. Pinned biodivine's unlimited `exists`/`substitute` were not probed against the bespoke capped versions. A researcher should confirm whether 0.6.3 has limit-taking variants beyond `binary_op_with_limit` and `fused_binary_flip_op_with_limit`.
  6. No Context7 or new-library search was done; matching is left to the library-research lanes.
