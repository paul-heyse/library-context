# Design review — reference

Companion to [SKILL.md](SKILL.md). Lenses and calibration examples that have repeatedly
turned up real defects. None is a required step. Principle IDs refer to the core design
principles; profile lenses live in the profile's own skill.

## §1 Lenses

### When the subject is a document

**Reconstruct rather than read.** Build the template's tables yourself from the document
instead of checking whether the document's own versions look complete. Every cell you have to
invent is a decision the design has not made, and the list of invented cells is the evidence:

- the **authority map** (slot 2): one row per semantic fact, with owner, revision boundary and
  update path;
- the **invariant table** (slot 3): enforcement point and failure behaviour per invariant. An
  invariant with neither is unresolved under DP-03, whatever the prose claims;
- the **stage table** (slot 4): inputs, observed dependencies, output contract, effects and reuse
  boundary per stage. Undeclared effects fall under DP-18;
- the **absence lattice** (DP-02): for each value that can be missing, which of not supplied /
  not applicable / not computed / unknown / invalid / partial / failed the design distinguishes,
  and which collapse into one representation.

**Sort the load-bearing sentences** to keep claim strength honest:

| Kind | How it reads | What it deserves |
|---|---|---|
| Specification | States what holds, where enforced, what is rejected | Assess directly |
| Intention | A desirable property with no mechanism | Unresolved until a mechanism is named |
| Assumption | Rests on an external system, library or later decision | Unstated assumptions are a DP-22 finding |
| Benefit assertion | Performance, simplicity, extensibility | A hypothesis unless evidence is cited (DP-22) |

**The divergence sentence.** For a core mechanism, the one sentence two implementers would read
differently is often the whole finding: "identity is content-based" (over which canonical form —
DP-04); "invalid input is rejected" (at ingest, at query or at publication — DP-03); "the plan is
cached" (keyed on what, invalidated by what — DP-09); "conversion is lossless" (byte, structural,
semantic or approximate — DP-08).

### When the subject is code

The bar in the second column is what makes a shape reportable. Below it, record the item in the
coverage note as examined and unsettled.

| Shape | What makes it evidence | Gate · principles |
|---|---|---|
| **Second authority** — one fact independently editable in two places (constant and schema, schema and validator, validator and adapter, default and fixture) | Both sites cited, and no derivation, generation step or assertion linking them. Check for a derivation first | G1 · DP-01 |
| **Unguarded boundary** — external input, partial construction or deserialization reaching an operation that assumes an invariant | Entry point, operation, and no rejecting check between them (a warning or opt-in check is advisory). Type-level enforcement counts | G3 · DP-03, DP-08 |
| **Hidden effect** — `validate`, `inspect`, `plan`, `explain` paths that mutate, register, lazily initialize or read clock, environment, filesystem, globals or randomness | The mutation or read, and a caller that reasonably assumes purity | G4 · DP-18 |
| **Silent degradation** — default on absence, catch-alls flattening failure classes, unsupported branches with different semantics, lossy conversions | The branch, what the caller observes in each case, and no declared loss or approximation policy | G2/G7 · DP-02, DP-15 |
| **Incomplete reuse key** — a cache or memo key missing a result-affecting input (policy, provider version, configuration, schema revision, upstream identity) | The key, the omitted dependency, and the change that yields a stale hit. Volatile inputs (timestamps, paths, addresses) in a key are the mirror defect | G6 · DP-09 |
| **Unbacked capability** — the accepted surface (enum, trait, registry, config schema, API) wider than the implementation that handles it | The set difference, with the accepted variant and the fallback cited. A typed `Unsupported` is aligned; a silent semantic fallback is not | G7 · DP-15 |
| **Bespoke generic machinery** — own solver loop, graph traversal, cache, parser, retry framework, derivative routine or hand-rolled built-in | The code, the library or built-in that provides the capability at the pinned version, and no stated reason for building it | G8 · DP-13, DP-14 |
| **Adapter with policy** — a conversion layer holding defaults, selection or domain rules found nowhere else | The rule, and the authority that should own it | G1/G2 · DP-14, DP-01 |

Trace the path that executes — the implementation actually selected, the branch taken under the
real configuration. Where dispatch is dynamic, say which path you traced.

### When the scope computes: reuse, graphs and staged execution

| Shape | What makes it evidence | Gate · principles |
|---|---|---|
| **Second reuse mechanism** — a cache or dependency tracker duplicating one another mechanism already owns | Both mechanisms, and a change one invalidates and the other does not | G1 · DP-09, DP-01 |
| **Handle mistaken for version** — a key on a handle, name or "latest" reference whose contents can change under the same identity | The key, the mutable content, and the edit that leaves the key equal | G6 · DP-09 |
| **Absence untracked** — failed lookups and membership not recorded as dependencies | The lookup, the missing membership dependency, and the addition that stays stale | G6 · DP-09 |
| **Effect inside a memo** — a write, registration or external call inside a body that may be skipped or repeated | The effect and the reuse path that skips or repeats it | G4 · DP-18 |
| **Unsound equality** — reuse on allocation identity, iteration order, loose float tolerance or non-canonical labels | The equality, and two results it equates that a consumer would treat differently | G6 · DP-04, DP-11 |
| **Output filter used as input filter** — a projection built from what the answer should contain | The predicate, the omitted element, and the changed result | G6 · DP-07 |
| **Silent graph reinterpretation** — symmetrized direction, collapsed parallel edges, dropped isolates, a weight read under the wrong meaning | The conversion site and the algorithm requirement it violates | G2 · DP-07 |
| **Index-space or version bridge** — results zipped against another iteration order, implicit compaction, types from two library majors joined by casts | The mapping or its absence, and the two index spaces or versions | G2/G3 · DP-04, DP-15 |
| **Misnamed algorithm** — a routine recorded under the name of an algorithm it does not implement | The entry point, its actual behaviour, and the recorded name | G7 · DP-15 |
| **Heuristic promoted to fact** — communities, rankings or similarity used as dependency, equivalence or execution boundaries | The consuming decision and the missing validation | G2/G7 · DP-07 |
| **Global computation per partition** — a whole-structure analysis run per batch, or a predicate pushed into it | The plan shape and the global result that differs | G6 · DP-08 |
| **Silent truncation** — a limit, iteration cap or sampling returning something indistinguishable from the complete result | The limit, and what the caller sees when it is hit | G5 · DP-12, DP-20 |

For reuse claims, the settling question is always: after the edit that would expose it, would
the incremental result equal a clean recomputation? Answer it by reasoning over the observed
dependencies; a comparison is worth running only where that reasoning leaves real doubt.

### When the subject is both

| Question | Finding shape |
|---|---|
| Do the document's semantics and the implementation's match? | Divergence against the pair; name the authority and the reconciliation (DP-01) |
| Does the document describe a state the code has left? | Stale specification — say whether the document is a target or a record; one that is neither competes with the code |
| Does the code implement semantics the document omits? | Undocumented surface (DP-24) |
| Does the document claim capabilities the code lacks? | G7 — narrow the claim or relabel it *Proposed* |
| Which evidence label does each claim now deserve? | Re-label per principles §D |

## §2 Finding calibration

Adequate and inadequate versions of the same observation. The difference is always the same
three things: a concrete consequence, evidence at the right grain, and citations that do work.

### A — Authority

**Inadequate.** "Violates DP-01: the schema appears in several places. Recommend consolidating."

**Adequate.** "Column nullability for the state-variable table is independently editable at
`catalog/schema.rs:88` and at `ingest/validate.rs:214`, which re-lists required fields as string
literals; neither derives from the other and no test compares them. **Consequence:** making a
field required only in the validator admits nulls the kernels assume absent, panicking at
`solve/assemble.rs:131`. **Correction:** derive the validator's required set from the schema
(~2 call sites). **Verification:** a test asserting the two sets are equal; it fails today.
DP-01, DP-03, G1."

### B — Absence semantics

**Inadequate.** "DP-02 is not fully satisfied; absence states should be clearer."

**Adequate.** "An unsolved variable, a variable outside its domain and a variable whose upstream
computation failed are all null in the result column (§4.3). **Consequence:** a consumer counting
nulls cannot separate a failed solve from a partial one. **Correction:** a typed status beside the
value — one declaration if made before the result contract has consumers. **Verification:** a
negative test asserting the three cases are distinguishable at the result boundary. DP-02, DP-19,
G2."

### C — Over-construction of bespoke machinery

**Inadequate.** Silence, or praise for extensibility.

**Adequate.** "§6 introduces an own provider registry with a capability-negotiation protocol for
one backend. The registry adds a declaration surface and conformance obligations the design does
not fund, and duplicates the selection mechanism the adopted framework already offers.
**Consequence:** the first real second backend needs distinctions the negotiation vocabulary
cannot express, so the registry is rewritten. **Correction:** use the framework's mechanism
behind the existing trait and typed `Unsupported`; remove the own registry. **Verification:** none
needed — this removes machinery. DP-16, DP-13, G8."

### D — Bespoke generic code

**Inadequate.** "Consider using a library here."

**Adequate.** "`solve/newton.rs:40–210` implements a damped Newton iteration with its own line
search for square systems. The adopted nonlinear solver library provides globalized Newton with
scaling and typed failure status at the pinned version; no stated reason explains the bespoke
path. **Consequence:** two convergence behaviours with different failure semantics, and fixes to
one do not reach the other. **Correction:** route square systems through the library; delete the
module and its tests. **Verification:** existing square-system tests pass through the library
path, and the module is gone. DP-13, DP-16, G8."

### E — Evidence labels

**Inadequate.** "The design is validated end to end — see the integration tests."

**Adequate.** "Round trip is *Tested* for the structural case (`tests/roundtrip.rs::schema_roundtrip`,
14 fixtures, exact structural equality) and *Proposed* for metadata, which §3 claims is preserved
but nothing exercises. Narrow the claim or add the metadata case. DP-22, DP-23."

### F — Projection scope

**Adequate.** "The block-analysis projection keeps only equations reachable backwards from the
requested outputs (§4.2), then decomposes cycles on that subgraph. An equation outside the
ancestor set that closes a cycle through two ancestors is dropped. **Consequence:** two coupled
equations are solved sequentially and converge elsewhere or fail, with no diagnostic.
**Correction:** decompose on the enclosing scope, then select every block the ancestors intersect.
**Verification:** a fixture with such a cycle asserting one block of size two; it fails today.
DP-07, DP-12, G6."

## §3 How the slots compress

| Slot | Document subject | Code subject | Change review |
|---|---|---|---|
| 1 Scope and coverage | With method note | With method note | Compressed; keep the coverage note |
| 2 Authority map | Reconstructed; invented cells marked | Where each authority lives | Only if a finding needs it |
| 3 Contracts | Often the core | Enforcement sites cited | Merged into findings |
| 4 Stage table | Reconstructed per stage | From the executing path | Only stages carrying a finding |
| 5 Journeys | Extension and failure at minimum | Same, through real code | One, chosen for relevance |
| 6 Gates | Tabular | Tabular | Tabular |
| 7 Findings | Tabular | Tabular | Tabular |
| 8 Library ledger | Expected | Expected | Rows for new bespoke code |
| 9 Alternatives | Worth the work | Worth the work | Optional; say why omitted |
| 10 Verification | Proposed checks | Existing coverage and gaps | Top gaps only |
| 11 Authority changes / exceptions | Target reviews; deviations | Same | Only if present |
| 12 Decision | Required | Required | Required |
