# Nominal derivation contracts — bounded review

Independent reviewer Dalton, 2026-09-29; core 3.0, code-intelligence 1.1, repository binding.
Design/target review of typed proof declaration, nominal targets, common cycle validation and
PostgreSQL explanation-view lowering. **Outcome: Accept, bounded, after F01 correction.**
Current disposition belongs to [cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition).

## Ownership and contract

Ordinary Rust records declare a rule, optional explicit conclusion field, and premise fields.
Actual nominal field types provide their targets. A scalar masquerading as a reference fails to
compile; model admission checks metadata against physical fields, requires a mandatory conclusion,
and reserves generated view names. An absent optional premise contributes no edge. A record without
an explicit conclusion concludes its own row. The field name supplies the premise role.

The model generates one common invariant across all proof sources. Source identity, conclusion and
premises remain relation-qualified. PostgreSQL lowers the same declarations into `derivations` and
`derivation_premises`; source relation/ID and `UNION ALL` preserve alternatives. Generated references,
sealed validation, receipts and publication grants cover the resulting contract. No producer supplies
a second relation-name registry or untyped proof target.

Existing petgraph 0.8.3 supplies iterative `toposort` over a nominal-keyed DiGraphMap. The exact pinned
source and rust-graphs skill were checked; Context7 documentation supplied an additional API route.
No dependency pin changed. The work bound is admitted before graph allocation; coordinated graph/
validator memory admission remains open. The CLI feature union was regenerated, with the model macro
crate excluded from that union alongside other lower libraries.

## Finding

<a id="F01"></a>
**F01 — Explicit-conclusion steps could cite themselves without cycle rejection (A2/A3, G3/G6).**
The initial proof projection omitted the source step and checked only conclusion-to-premise edges.
A step concluding a distinct node could therefore cite itself, or another mutually dependent step.
Proof now retains source identity and constructs conclusion-to-source when distinct, followed by
source-to-premise edges. Self-concluding records do not gain an artificial self-link. Tests cover an
acyclic positive, a conclusion-level cycle, explicit-step self-reference and mutually dependent step
types. The reviewer independently reinspected and reran model and real-PG controls.

A1 localizes declaration and validation ownership; A2 encodes nominal targets and source identity;
A3 composes declarations across proof types without losing dependency edges. A1–A3 are satisfied at
this boundary; the prior G3/G6 failures are resolved.

## Evidence and limits

**Tested, 2026-09-29**, commands prefixed by `python3 scripts/build_environment.py --`:

- Reviewer independently ran `cargo test --release -p lctx-model --test domain_derivation`: passed,
  three controls, including all corrected source-identity cases.
- Reviewer independently ran `cargo test --release -p lctx-postgres --test domain_derivation`: passed,
  one real PG18 test covering four cases, pre-publication access refusal, post-publication grants,
  optional-premise omission, nominal conclusion targets and cyclic publication refusal.
- Reviewer independently ran `cargo test --release -p lctx-model --doc domain::derivation`: passed,
  paired positive/compile-fail declaration controls before the graph correction; that correction
  changes proof edges, not declaration admission.
- Author ran the transfer PG18 test with generated selection views and both typed premise targets:
  passed. These remain contract fixtures, not P4 producer or serving qualification.

Serving explanations, P4 consumers, full resource accounting and assembled P0 acceptance are outside
this receipt. Formatting/lints/integrated gates were not_run because functional scope is incomplete.
