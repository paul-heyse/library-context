---
id: ADR-0024
title: Use bounded decision diagrams for behavioral conditions
status: proposed
date: 2026-09-24
supersedes: []
superseded-by: null
design: [§B10, §3.9, §9.9]
evidence: Proposed
revisit: The pilot's BDD node limit is hit more often than the Stage 2 DNF budget, or a registered condition question needs an unjustified stability relation.
---

## Context

Stage 2's 16-conjunction by 8-literal DNF budget loses 66 pilot claims to
`budget_reached` after the sound per-site identity correction. Stage 3 composes call paths and
needs compatibility and implication for Q09. A rendered DNF cannot be the authority for these
decisions: expansion may be much larger than the Boolean function. DESIGN §3.9 declares syntactic
identity and defers compatibility; §B10 calls the finite evaluator "not a solver". Evaluation
identity must stay per-site because a call or nonlocal write can change a name even when its
static definition set is unchanged (Stage 2.9 compact review F01).

The read-only [pilot conversion survey](../design_review/evidence/2026-09-24_bdd-pilot-survey/README.md)
measured 13,775 stated condition rows converting to BDDs in 0.97s; the one `over_budget`
sentinel row is cited by 237 regions and 846 reaching facts across multiple modules and
has lost their separate source expressions. Converting only stored DNF rows therefore cannot
address that source loss; whether the 66 recorded budget claims fall requires a product pilot.
This is design evidence, not product acceptance.
The focused [test-leaf proof join probe](../design_review/evidence/2026-09-24_test-leaf-proof-joins/README.md)
found both a compound test with several leaf sites and a two-arm `match` whose distinct
synthetic atoms share the same subject span.

## Options

1. **Keep DNF with a larger budget.** This is simpler and preserves its encoding, but it does not
   bound exponential composition or provide implication. Rejected.
2. **Use a general SMT solver.** It could relate scalar values but adds a broad language before
   the product needs one. Rejected.
3. **Use `biodivine-lib-bdd` 0.6.3 with a narrow typed theory, chosen.** The 2026-09-24 probe in
   the forward handoff §7.3 confirmed canonical Boolean equality, factoring, compatibility and
   a node limit; it also found an unbounded stress case that ran past ten minutes. Product
   integration and pilot measurement remain required.

## Decision

- Conditions are Boolean functions over evaluation identities. Each persisted condition has its
  own sorted support of encoded atom identities and a structural Merkle id over the root and
  child nodes; a library-local `BddVariable` index is never an id. Operations transfer operands
  into the sorted union vocabulary. Same-spelling tests at different sites remain
  distinct. Cross-site equality requires an effect-stability witness; type equality or an
  unchanged static definition set alone is insufficient. Without a witness, the atoms stay
  independent and a negative conclusion is unavailable.
- The Rust kernel owns `and`, `or`, `not`, `given`, `implies` and `compatible`. Before
  constructing a variable set or transferring, it checks support count and input nodes times
  support against a work budget. Before every binary apply, including theory conjunction, it
  checks the input-node product and remaining invocation work; it then calls the library's
  `binary_op_with_limit` for the result-node cap. The pinned source's apply uses a memoized
  pair-of-nodes worklist. `not` is allowed only on an already capped diagram and is linear in
  its nodes. Display takes a capped number of paths from the lazy `sat_clauses` iterator, never
  `to_optimized_dnf` on an unbounded result. A preflight refusal and a library node-limit hit
  are distinct `budget_reached` reasons; neither is false. There is no hard wall-clock guarantee.
- `given` nominates a quotient using the bounded Stage 2 rule and verifies
  `original == factor & quotient` with capped operations. If it cannot verify, it returns the
  original condition with an explicit `not_factored` status, never a falsely simplified claim.
  Implication is incompatibility of `a & !b`; compatibility is satisfiability of `a & b`.
- A lossless diagram-node relation in the `flow` family is authoritative. A condition row names
  its root; terminals have fixed ids; each nonterminal names an atom identity and its low/high
  child ids. Publication and native load validate closure, acyclicity, strictly increasing atom
  order on every path, distinct children, unique reduced nodes and recomputed Merkle ids.
  The `cpg-flow` translator must construct and compose diagrams **before** Stage 2's
  `Condition::from_dnf` limit can discard an expression. Converting the stored DNF after
  extraction is insufficient; the flow-model consumer migrates to the same root/node authority
  in the schema change.
  DNF text becomes a bounded display rendering with a truncation marker, neither the raw id
  input nor serving's sole input. Condition ids and the flow schema migrate together. The bundle
  carries the root/node closure and a kernel-format version.
- `cpg-flow` emits `flow_test_leaves` from each provider predicate **before** the current
  span-only `flow_tests` deduplication. Each row identifies the snapshot/module/scope, provider
  predicate identity, test span and condition root, and one leaf evaluation atom identity and
  leaf span. A compound test has multiple rows; separate `match` arms with one subject span
  retain separate synthetic predicate/atom rows. `flow_tests` remains a test-span projection,
  not the proof authority. Only a stated root with validated leaf support can authorize a
  proof row. A leaf without a justified operand-use mapping remains available as a Boolean
  atom but cannot support a typed exclusion.
- A new attributed test-use observation joins `flow_test_leaves` to the exact place-use span and
  its Pyrefly type term. The current `type_observations` roles do not supply this join. The typed
  theory's exact-type proof origins are a closed whitelist: a modeled source literal or a
  modeled `type(x) is <builtin>` runtime guard. A same-evaluation identity fact proves only
  value stability; it cannot establish exact builtin class or standard equality. A Pyrefly term may
  reject an impossible candidate under the stated type-trust model, but `int`/`str` annotation
  or inferred narrowing alone never proves exact runtime class or standard equality. Any new
  origin needs its own checked rule and counterexample. `Any`/`Unknown`, dynamic aliases,
  possible subclasses, custom `__eq__` and intervening effects leave the relationship
  unknown. The raw Boolean condition id is independent of theory-conditioned verdicts; the
  latter cite the theory revision and proof witness. No arbitrary Python expression is evaluated.
- The `flow_test_types` relation cites `flow_test_leaves.fact_id` and its leaf evaluation atom
  identity, exact `flow_uses.use_id` and operand span, type-term id, proof-origin code and cited
  fact ids. A test-span id alone cannot identify which BDD variable the proof constrains. Its
  Rust producer queries Pyrefly's
  expression trace at the exact operand-use span and joins by source identity in the same
  module/snapshot; the
  current `type_observations` table alone is insufficient. An ambiguous or absent trace emits
  no positive proof.
  A shared publication validator checks the leaf row's unique predicate/atom identity and that
  its atom is in its own condition root's support. It checks the same-snapshot/module use,
  operand/leaf spans and role, type term, origin whitelist and cited facts before an exclusion
  may use it. A synthetic pattern atom receives no place-use proof merely because its subject
  span contains a use; ambiguous attribution stays unknown.
- An `approximated` flag records ty's ambiguous terminal or another stated assumption. Query
  results report budgets and whether approximation affected them.

DESIGN §B10, §3.9 and §9.9 are amended with this decision. The spike must reproduce existing
condition answers, factoring, shuffled-input determinism and limit behavior. The product gate
measures pilot `budget_reached`, limit hits and compile time.

## Consequences

Summaries and serving can ask semantic questions of the same Rust authority, and compact
Boolean functions need not be discarded for DNF expansion. Node serialization, stable-place
proofs and migration checks become new obligations. A capped renderer is deliberately
incomplete; clients use the diagram-backed result to decide. Old DNF ids cannot silently mix
with the new scheme.
