# Stage 3.0 persisted graph and native load — compact change review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Stage 3.0 BDD producer and consumer (`0a7d663`), FORMAT 6 condition files, shared graph hydration, and `lctx_semantics.ConditionGraph` |
| Standard | Core 2.0, code-intelligence profile 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex, direct review of the code and executed checks · 2026-09-24 |
| Decision | Accept scoped to exact propositional persistence and generation load; no served compatibility verdict is accepted |

**Outcome sought.** A condition is constructed before DNF truncation, stored as a root and content-addressed nodes, validated at publication and native generation load, and queried by the same Rust BDD kernel. The source-use/type proof, cross-site stability theory, and operation-anchored FastMCP filter remain outside this slice. The pilot is a pinned FastMCP 4.0.5 release, not an evaluation reference fed into the compiler.

**Coverage.** Inspected the producer, schema, flow-model and behavior consumers, the publication validator, bundle, Python loader, native bridge, and the targeted corruption tests. Tested the 17-site condition, published-node tamper, fixture generation/native load, and a fresh pilot. The full Stage 3 behavior and structured question set were not examined as completed functionality.

**Fact and fidelity boundary.** `flow_test_leaves` is an attributed ty predicate observation; it does not by itself prove a tested value's exact type or stability. `conditions` and `condition_nodes` are derived Boolean structure. The `ConditionGraph` answers exact propositional questions over encoded evaluation atoms. It cannot conclude Python runtime feasibility when distinct sites might read the same value, nor can it make an operation-level claim without a checked entry-to-test link.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass in scope | Rust `Diagram` owns Boolean meaning; Delta and the generation carry derived roots/nodes; Python holds no Boolean evaluator | — |
| G2 Semantic fidelity | Pass in scope | `flow_model` and `behavior` decide from hydrated diagrams; `conditions.encoding` is display. Native queries take condition IDs and return a named boundary on refusal | F03 before operation-level serving |
| G3 Validity | Pass in scope | `hydrate_catalog` checks duplicates, canonical atoms, root closure/IDs and orphan nodes; publication and native load call it. The leaf validator checks source and predicate identity | — |
| G4 Hidden behavior | Pass | Native operations use loaded memory; generation load reads declared files; no analyzer configuration or source execution enters query time | — |
| G5 Consistency and recovery | Pass in scope | FORMAT 6 and kernel-format checks reject older or mismatched generations; exact file names, symlink refusal, hashes and row limits constrain load | — |
| G6 Transformation and reuse | Pass in scope | Structural Merkle IDs survive local BDD variable indices; the bundle is rebuilt from one pinned snapshot; `given` verifies its quotient | — |
| G7 Truthful capability claims | Pass for the scoped claim | `probe_*` remains developer smoke; `ConditionGraph` is loaded but no FastMCP compatibility tool advertises an entry-value verdict | F03 before Stage 3.6 |
| G8 Library leverage | Pass | biodivine owns Boolean apply/BDD representation, Arrow IPC owns files, PyO3 owns the native boundary; bespoke code is source identity and graph admission | — |
| CI-G1 Fidelity | Pass in scope | Provider predicate leaves are not promoted into type/value proofs; missing links remain unknown | F03 before a negative verdict |
| CI-G2 Evidence closure | N.a. | The new graph has no served claim or citation path yet | Review served filter at Stage 3.6 |
| CI-G3 Evaluation integrity | Pass | No gold-family input route was added | — |

**Applicable principle verdicts.** DP-01/02/03/04/08/09/11/13/14/15/18/19/20/21/22/23/24 and CI-01/02/04/06/08/10/12/13 are **Satisfied for this scoped path** at the enforcement points above. DP-07/12/16/17 and CI-03/05/07/09 add no independent decision for this graph-load change. CI-11 is **Unresolved for Stage 3.6**, which has no served compatibility claim in this slice. Cross-site typed theory and proof attribution are **Unresolved for the remaining Stage 3.0 target**, not certified here.

## 7. Findings and disposition

| ID | Finding | Principles · gate | Evidence and consequence | Correction or boundary | Verification |
|---|---|---|---|---|---|
| F01 | A forged manifest could name a file outside its generation | DP-01/19, CI-13 · G5 | `_read` formerly joined `root` to manifest `file` without checking its path; a relative escape could load bytes outside the generation even if their hash matched | Loader and Rust verifier now require exactly `<name>.arrow` and refuse symlinks | Python mismatched-generation test; Rust changed-generation test |
| F02 | Condition files could exhaust load resources before graph validation | DP-20, CI-08 · G5 | `_read` decoded the whole IPC file and `hydrate_catalog` allocated a map before any global row cap | 64 MiB per condition file before read; native-sourced 100,000 condition/node row limits checked before `to_pylist` and hash-map construction | Shared-catalog limit test; native fixture load |
| F03 | No served behavior row carries a checked operation-entry-to-predicate bridge | DP-02/21/22, CI-06/11 · G2/G7 | The `behaviors` serving file has display `condition` text, while `ConditionGraph` takes structural IDs. Querying compatibility from that text or matching a parameter name to a leaf would overstate source-value stability | Stage 3.0 proof rows and Stage 3.6 query contract must add cited root IDs, entry/use/stability links and explicit unknown results. No FastMCP filter is claimed by this slice | Compound/sibling operand, annotation, call/write and synthetic-pattern counterexamples, then a generation-pinned tool test |

F01 and F02 were corrected during this review. F03 is a remaining Stage 3 obligation and a boundary on the accepted scope. `condition_literals` and derived condition strings remain display projections; downstream semantic decisions must use the diagram. The new exact-support use in `flow_model` removes one DNF-dependent atom enumeration.

**Measured join pressure (2026-09-24).** A read-only query on pilot snapshot `226d70b7c94aabca98de16a0be968229` joined each of 6,882 leaves to non-annotation uses contained by its leaf span: 43 had no such use, 2,969 had one, and 3,870 had multiple. This is a count of containment candidates, not an operand proof; it supports F03's requirement for structural operator attribution.

## 8. Library leverage

| Capability | Current placement | Library feature and fit | Decision |
|---|---|---|---|
| Boolean equivalence, implication and compatibility | `cpg-schema::condition_kernel` adapter | biodivine-lib-bdd 0.6.3 supplies bounded binary apply and BDD traversal; source identity and budgets are domain rules | Keep |
| Bundle storage and decode | Arrow IPC via Rust and PyArrow | File schema and IPC decode are library-owned; the Rust/Python schema digest checks are deliberate independent interoperability checks | Keep |
| Python native API | PyO3 0.29.2 | `#[pyclass]` holds the hydrated graph once and returns typed bool/boundary pairs; no per-atom Python callback | Keep |
| Graph admission | `hydrate_catalog` | BDD libraries do not know this repository's Merkle node IDs, atom encoding or publication contract | Keep shared domain validator |

## 12. Decision

**Accept scoped.** The exact propositional graph is produced before DNF loss, persists in Delta and FORMAT 6, and passes one shared graph validator at publication and native load. The pilot's `budget_reached` behavior count fell from the registered 66 to zero. This is a **Measured** pilot observation for FastMCP 4.0.5, not a claim that the typed theory, summaries or Stage 3 questions pass. F03 blocks an operation-level compatibility verdict until the proof bridge and served contract exist.

**Executed checkpoint, 2026-09-24.** `just test-all` **passed** on the reviewed tree: 283/283 Rust tests, fixture generation 1/1, 96/96 Python tests, Pyrefly, seven repository rules, ADR and agent lint, 66 fixture parses, dependency/fork/shear/gold checks. `just pilot build/store-stage3-bdd-format6-final` **passed**: snapshot `226d70b7c94aabca98de16a0be968229`, generation `b31985ff58878132`, 20/20 FastMCP smoke, 47.1 s total and 4,070 MiB peak RSS. The exact-path/symlink refusal was added after the pilot's release build; its focused Rust/Python tests and the subsequent `just test-all` passed. A read-only Delta query of that pilot snapshot returned zero `behaviors` rows with `budget_reached` (code 6). `git diff --check` **passed** before the review was saved.

| Priority | Change | Finding | Acceptance evidence |
|---|---|---|---|
| Correctness | Carry structural condition IDs, row-local approximation and checked entry/test-use links to served outcomes | F03 | Negative counterexamples and a generation-pinned FastMCP query with unknown on missing proof |
