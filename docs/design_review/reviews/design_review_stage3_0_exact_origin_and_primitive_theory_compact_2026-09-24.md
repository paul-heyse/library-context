# Stage 3.0 exact origin and primitive theory — compact change review

## 1. Scope and decision

**Subject:** resolved builtin `type(x) is C` source atom, `flow_test_exact_origins`, the
`StableAfterExactTypeGuard` value-link origin and `primitive_theory::refute_exact_input`.
**Standard:** core 2.0, code-intelligence 1.0 and the library-context binding. **Tier/purpose:**
change/conformance. **Reviewer/date:** Codex, 2026-09-24. **Decision:** accept the narrow
positive proof rows and the unserved exact-input evaluator; keep native negative serving gated
on the runtime-builtin assumption and validation boundary below.

I inspected the translator, lexical-reference join, raw flow and condition contracts, producer,
publication recomputation, fixture controls and primitive BDD operations. This review does not
certify Python runtime builtins in an embedding application, the broader models catalog,
source-source constraints, or a served FastMCP/native response. The integrated Stage 3 repository
and pilot gates remain `not_run` by operator direction.

## 2. Facts, fidelity and authority

| Fact | Authority and scope | Consumer |
|---|---|---|
| Resolved `type_is` leaf | `cpg-flow` atom, admitted only when both references resolve as builtins; its operand is the inner `x` use | Checked entry bridge |
| Entry-value link | One stated, unique parameter reach plus an effect interval. Origin 1 exempts only its own resolved `type(x)` call; origin 2 cites a guard origin and a reaching-path condition entailing its true atom | Exact origin and primitive evaluator |
| Exact-class origin | Derived from an origin-1 link and its leaf; class holds under the atom's true assignment in the standard-builtin-namespace model | Origin-2 link; later semantic serving only with an explicit model assumption |
| Exact-input refutation | Query-supplied primitive value assigns only source atoms with matching checked links. Bounded BDD conjunction returning false permits refutation; a satisfiable remainder is unknown | Stage 3.6 native executor, still unbuilt |

Pyrefly's test type remains an observation. A missing link is not a negative relation. The two
proof tables are analysis results with content-derived IDs, cited source facts and an effect-rule
digest; the shared publication validator recomputes their full contents from the pinned raw
views. The new codebook code is append-only and the schema change has reviewed snapshots.

## 3. Derivation and counterexamples

The source guard requires one argument, both lexical builtin resolutions and the inner operand
use. Shadowed `type` or class names stay opaque. The guarded bridge requires the actual call
syntax row and withholds a link after an unrelated call. A later test must have a unique,
non-approximated, non-loop-carried formal reach. Its reaching fact supplies the **path**
condition: the test leaf's own condition contains only its local predicate and cannot prove the
enclosing branch. The path diagram must imply the exact guard atom. All other earlier calls,
bindings, effect-bearing syntax and tests remain barriers; a compound guard does not qualify.

The fixture has a nested positive case, shadowed names, a call before the guard and a call
inside its true branch. Publication admits the nested link and withholds both call cases.
Stored-class tampering is rejected. The primitive controls distinguish unequal exact strings
from `True == 1`, and require an explicit `StandardAssumed` builtin namespace choice before a
`type_is` refutation. The result carries the link IDs used. It does not claim concrete Python
feasibility from a satisfiable BDD.

## 4. Gate verdicts and principle verdicts

| Gate | Verdict | Reason |
|---|---|---|
| G1 authority | Pass in scope | Source rows and the schema own meaning; proof rows are fully recomputed. |
| G2 fidelity | Pass in scope | Pyrefly type, exact class under a model, source value identity and Boolean atom stay distinct. |
| G3 validity | Pass for publication; unresolved for serving | The shared validator rejects stored tampering. The primitive helper takes slices and relies on its caller to establish that validation occurred; the native executor is not yet built. |
| G4 effects | Pass in scope | The analyzed Python fixture is parsed, never executed. The interval conservatively blocks unmodeled effects. |
| G5 publication | Pass in scope | Both tables join one validated Delta attempt; no partial result is published. |
| G6 transformation | Pass in scope | Cited reach, guard and path-condition IDs preserve the derivation; BDD node/work caps return a boundary. |
| G7 claims | Pass for this scoped claim; unresolved for serving | Focused tests warrant proof-row behavior. No end-to-end pilot or native verdict is claimed. |
| G8 library leverage | Pass | DataFusion handles joins, Arrow/Delta handle contracts/publication, biodivine handles bounded Boolean composition. Bespoke code is the source-specific effect rule and primitive Python semantics. |
| CI-G1 fidelity | Pass in scope | Missing/ambiguous evidence stays unknown and relations retain their distinct kinds. |
| CI-G2 evidence closure | Unresolved for serving | No user-facing negative claim exists; Stage 3.6 must tie a request and all proof rows to one validated generation. |
| CI-G3 evaluation integrity | Pass in scope | Gold-reference skill content is not a producer input. |

Applicable DP-01/02/03/04/05/07/08/11/13/14/15/16/18/19/21/22/23/24 and
CI-01/02/04/06/07/10/11/12/13 are satisfied **for the published positive relation**.
DP-03, DP-15, CI-11 and CI-13 remain unresolved for the future served negative result because
its validation, assumption and generation binding do not exist yet. DP-09/10/12/17/20 and
graph-specific CI principles introduce no new obligation in this local nonrecursive slice.

## 5. Findings and disposition

| ID | Finding and consequence | Disposition |
|---|---|---|
| Deferred E03 | Lexical builtin resolution does not prove that an embedding application's runtime `builtins.type`, `str`, `int` or `bool` still name standard objects. Serving a negative class verdict without stating and enforcing that model could falsely refute a value. | The primitive API requires an explicit `BuiltinNamespace::StandardAssumed`; `Unknown` withholds this rule. At Stage 3.6, declare and expose the assumption in the generation/request contract or prove a narrower source guarantee before using the origin for a negative verdict. |
| Deferred E04 | The unserved primitive helper accepts proof slices and cannot itself know whether native load validated them. A future caller could bypass publication checks and refute from doctored rows. | At native-executor construction, hydrate only one validated generation and expose a handle that owns checked rows; exercise malformed proof and mixed-snapshot load rejection before tool acceptance. |
| Deferred E05 | The interval and single-guard rule withhold many real stable paths after other predicates or effects. | Missing links remain unknown. Revisit only when a model supplies a cited effect/transfer witness and a registered question needs the coverage. |

The strongest counterexample tried in this slice is a nested test after an unknown call: the
reaching parameter alone still exists but the value link is withheld. A runtime-builtin mutation
counterexample is not executable inside the repository's never-execute-fixtures rule; E03 keeps
that semantic limit visible.

## 6. Library choice and verification

The chosen representation uses the existing BDD's capped `and`/`implies` and structural roots.
A second textual condition interpreter or an SMT solver would add generic machinery without a
registered arithmetic/order question. DataFusion candidate joins and the table macro keep new
fact contracts aligned with the validator; the small Rust interval check remains domain-specific.

**Tested, 2026-09-24:** focused `cargo nextest run --release --workspace` selections exercised
the published nested/withheld/tampered cases, primitive controls, codebook/contract snapshots and
synthesis ledger; `cargo check --workspace` passed. The final focused four-test rerun after the
explicit builtin-assumption parameter passed. `just test-all`,
fresh-store `just pilot`, generation-pinned native query and Stage 3 evaluation are `not_run`.
