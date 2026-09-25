# Stage 3 call-result provenance — standard design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Proposed ADR-0028 and DESIGN §B5, §3.9, §9.9 call-result bridge |
| Standard | Core 2.0, code-intelligence profile 1.0, library-context binding |
| Tier · purpose | Design · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the proposed source-provenance contract; implementation and acceptance remain open |

I traced a Python value use from `cpg-flow::Walk::sources` to raw `flow_values`, `cpg-core::flow_model`, the `through_call` behavior boundary, and the newly applied model transfer sites. I examined nested calls, callee-versus-argument uses, and keyword argument role/value spans. I did not run a product pilot or certify a summary. The current product still withholds `call_transfer`; the proposed design does not assert one has been discharged.

## 2. Authority and identity map

| Fact | Authority and semantic identity | Revision and fidelity | Consumer |
|---|---|---|---|
| Use and enclosing call path | The pinned `cpg-flow` parse over the analyzed bytes | Per value fact, ordered outer-to-inner AST call spans and operand roles | Raw flow-call-step facts |
| Call and argument roles | Ruff source facts from the same bytes | One call syntax id, one argument role id; argument value span excludes keyword text | Exact bridge |
| Applied model transfer | Committed model rule, pinned target and source call candidate | Candidate model action, not observed completion | L3 summary |
| Composed flow | Summary path over validated call steps | Unknown if any step, role, argument identity, dispatch or budget is unresolved | Future behavior verdict and serving |

The step identity is parent `flow_values.fact_id` plus ordinal, not an AST array index or source text. A span is only a join coordinate until the unique call/argument fact is cited. The authoritative model rule remains the tagged catalog, never its rendered path spelling.

## 3. Contracts and invariants

| Contract | Enforcement point proposed | Failure behavior |
|---|---|---|
| `through_call` iff a nonempty path; ordinals are dense outer-to-inner | Flow emitter and shared publication validator | Reject malformed generation |
| Every path step maps to exactly one `call_syntax` in its module/snapshot | DataFusion exact join with counted candidates | Named unknown boundary for zero/multiple matches |
| An argument step maps to one value span in `arguments` under that call; callee is distinct | Typed role and exact span join | Withhold transfer, never reinterpret callee as argument |
| A use passes unchanged into an argument only when its exact value span and flow proof agree | L3 endpoint rule | Derived/computed argument remains unknown for identity transfer |
| Nested calls compose in path order under bounded condition/graph work | `lctx_analytics::summaries` and condition kernel | `summary_boundaries`, not false or truncated success |

## 4. Derivation and execution

| Stage | Inputs → output | Scope, assumptions and limits | Provenance |
|---|---|---|---|
| Flow producer | One parsed AST sink/use → ordered call frames | A finite AST walk; no source-text parser downstream | Provider/run and parent value fact |
| Source bridge | Call/operand spans → unique Ruff call/argument ids | Byte-identical module; ambiguous or missing joins withheld | Both raw fact ids |
| L3 composition | Validated path + model applications and bindings → summary | SCC iteration and BDD/work caps; open dispatch remains open | Step/model/call witness ids and boundary rows |

The producer must preserve separate callee and argument roles even for `obj.method(x)`. The operand value span is distinct from the keyword's whole authored role span. The `through_call` boolean is a derived compatibility field, not a second source of which call carried the value.

## 5. Journeys

`return cast(object, x)` has one call step: `x` enters the `val` argument and can use the pinned identity model if its source/value bridge and normal completion hold. `return wrapper(cast(object, x))` has two ordered steps; proving only the inner `cast` leaves the outer call unknown. `return f(x + 1)` has an argument role but x is not identical to the argument value. In `return f(x)`, the use of `f` is a callee use, not an input value. `return f(val=x)` needs the value span of `x`, excluding `val=`. A model candidate with an unresolved remainder cannot establish a closed whole-operation verdict.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass, design | Producer AST, Ruff source facts and authored model each have one owner. | Keep separate identities in code. |
| G2 Fidelity | Pass, design | Ordered steps and typed roles preserve distinctions the boolean loses. | Exercise nested/callee/keyword cases. |
| G3 Validity | Pass, design | Unique joins, dense paths and explicit refusal are specified. | Implement shared validator and tamper case. |
| G4 Hidden behavior | Pass, design | All input is pinned source/facts; no summary-time reparse or execution. | — |
| G5 Consistency | Pass, design | Steps and joins are snapshot-local; publication remains atomic. | Test malformed generation rejection. |
| G6 Transformation | Pass, design | Path order and bounded composition are declared; no shortcut equates `through_call` with proof. | Measure bounded work. |
| G7 Claims | Pass, design | Current unknown is retained until exact proof; Proposed is not reported as tested. | Keep served verdict unknown until L3. |
| G8 Library leverage | Pass | Ruff AST/ty facts, DataFusion joins, Arrow schemas and existing BDD/petgraph cover generic parts. | No separate parser or hand-written join engine. |
| CI-G1 Fidelity | Pass, design | Callee, direct argument, computed argument and unknown target stay distinct. | — |
| CI-G2 Evidence closure | Not applicable yet | No served claim uses this proposed bridge. | Trace a Q01/Q03/Q05/Q09 claim at FORMAT 7. |
| CI-G3 Evaluation integrity | Pass, design | Planned fixtures and runtime/Pysa oracles are independent of model catalog and gold. | Keep them separate in implementation. |

## 7. Findings and principle verdicts

**D01, required implementation boundary:** current `flow_values.through_call` names no call. At review baseline, `arguments` stored only the authored role span; the first ADR-0028 source seam now adds a separately validated value span, but no call path. Source inspection establishes that a summary join on the boolean still cannot discharge a specific call transfer. Until call steps are published and validated, the current verdict remains unknown. **D02, deferred to L3:** an exact nested path still does not prove normal completion, candidate closure or identity through a computed argument. The summary kernel must carry those boundaries independently.

DP-01/02/03/04/05/07/08/09/11/13/14/15/18/19/20/21/22/23/24 and CI-01/02/03/04/05/06/07/08/09/10/12 are satisfied by the proposed contract within this design scope. DP-12 and CI-11/13 require the later SCC and served implementation; no product pass is claimed.

## 8. Library-leverage ledger

| Capability | Own code | Qualified library feature | Fit |
|---|---|---|---|
| AST call ancestry and operand role | Domain-specific augmentation of current walk | Pinned Ruff AST already held by `cpg-flow` | Capture once at source; no second parser. |
| Exact source fact join | Declared relation and cardinality rule | DataFusion equality joins, grouping and Arrow columns | Preserve all candidates and reject nonunique matches. |
| Recursive propagation | Domain summary rule | petgraph SCC and existing bounded BDD kernel | Generic decomposition and Boolean algebra stay library-owned. |

## 9. Alternatives

| Alternative | Result | Decision |
|---|---|---|
| Keep only `through_call` | Soundly unknown forever for call-result questions | Insufficient for Stage 3 exit. |
| One nearest call id | Loses outer calls and operand roles | Reject. |
| Reparse source in L3 | Creates competing source interpretation and provenance | Reject. |
| Ordered producer path, exact fact joins | Extra raw rows but complete local attribution and explicit unknowns | Select. |

## 10. Verification plan

The first value-span seam is **Tested** 2026-09-25: focused `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema -p cpg-core -E '…' --no-tests=pass --no-fail-fast` passed 4/4 after reviewing and accepting the `arguments` schema migration. Remaining: run focused direct, nested two-call, callee, computed argument, positional, keyword and starred call-path fixtures. Assert path order and role; exact-one raw joins; unknown on a missing/ambiguous mapping; source-span tamper rejection and extractor-version bump. Then run a bounded summary fixture for normal return versus raise/open dispatch. Integrated `just test-all`, fresh pilot, Q01/Q03/Q05/Q09 and wheel remain `not_run` until the full Stage 3 functional scope exists, per operator instruction.

## 11. Authority changes and exceptions

ADR-0028 adds a source-provenance obligation to §B5 without changing ty as provider or Pysa as oracle. It does not supersede ADR-0027's conservative raise rule or authorize a source-text fallback. No exception to the layered standard is requested.

## 12. Decision

**Accept the proposed source-provenance contract for implementation.** The current boolean-only product remains unknown for call transfers. D01 must land as an enforced producer/bridge relation before L3 can discharge one; D02 remains an explicit summary obligation.
