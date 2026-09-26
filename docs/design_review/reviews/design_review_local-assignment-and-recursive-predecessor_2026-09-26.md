# Design review: local assignment reads and recursive predecessors

**2026-09-26 · change/conformance · scoped review.** Core standard 3.0,
code-intelligence profile 1.1 and library-context binding. Subject: Stage 3
order 1's append-only `assignment_name_normal` status in the shared argument
classifier and the real-provider controls for a terminating branch versus an
unconditional recursive predecessor. The [forward plan](../../plans/behavioral-model-forward-plan_2026-09-24.md#3-stage-3-execution-queue)
owns remaining work.

## 1–5. Ownership, contracts and change scenario

The schema-owned query joins a whole direct Ruff name argument, its lexical
resolution, and ty's same-range reaching definition. It admits an assignment
name only when exactly one row remains, with a nonnull, non-approximate,
non-loop-carried definition matching the same lexical assignment binding and
an admitted condition root. It cites `flow_reaching.fact_id`. A possibly
unbound local produces no normal-evaluation witness. The same query feeds
modeled return and preceding-call paths, so a future exact local-name case
changes one owner. The pure finite producer consumes its evidence in source
argument order; the shared validator reconstructs proof identity, and the
native executor receives the validated result. A name spelling alone is never
evidence of a runtime read.

The change scenario is `local = 1; cast(object, local); return value`. Its
argument can complete normally because ty has one exact local definition.
`if flag: local = 1; cast(object, local)` retains an unknown boundary because
the read may be unbound. A second scenario places `return value` in a
terminating branch before a recursive call in the other branch. The bounded
condition kernel proves the call region disjoint from that return path; the
direct path is retained. An unconditional self-call before `return value`
instead blocks the direct path. This verifies the accepted predecessor rule,
not recursive summary composition.

| Relation | Authority | Withholding boundary |
|---|---|---|
| `references` / `reference_resolutions` | Ruff direct name and lexical binding candidate | Flow-insensitive resolution alone cannot certify the read |
| `flow_uses` / `flow_reaching` / `flow_definitions` | ty same-range, exact reaching assignment and condition | Null, multiple, approximate or loop-carried definitions remain unknown |
| `preceding_call_regions` / `summary_flows` | ty call regions; bounded BDD compatibility; pure ordered producer | A compatible unproved recursive call withholds direct flow; disjoint path needs no completion witness |

The implementation uses DataFusion joins/grouping over existing provider
relations and the existing BDD compatibility operation. Duplicating ty's
use-def analysis or adding a separate Python evaluator would add a second
semantic authority. No new crate or adapter is justified. `AssignmentNameNormal`
is a schema/codebook migration; the status is appended without renumbering.

## 6–8. Judgments, gates and decision

A1–A3 and FP-01–FP-06 are **satisfied for this slice**: the extraction and
semantic owners remain distinct, the two summary consumers share the
classifier, and the scoped cases are exercised through publication and
native serving. Applicable DP-01/02/03/08/11/13/15/16/21/23/24 and
CI-01/02/04/06/08/11 are satisfied at this tested boundary. G1–G8 and
CI-G1/CI-G2 **pass for these cases**: the new status has an evidence check,
an append-only code and a real positive/withholding pair. CI-G3 is unchanged:
gold is not an input. General statement/argument evaluation, path-local
predecessor identities, nested calls, possible-raise classification and
recursive composition remain **unresolved** in the enclosing Stage 3 scope.
No new ADR is needed because this is an instance of the accepted typed
argument contract. There is no finding against the slice; the enclosing
limitations stay in plan order 1 and order 6.

**Tested 2026-09-26:**
`RUST_MIN_STACK=16777216 cargo test -p cpg-core --test bundle
finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`
passed the local assignment/unbound and terminating/unconditional recursion
controls through Delta, FORMAT 8 and native query.
`RUST_MIN_STACK=16777216 cargo test -p cpg-core --test compile
pinned_identity_models_require_and_publish_their_real_formals --quiet`
passed the shared classifier's existing modeled-return regression.
`INSTA_UPDATE=no cargo test -p cpg-schema --test codebooks --test contracts
--quiet` passed after reviewing/accepting the append-only codebook, schema
check and validation-rule snapshots. `just fmt`, `just test-all`, fresh
`just pilot` and complete Stage 3 evaluation are `not_run`.

**Decision:** accept this scoped change/conformance slice; Stage 3's assembled
design and product qualification remain open.
