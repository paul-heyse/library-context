# Stage 3 summary sibling boundaries — compact review

## 1. Scope and coverage

**Tested (2026-09-25), change/conformance.** This review covers the `summary_boundaries`
derivation, its compiler revision, and one synthetic sibling-origin challenge of a compiled
identity fixture. I inspected the contribution and summary keys, the finite producer's
admission conditions and shared boundary validator. Recursive composition, effect/role
summaries and operation-wide negative closure were not examined here.

## 2–5. Authority, contracts and journeys

`value_flow_contributions` is the authority for distinct source origins; a `summary_flows`
row is one positive, cited may-path. Both can share the raw return fact and condition. The
boundary table has a coarser key, so it cannot say which of two sibling contributions is open.
The derived rule now suppresses a boundary after a positive summary only when there is exactly
one same-callable parameter-origin contribution at that key. With two, it retains the aggregate
boundary. A crossed sibling sets `call_transfer`; otherwise the existing control boundary
applies. The shared publication validator reconstructs this relation. Compiler output version
67 names the changed derivation; no schema or codebook changed.

The ordinary one-origin direct identity still has no boundary. In the targeted test a second
distinct `source_key` is added at the same raw fact/condition after compilation. Re-derivation
adds one `unsupported_control_flow` boundary despite the positive path. The test restores the
original registered relation afterward. This is a relational counterexample, not a claim that
the synthetic sibling came from a Python program.

## 6. Gates

| Gate | Verdict | Basis or remaining action |
|---|---|---|
| G1 authority | Pass | Contributions and summaries remain published authorities; boundary is derived. |
| G2 fidelity | Pass, scoped | A positive may-path no longer hides an unproved sibling. Full origin-specific closure is unresolved. |
| G3 validity | Pass | The shared validator reconstructs the changed SQL relation. |
| G4 hidden behavior | Pass | Pure DataFusion derivation over pinned tables. |
| G5 consistency | Pass, scoped | Version 67 changes compiler identity; integrated publication gate awaits end of scope. |
| G6 transformation | Pass | Grouping by snapshot, callable, formal, raw fact and condition retains multiplicity before suppression. |
| G7 claims | Pass | The boundary is unknown, never a negative transfer verdict. |
| G8 library leverage | Pass | DataFusion counts and joins origins; no custom grouping loop. |
| CI-G1 fidelity | Pass, scoped | An open sibling remains visible. |
| CI-G2 evidence closure | Partial | Aggregate boundary cites a raw fact/condition, not the particular open contribution. |
| CI-G3 evaluation integrity | Pass | The challenge uses analyzed fixture rows and a synthetic counterexample, not gold. |

## 7–10. Findings, alternatives and verification

**F01, deferred:** the boundary key cannot give one row per unproved origin. An origin-keyed
proof relation would be more precise, but requires new summary step and boundary identity
contracts and a schema migration. The conservative aggregate rule is sufficient to prevent
false closure now. Revisit when recursive or operation-wide closure needs origin-specific
negative results. Adding a second Python-side count would duplicate DataFusion meaning.

**Passed, 2026-09-25:** `RUST_MIN_STACK=33554432 cargo test -p cpg-core --test compile
pinned_identity_models_require_and_publish_their_real_formals -- --exact` with the synthetic
sibling case; targeted `cargo clippy -p cpg-core --test compile -- -D warnings`.
Formatting, `just test-all`, fresh pilot, structured evaluation and clean-wheel acceptance are
**not_run** until the entire Stage 3 functional scope lands.

## 11–12. Decision

**Accept the conservative boundary correction.** Keep F01 explicit; this does not prove
negative closure or change a §B decision.
