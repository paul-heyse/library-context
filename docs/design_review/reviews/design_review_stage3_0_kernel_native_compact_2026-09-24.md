# Stage 3.0 condition kernel and native member — compact implementation review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | New `cpg-schema::condition_kernel`, pinned biodivine/PyO3/maturin dependencies, `python/lctx_semantics`, and its Python smoke test, against ADR-0024/0025 |
| Standard | Core 2.0, code-intelligence profile 1.0, repository binding through `standard.toml` |
| Tier · purpose | Change · conformance |
| Reviewer · date | Independent design-review agent · 2026-09-24 |
| Decision | **Accept for the foundation slice** after the corrected-tree disposition in §12; the initial pass was Revise |

**Outcome sought.** A bounded, structurally identified Boolean kernel and loadable CPython 3.14.7 extension sharing that kernel. The slice does not yet convert extraction to BDD facts, persist/load diagrams, add typed proof relations, or serve `find_operations` semantics.

**Method.** Read the new Rust/Python code, manifests and lock/pin changes, the Stage 2 condition parser, pinned biodivine apply/transfer source, ADR-0024/0025 and standard. Inspected code and tests; did not rerun `just test-all` or pilot while the implementation agent owns those gates. A test in the tree is **Implemented**, not **Tested** in this review until an executed result is reported. No generated artifact was staged by this reviewer.

**Fact/fidelity and analysis boundary (CI).** The Rust kernel consumes Stage 2 `Condition::Dnf` as a source representation and returns a propositional BDD over encoded evaluation atoms. Its structural root/node IDs are derived, not source facts. No Pyrefly exact-use observation, effect-stability proof, typed exclusion or generation evidence is consumed in this slice. `compatible` and `implies` are exact only as Boolean questions over their input atom identities; they do not decide source-value stability or concrete Python feasibility. The native module currently parses raw condition encodings rather than a pinned generation. These distinctions govern F03.

## 6. Gates

| Gate | Verdict | Independent evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass in slice | `cpg-schema::condition_kernel` owns Boolean operations and structural IDs; PyO3 calls it, with no Python twin | — |
| G2 Semantic fidelity | Pass in slice | Per-site atom strings remain distinct; union transfers are by name in a shared atom order. No theory or entry-value interpretation is claimed by the kernel | Keep native wrapper internal (F03) |
| G3 Validity | **Fail** | `validate_nodes` has no size/depth admission cap before map construction/recursive traversal (F02); malformed generation handling is a claimed contract | Bound and reject malformed input before recursion |
| G4 Hidden behavior | Pass in slice | Kernel operations are in-memory; native module only parses supplied text and calls Rust, no Delta/network/compile route | — |
| G5 Consistency and recovery | **Fail** | Native input parse and public DNF conversion can do unbounded aggregate work before/around per-apply caps (F01); partial remains `None`, not false | Add input and invocation limits |
| G6 Transformation and reuse | Pass in slice | Node IDs hash atom/children; library variable indices are local; `given` returns original unless capped equivalence succeeds | — |
| G7 Truthful capability claims | **Unresolved** | Exported `compatible`/`implies` present raw strings and bare bool/`None`, while ADR-0025 requires typed, generation-pinned, proof-citing verdicts (F03). Foundation-only scope avoids claiming serving completion | Mark API internal and defer serving claim |
| G8 Library leverage | Pass | biodivine owns BDD operations; PyO3/maturin own native bridge/build. Bespoke code is the atom/ID/budget adapter | — |
| CI-G1 Fidelity | Pass in kernel scope | No source test is relabelled as a stable value; raw Boolean results only. User-facing interpretation remains open (F03) | — |
| CI-G2 Evidence closure | N.a. | No served claim, generation row or source proof enters this native smoke path | Review at Stage 3.6 |
| CI-G3 Evaluation integrity | Pass | No gold/reference path is introduced into kernel or native member | — |

**Principle verdicts.** DP-03 and DP-20/CI-08 are **Violated** for F01/F02's admission/resource paths. DP-21/22 and CI-06 are **Unresolved** at the exported wrapper because F03 lacks named reasons and a generation/model context. DP-01/02/04/08/11/13/15/16/18/19/23/24 and CI-01/02/04/10/12/13 are **Satisfied within the foundation scope** by one Rust owner, explicit local identities, checked Boolean operations and pinned package dependencies, with Stage 3 publication/serving behavior excluded. DP-05/06/07/09/10/12/14/17 and CI-03/05/07/09/11 add no separate decision in this slice; CI-11 evidence closure is deferred to a served path.

## 7. Findings

| ID | Finding | Principles · gate | Evidence | Consequence | Correction | Verification |
|---|---|---|---|---|---|---|
| **F01** | Aggregate construction work is not bounded before or across kernel applies | DP-03/20, CI-08 · G3/G5 | `python/lctx_semantics/src/lib.rs:15–17` calls `Condition::parse` before a size check. `condition.rs:568–576` parses all terms, then `from_dnf` enforces 16×8 only at `:439–442` after normalization. `condition_kernel.rs:146–179` accepts public `Condition::Dnf` and limits unique support, then loops over all input terms/literals; `apply` at `:109–122` caps each binary operation, not the invocation total. | A very large raw string or directly constructed DNF of repeated one-atom terms can consume substantial CPU/memory despite every individual BDD apply passing. Returning `None` afterward does not satisfy ADR-0024's bounded-work promise. | Preflight native input bytes and parsed term/literal counts before normalization; enforce an aggregate construction-work budget in `from_condition`, with a named boundary. | Small limits in tests: oversized raw input fails before parse; many duplicate DNF terms refuse before repeated apply; ordinary 16×8 input still works. |
| **F02** | The persisted-node validator can allocate/traverse arbitrary supplied graph size and recurse without a depth guard | DP-03/20, CI-08 · G3/G5 | `condition_kernel.rs:60–99` inserts every supplied row into a map before checking its count; `visit` recursively descends until the first invalid edge. It does not enforce MAX_NODES/MAX_ATOMS or a maximum path depth on input. | A corrupt native bundle with many nodes or a long lexically increasing atom chain can exhaust memory or stack before it is rejected, at the very boundary designated to validate it. | Reject node and atom counts before map allocation; use bounded iterative traversal or a checked depth counter, then verify root closure/order/reduction/hash. | Corrupt over-limit node list and deep ordered chain return a typed validation error without panic; valid kernel-produced closure passes. |
| **F03** | The exported native functions can be mistaken for the ADR-0025 serving executor, but return no generation, proof or boundary reason | DP-21/22, CI-06/11 · G7 | `_native` exposes `compatible` and `implies` on raw strings (`src/lib.rs:20–42`), `.ok()` folds every `KernelBoundary` to `None` (`:17, :26, :34`), and `__init__.py` exports those names. `test_native_semantics.py:7–15` checks bool/None only. No generation is loaded. | A caller can use unanchored strings as if Q09's entry-value constraint were evaluated, or cannot distinguish source over-budget from node/work/transfer failure. The extension cannot yet support the named `unknown` and cited-evidence contract. | State that these functions are internal kernel smoke APIs; avoid wiring them into FastMCP. Before serving, expose typed request/result with operation/formal and generation IDs, proof/evidence, verdict and boundary/budget reason. | Server integration test must reject or mark unanchored input unknown and report distinct budget reasons from one pinned generation. |
| **F04** | Editable native build output is unignored source-tree content | DP-19/21 · G5 | `python/lctx_semantics/python/lctx_semantics/_native.cpython-314-x86_64-linux-gnu.so` is 1.8 MiB and untracked; `.gitignore` excludes `/target/` but no extension binary; `git check-ignore` gives no rule. | A host-specific compiled artifact can accidentally be staged with the new package and conflict with a clean wheel or another interpreter/platform. | Add a package-scoped ignore for native build products; keep generated binary out of source commits. | `git check-ignore` names the rule and `git status` shows only source/lock changes. |

**Additional precision check.** Stage 2 permits 16 conjunctions of eight literals, hence up to 128 distinct atoms. `MAX_ATOMS=64` (`condition_kernel.rs:8, :157–159`) may turn a currently stated condition into `AtomLimit`. This is a declared unknown rather than a false answer, so it is not a soundness defect in this foundation slice; the Stage 3.0 pilot should count such regressions before calling the migration precision-improving. No pilot measurement was available to this reviewer yet.

## 8. Library-leverage ledger

| Capability | Bespoke code | Library fit | Verdict |
|---|---|---|---|
| Boolean apply/transfer | Narrow adapter in `condition_kernel.rs` | Pinned biodivine 0.6.3 supplies canonical fixed-order BDDs, limited binary apply and transfer; it does not supply source atom identity or budget policy | Appropriate |
| Python extension/build | Thin PyO3 module and maturin member | Pinned PyO3 0.29.2/maturin 1.15.0 cover CPython 3.14.7 extension mechanism; package lock and wheel route are in place | Appropriate |
| Input/graph admission | Bespoke domain checks | Parser and BDD library do not impose the repo's aggregate work/node-closure contract | F01/F02 require a small explicit adapter check |

## 12. Decision

**Decision: Revise the foundation slice.** The Boolean identity and library reuse are sound within the tested shapes, and the native package is a real Rust/Python bridge. F01/F02 leave inputs to the public kernel and validator outside the promised resource bound. F03 marks the exact scope of the current wrapper; F04 prevents a build artifact from entering source control. These corrections are local to the foundation slice; the typed theory, persisted node relation, Stage 3.6 semantic filters, full test-all/pilot and registered evaluation remain separate gates.

| Priority | Change | Findings | Acceptance evidence |
|---|---|---|
| Correctness/resources | Bound parse/construction and persisted-node validation | F01, F02 | Refusal tests and no panic on oversized/malformed inputs |
| Truthful boundary | Mark smoke functions internal; design typed serving result later | F03 | Package docs and no FastMCP use of raw string/bool API |
| Publication hygiene | Ignore editable native binary | F04 | `git check-ignore`, clean staged source list |

**Deferred friction.** Cross-site typed theory, exact-use/value-link proofs, Delta/node publication, generation hydration, semantic filters and whole Stage 3 evaluation are not implemented in this foundation slice; ADR-0024/0025 and the standard design review own their planned contracts. Their absence is not treated as a failure of this scoped foundation review.

### Corrected-tree disposition (2026-09-24)

The findings and first gate table above describe the initial implementation. The author corrected the source during review; the assessment below supersedes the initial decision while preserving each counterexample and correction rationale.

| Finding | Inspected correction | Executed evidence or remaining limit | Status |
|---|---|---|---|
| F01 | `diagram` rejects input over 64 KiB before `Condition::parse` (`src/lib.rs:16–22`). `Diagram::from_condition` counts terms/literals, caps encoded atom bytes and distinct atoms, and decrements aggregate pair-work across every construction apply (`condition_kernel.rs:160–215`). The atom cap is 128, covering Stage 2's maximum distinct support. | Oversized raw input and direct DNF tests are present; executed focused/full results to be added from implementation agent. This bound covers the smoke text and DNF construction, not future generation-file loading. | Corrected in foundation source |
| F02 | `validate_nodes` rejects >50,000 rows or >4,096-byte atom names before map construction, stops traversal past 128 levels, and counts global distinct atom support across the reachable closure (`condition_kernel.rs:63–123`). It still checks closure, acyclicity, order, reduction and IDs. | Oversized row/atom and a branching 129-atom graph with path depth ≤128 are in source; future bundle readers must cap bytes before decoding an Arrow file. | Corrected in foundation source |
| F03 | PyO3 exports `probe_compatible`/`probe_implies`; Rust and Python module docs label them developer smoke probes with no generation/proof/typed verdict (`src/lib.rs:1,25–39`, `__init__.py`). No FastMCP call uses them. | `None` still intentionally folds boundary reasons in the smoke API. The served query API remains a separate Stage 3.6 implementation gate. | Scoped and truthfully named |
| F04 | `python/lctx_semantics/.gitignore` ignores the native `_native*.so`; `git check-ignore -v` confirms the generated CPython 3.14 binary is excluded. | Wheel/source inventory remains a packaging gate before commit. | Corrected |

| Gate | Corrected-tree verdict | Basis / remaining gate |
|---|---|---|
| G1 | Pass | One Rust Boolean authority |
| G2 | Pass in foundation scope | Site atoms and structural IDs retain meaning; no typed theory claim |
| G3 | Pass in foundation scope | Construction and node-validation admission checks now reject oversized inputs |
| G4 | Pass | In-memory developer probe; no hidden store/network path |
| G5 | Pass in foundation scope | Bounded construction and validator; no publication path yet |
| G6 | Pass | Canonical structural root and checked factoring remain |
| G7 | Pass in foundation scope | Smoke probes are explicitly distinct from generation-pinned serving |
| G8 | Pass | Generic library roles unchanged |
| CI-G1 | Pass in foundation scope | Raw propositional questions only; source-value claims deferred |
| CI-G2 | N.a. | No served generation claim in this slice |
| CI-G3 | Pass | No evaluation input route |

**Final principle verdicts.** The initial DP-03, DP-20 and CI-08 violations are **Satisfied** in the corrected foundation source by the stated admission limits; DP-21/22 and CI-06 are **Satisfied within the developer-probe scope** by explicit naming and documentation. The other applicable principle verdicts in §6 remain Satisfied for this scope. A future served path must meet CI-11 and the rest of ADR-0025 independently.

**Current decision: Accept the corrected foundation slice.** Source inspection closes F01–F04 at this scope. This does not certify Stage 3.0 persistence, typed theory or Stage 3.6 serving.

**Executed checkpoint, 2026-09-24.** `cargo test -p cpg-schema condition_kernel --lib --quiet` **passed 8/8** after the global-support validator correction. `uv run pytest python/lctx_mcp/tests/test_native_semantics.py -q` **passed 1/1** after rebuilding the editable native member. A clean final-tree `just test-all` **passed**: 277/277 Rust tests, fixture generation 1/1, 94/94 Python tests, Pyrefly, seven rule tests, agent/ADR lint (25 ADRs), 65 fixture parses, dependency/fork checks, cargo shear and gold (implementation agent's executed report). The earlier `just test-all` pass is not used as final-tree evidence because its main nextest phase overlapped the global-support edit. `just pilot` **passed** on snapshot `ecf8b9cdc11ebaf4c1dae61fa9b35dee`, generation `88660525d69030df`, with 20/20 FastMCP smoke briefs, 29.0 s extraction, 44.9 s total and 4,227 MiB peak RSS. That pilot still uses Stage 2 DNF in the compiler; it does not measure BDD rows or budget reduction. `uv build --package lctx-semantics --out-dir build/native-wheels` **passed** for sdist and CPython 3.14 wheel; an isolated CPython 3.14.7 `python -I` import from the installed wheel exercised `kernel_format`, `probe_compatible` and `probe_implies`. The sdist inventory contained kernel source and no compiled `.so`, while the wheel contained the expected extension (implementation agent's executed report). `git diff --check` **passed** after this review update.
