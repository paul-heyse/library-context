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
static definition set is unchanged (an executable counterexample at Stage 2.9).

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
3. **Use `biodivine-lib-bdd` 0.6.3 with a narrow typed theory, chosen.** A standalone 2026-09-24
   probe confirmed canonical Boolean equality, factoring, compatibility and
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
  The Arrow row has `snapshot_id`, `fact_id`, `module_node_id`, scope kind/name span,
  `predicate_key`, test start/end, `condition_id`, `atom_id`, full encoded atom and leaf
  start/end. `predicate_key` is exactly `H("flow-synthetic-predicate", format!("{fid:?}"),
  format!("{predicate_id:?}"))`, the current synthetic identity's predicate digest;
  snapshot and module scope are separate row columns. The unique source key is
  `(snapshot_id, module_node_id, predicate_key, atom_id)`.
  `atom_id` is a domain-separated hash of the full evaluated `Atom::encode()`, including its
  site/synthetic identity; BDD nodes still persist the full atom string. Equal spellings at
  different sites do not merge. A `Site` leaf span must equal the
  atom's encoded site span; a synthetic pattern leaf records its subject span and its
  encoded predicate key must equal the row's. The validator recomputes the atom's module key
  from the cited source path and original text using `flow-evaluation-module`; a matching
  span or predicate hash with a foreign module key is insufficient. Producer and validator
  reject duplicate keys, invalid source spans, mismatched identities, missing condition roots or
  an atom outside that root's support. A bounded/unstated root cannot authorize a proof row.
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
- The first theory implementation uses the same bounded BDD kernel to conjoin only
  independently justified primitive constraints. It may separate `is_none` from a proved
  non-`None` singleton, and unequal string `==` atoms only after an exact builtin `str` origin
  and a same-value witness. It does not identify `==` with `is` or assume distinct numeric and
  Boolean literals compare unequal (`1 == True`). A query-supplied exact literal is an explicit
  entry-value input; it need not add a synthetic BDD variable. Checked entry-value links permit
  assignments to source atoms that read that value. The bounded BDD may refute a condition only
  when the conjunction is false; a satisfiable remainder is unknown. A parameter default is not
  such an origin when an explicit argument can replace it. If proof is absent or capped, the typed result is unknown, irrespective of raw
  propositional compatibility. This stays within biodivine's bounded apply; Z3 is reconsidered
  only for a registered question requiring arithmetic or another genuinely non-Boolean theory.
- The exact `type(x) is <builtin>` origin resolves both the `type` call and the class operand to
  their builtins via source reference facts. Its tested operand is the inner `x` use, not the
  whole call span. The one-argument builtin type call may be exempted from an effect barrier
  only under that resolution and a revisioned model. A later test needs a path-specific
  stability witness across the guard; the current direct-link origin's blanket prior-predicate
  barrier is not weakened by inference from source order alone.
  Lexical builtin resolution assumes the runtime builtin namespace retains its standard CPython
  bindings. The compiler does not currently prove that environmental assumption; a served
  refutation depending on it remains unknown until the generation contract declares and checks
  the assumption or a narrower source proof excludes mutation.
- The `flow_test_types` relation cites `flow_test_leaves.fact_id` and its leaf evaluation atom
  identity, exact `flow_uses.use_id`, operand span and role (`tested_place`), type-term id,
  proof-origin code and cited fact ids. A test-span id alone cannot identify which BDD variable
  the proof constrains. Its Rust producer queries Pyrefly's expression trace at the exact
  operand-use span and joins by source identity in the same module/snapshot; the current
  `type_observations` table alone is insufficient. An ambiguous or absent trace emits no
  positive proof.
  The `tested_place` join is a structural match from the atom's modeled operator to the
  corresponding operand AST node and its `flow_uses` row, not a choice of any use contained
  by the test span. For a synthetic pattern, the provider predicate id and pattern subject
  node must establish the mapping separately. Annotation uses and sibling operands are never
  candidates by proximity alone.
  A shared publication validator checks the leaf row's unique predicate/atom identity and that
  its atom is in its own condition root's support. It checks the same-snapshot/module use,
  requiring exactly one `flow_uses` row for the proof's use id, operand span and decoded
  `tested_place`; it also checks leaf span, type term, origin whitelist and cited facts before
  an exclusion may use it. A synthetic pattern atom receives no place-use
  proof merely because its subject span contains a use; ambiguous attribution stays unknown.
- An `approximated` flag records ty's ambiguous terminal or another stated assumption. Query
  results report budgets and whether approximation affected them.

DESIGN §B10, §3.9 and §9.9 are amended with this decision. The spike must reproduce existing
condition answers, factoring, shuffled-input determinism and limit behavior. The product gate
measures pilot `budget_reached`, limit hits and compile time.

## Consequences

**Partial implementation evidence (2026-09-24, Tested).** The source translator now emits a
resolved `type_is` atom with the inner operand use. The guarded entry-value link and separate
exact-class origin are published with cited source identities and recomputed by the shared
validator. A focused fixture confirms shadowed names and a preceding unknown call withhold the
origin and that a doctored stored class is rejected. A second focused fixture proves one later
test use under the exact guard by the reaching path condition, and withholds it after an unknown
call. The exact-input evaluator uses the bounded BDD to refute an incompatible source condition;
numeric/Boolean cross-equality and missing links remain unknown. Source-source theory,
generation-pinned native serving and pilot acceptance remain open, so this ADR remains proposed.

Summaries and serving can ask semantic questions of the same Rust authority, and compact
Boolean functions need not be discarded for DNF expansion. Node serialization, stable-place
proofs and migration checks become new obligations. A capped renderer is deliberately
incomplete; clients use the diagram-backed result to decide. Old DNF ids cannot silently mix
with the new scheme.

**Current role (2026-09-25).** This record still carries a live choice: the §B10 allowance for a
bounded decision-diagram kernel and a narrow typed theory, the move of condition identity from
the Stage 2 DNF encoding to structural diagram ids, and the kernel's operations and budgets. The
code implementing it is not a review of it, so it stays proposed until the increment-end
design/target review accepts, revises or rejects it. Two narrower uses are accepted in ADR-0045:
the persisted, validated analysis condition catalog and the predecessor compatibility screen.
Known kernel defects (decisions refused at the node cap, construction-dependent support,
aggregate hydration retention, DNF-bound factoring, membership and integer equality) are owned by
the [forward plan §6](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
(W6, W11) and must be resolved, or recorded as limits, before acceptance.
