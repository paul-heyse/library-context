# Stage 3 raise-escape boundary — standard design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | ADR-0027, DESIGN §3.9/§9.9 and the `raise_sites.escapes` producer/consumer |
| Standard | Core 2.0, code-intelligence profile 1.0, library-context binding |
| Tier · purpose | Design · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the conservative local escape rule |

The target is a truthful local guard when an explicit raise may leave its function. The baseline in ADR-0022 parsed raised and handler names from source text and assumed unknown context managers did not suppress. I read that authority, the executable producer and consumer, the new fixture, schema and focused tests. This review covers explicit local raise guards, not call-raised exceptions, handler completion, modeled context-manager actions, summaries or serving. No pilot or integrated gate was run.

## 2. Authority and identity map

| Fact or concept | Semantic type and identity | Authority / owner | Revision boundary | Update path | Derived representations |
|---|---|---|---|---|---|
| Explicit raise | Source site and owning function | Ruff syntax and ty region | Compiled snapshot | Extract/derive | `raise_sites` |
| Enclosing frame | `try`/`with` body span in the same function | Ruff structural clause facts | Compiled snapshot | Extract/derive | Local escape witness |
| Escape status | Positive unframed witness; false is unknown | ADR-0027 and `flow_model.rs:1119-1154` | Compiler output digest | Recompile | Guard, `raises_when` candidate |

| Fact family or relation | Provider and revision | Fidelity | Coverage and unknowns | Identity | Consumers |
|---|---|---|---|---|---|
| `raise_sites` | Ruff/ty under pinned extraction | Structural source plus conservative local derivation | Framed cases are unknown until L2; no claim that a false row is caught | Snapshot, module and byte span | `cpg-core::behavior` guards and findings |

Opaque behavior is handler matching/completion and context manager `__exit__`; no name-text heuristic substitutes for it. Source spans and the snapshot keep the row identity; a changed source or compiler rule rebuilds the snapshot.

## 3. Contracts and invariants

| Invariant or contract | Enforcement point | Failure behavior | Evidence |
|---|---|---|---|
| A definite escape has no enclosing `try` or `with` body in its owner | `flow_model.rs:1119-1154` | Withhold guard and fate | Focused fixture and test `compile.rs:427-485` |
| False does not mean caught | `behavior.rs:291`, which reads only true rows, and schema comment | No positive `raises_when` from false | Source inspection |
| Only a published snapshot is visible | Existing compile/publish path | Failed attempt has no published generation | `attempt.rs:604-649` and existing publication checks |

Absence of a raise site differs from an unknown fate: the site remains recorded with `escapes=false`. This slice offers no semantic equivalence between a text name and a class. Coverage is limited to explicit local raises.

## 4. Derivation and execution

| Stage | Question, projection and output | Method and inputs | Exactness and boundary | Effects and cost |
|---|---|---|---|---|
| Frame collection | Can an enclosing statement alter this raise? Same-module, same-owner `try`/`with` body containment | Ruff clause spans, `flow_model.rs:1119-1143` | Conservative; every such frame withholds | In-memory per analyzed compile; no external effect |
| Raise classification | Is an explicit site unframed? `raise_sites.escapes` | Containment lookup, `flow_model.rs:1145-1152` | Positive only; false unknown | Published with source condition; finite scan |
| Guard use | May a raise condition restrict the normal path? | Filter true rows, `behavior.rs:291` | Framed source cannot become a guard | Existing behavior derivation |

The projection has no graph traversal, weighting, recursion or separate cache. Work is bounded by the finite clause and raise rows; no new process or storage authority exists.

## 5. Journeys

A new explicit raise in an unframed function yields a positive local witness. The same raise under an opaque `with` or `try/finally` remains a site but is withheld as a guard. Editing source or the rule changes compiler identity and republishes through the existing attempt path. A module full of unresolved class references cannot make an enclosed raise definite through spelling. No cross-language round trip or evaluation reference is involved in this slice; the later native serving boundary must recheck proof closure.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Structural source and ADR-0027 own the rule; old text shortcut is removed. | — |
| G2 Semantic fidelity | Pass, scoped | False is explicitly unknown; an enclosing frame blocks a positive escape. | L2 must add cited fates. |
| G3 Validity | Pass | The source frame and site are tied to one function/module; focused counterexamples exercise both directions. | — |
| G4 Hidden behavior | Pass | No source text, ambient exception list or import is used for escape. | — |
| G5 Consistency and recovery | Pass | Existing attempt publishes the derived row with one snapshot. | — |
| G6 Transformation and reuse | Pass | Compiler output identity changes with the rule; no stale cross-snapshot reuse. | — |
| G7 Truthful claims | Pass, scoped | A framed raise supplies no `raises_when`; docs state unknown. | Keep false from becoming caught. |
| G8 Library leverage | Pass | Ruff supplies syntax, ty regions; own containment rule is a small domain policy. | — |
| CI-G1 Fidelity | Pass, scoped | Unknown frame behavior is not relabelled as escape or catch. | — |
| CI-G2 Evidence closure | Not applicable | No new served claim or native proof is produced. | Revisit at FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | Fixture is a test input, not a gold compiler input. | — |

## 7. Findings and principle verdicts

No in-scope MUST defect remains after the correction. The next L2 consumer must preserve these deferred boundaries:

| ID | Finding / boundary | Principles · gate | Evidence | Consequence | Correction | Verification |
|---|---|---|---|---|---|---|
| D01 | The boolean cannot express caught versus unresolved. | DP-03, CI-04 · CI-G1 | `behavior.rs:737-758` | Reading false as caught would yield a false negative. | Add typed L2 fate and reason before supporting catch/refutation. | Shadowed class, `finally` and context-manager cases. |
| D02 | A positive unframed escape is local only. | CI-06 · CI-G1 | `flow_model.rs:1145-1152` | A summary could overstate a caller's exception fate. | Compose with call/model/handler conditions. | Nested-call and candidate-dispatch cases. |

DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/04/06/10/12 are satisfied for this local positive rule by the authority, contracts and gates above. DP-07/12/20 and CI-03/05/07/08/09/11/13 concern later graph, summary or served-claim work and are outside this narrow rule; they are not deemed passed.

## 8. Library-leverage ledger

| Capability | Bespoke code | Candidate built-in | Fit and recommendation |
|---|---|---|---|
| Source frames | Span containment in `flow_model.rs` | Ruff structured syntax facts | Keep Ruff as authority; do not parse source text. |
| Reachability condition | Existing flow model | ty regions and bounded condition kernel | Keep existing region facts; attach L2 fate conditions later. |
| Exception match | Removed string heuristic | Pinned context identities, DataFusion joins, model rules | Build a separate cited relation; no generic parser needed. |

## 9. Alternatives

| Alternative | Meaning and locality | Risk | Evidence and decision |
|---|---|---|---|
| ADR-0022 baseline | Handler/raise meaning duplicated in strings and hardcoded ancestry | Opaque `__exit__` or shadowed names can yield false escape | Rejected; source inspection and focused counterexamples |
| Conservative frame boundary | One structural local rule; later L2 owns proof | Fewer positives until L2 | Selected; focused tests passed |
| Full matching immediately | Adds class, order, completion and model logic together | Cannot close absent source evidence yet | Later Stage 3 destination |
| Simplest/library-owned alternative | Treat every explicit raise as unknown | Sound but loses even unframed guards | Rejected; Ruff/ty facts suffice for the narrower positive rule |

## 10. Verification plan and current status

| Claim or risk | Evidence label | Test / analysis | Expected and result |
|---|---|---|---|
| Opaque manager, `finally` and handler withhold; unframed raise remains | Tested 2026-09-25 | Focused release Nextest `unresolved_frames_withhold_raise_escape_guards` | `passed` |
| Existing guard/normal-path behavior remains | Tested 2026-09-25 | Seven selected behavior and handler Nextest cases | `passed` (3 + 4 selections) |
| Package lint | Tested 2026-09-25 | `cargo clippy --release -p cpg-core -p cpg-schema --all-targets -- -D warnings` | `passed` |
| ADR records | Tested 2026-09-25 | `just adr lint` | `passed` |
| Integrated Stage 3 and pilot | Not run | `just test-all`; `just pilot` | `not_run` until functional scope assembled |

## 11. Authority changes and exceptions

ADR-0027 supersedes ADR-0022's raise-escape shortcut and retains its remaining behavior-model decisions. DESIGN §3.9 and §9.9 state the narrower rule. No SHOULD exception is needed.

## 12. Decision

**Accept the conservative local escape rule.** The old heuristic allowed an unresolved frame to become a positive guard. Structural withholding closes that defect while preserving an unframed positive case. D01 and D02 are mandatory boundaries for the remaining L2 and summary work.

| Priority | Change | Findings | Acceptance evidence |
|---|---|---|---|
| Correctness first | Keep false as unknown and add cited L2 fates | D01–D02 | Focused source/withholding tests, then Stage 3 integrated gate |
