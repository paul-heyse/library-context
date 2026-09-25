# Stage 3.0 entry-value bridge — compact change review

## 1. Scope and decision

**Subject:** `flow_test_value_links` contract, direct-formal producer, publication validator and fixture. **Standard:** core 2.0, code-intelligence 1.0, library-context binding. **Tier/purpose:** change/conformance. **Reviewer/date:** Codex, 2026-09-24. **Decision:** accept the narrow positive-link relation; do not use it for typed exclusion or a served compatibility verdict yet.

The baseline had an attributed test operand and a Pyrefly type observation, but no operation-entry value identity. I inspected the schema, producer SQL and admission code, publication path, negative fixtures, `just test-all`, and one fresh FastMCP pilot. I did not inspect every Python AST lowering rule or prove the runtime effect abstraction complete; the implementation consequently admits only a conservative source interval and exposes no negative verdict.

## 2. Fact and fidelity

| Relation | Authority and fidelity | Identity, coverage and consumer |
|---|---|---|
| `flow_test_types` | Pyrefly trace at an exact structurally attributed operand; observation, not runtime class proof | Leaf/use/source keys; absent traces produce no link |
| `flow_reaching`, `flow_definitions`, `parameter_syntax` | ty flow and extraction facts under the stated flow model | One non-approximated parameter definition and one reaching fact required; incomplete module flow coverage excludes links |
| `flow_test_value_links` | Derived by `cpg-core::entry_links`; `DirectParameterReachNoEffect` is the sole current origin | Snapshot plus operation/formal/leaf/use key, cited raw fact ids and effect-rule digest; currently consumed by publication validation, with semantic serving still planned |

An absent link means unknown. The analysis does not claim that Pyrefly's type term is an exact runtime class. New link IDs are derived from source identities; codebook codes are append-only and the new Arrow contract has a reviewed insta snapshot.

## 4. Derivation and execution

The producer selects public operation test uses with complete flow coverage (`entry_links.rs:70-88`), joins reaching definitions to an actual parameter declaration (`:89-106`), and excludes intervening calls, bindings, effect-bearing syntax and earlier predicates (`:107-149`, `:208-224`). It accepts only one stated, non-approximated, non-loop-carried reaching fact in the same function (`:193-224`). The exact recorded literal docstring span is exempted from the expression barrier; no other expression statement is. The output cites the leaf, operand use, definition and reaching facts, and the effect-rule digest (`:225-249`). There is no recursive search or pairwise closure. DataFusion performs the relational joins; `BTreeMap`/`BTreeSet` implement the domain-specific interval admission. Publication recomputes and compares the complete relation from pinned source views (`validate.rs:123-159`). The analysis result is stored through the attempt's ordinary Delta publication boundary.

## 6. Gates

| Gate | Verdict | Evidence or boundary |
|---|---|---|
| G1 Authority | Pass | Schema is in `cpg-schema`; the stored link is derived from named raw facts and recomputed by the shared validator. |
| G2 Fidelity | Pass in scope | The distinct origin says direct formal reach only. An absent or ambiguous reach is not reinterpreted as a negative result. |
| G3 Validity | Pass | Full-row recomputation rejects a doctored link; the fixture exercises that publication path. |
| G4 Hidden behaviour | Pass | Compilation inspects pinned facts; the analyzed Python package is not executed. |
| G5 Consistency/recovery | Pass | A versioned table joins the ordinary validated attempt and pinned Delta snapshot. |
| G6 Transform/reuse | Pass | Source identities and a rule digest constrain derived links; deterministic row ordering follows admission. |
| G7 Claims | Pass in scope | The documented claim is one positive link under the narrow effect rule, not exact type or compatibility. |
| G8 Library leverage | Pass | DataFusion joins, Arrow/Delta publication and standard ordered maps replace bespoke relational or storage machinery. |
| CI-G1 Fidelity | Pass in scope | Pyrefly observation, flow reach and derived source identity remain separate. Missing links stay unknown. |
| CI-G2 Evidence closure | Not applicable to this slice | No link is served as a user-facing claim. Stage 3.6 must settle this gate before serving. |
| CI-G3 Evaluation integrity | Pass | The FastMCP gold skill does not enter the producer; pilot smoke is operational evidence only. |

Applicable DP-01/02/03/04/08/11/13/14/19/21/22/23/24 and CI-01/02/04/06/07/10/12 are satisfied for the narrow published relation. DP-15 and CI-11 remain open for future typed and served semantics, outside this slice. Graph projection, heuristic and recursion principles do not apply to this direct relation.

## 7. Findings

| ID | Finding | Consequence and disposition |
|---|---|---|
| Deferred E01 | A link cites source reach and an effect-rule digest, but no exact runtime type/value origin or theory witness. | Feeding the Pyrefly type observation directly to exclusion could falsely refute a possible input. Stage 3.0 typed-theory and Stage 3.6 serving must require separate exact-origin and same-value proofs; exercise narrowed-annotation and subclass counterexamples before a negative verdict. |
| Deferred E02 | The conservative interval blocks many true stable paths after ordinary calls, assignments or predicates. | Fewer positive links; absence must remain unknown. Revisit only when a modeled effect/transfer has a cited witness and a consumer needs the additional coverage. |

No in-scope defect requiring revision was found. The key correctness protection is the separation between the positive link and the future typed verdict; removing it would violate CI-02/06. The pilot's 102 links establish occurrence, not precision or recall.

## 8. Library leverage and alternatives

| Capability | Route | Assessment |
|---|---|---|
| Relational candidate discovery and validation | DataFusion SQL over registered Arrow tables | Fits exact joins and keeps query inventory visible. |
| Effect-stability admission | Small Rust interval check over cited source spans | Specialized domain rule; no generic solver is needed for this first origin. |
| Condition algebra for later typed comparisons | Existing bounded BDD kernel | Reuse at the next stage; textual predicate parsing would create a second condition authority. |

Keeping only the preexisting Pyrefly type row would leave the entry identity gap. Treating every same-name predicate as linked is simpler code but unsound after rebinding or effects. The selected positive-only relation takes the narrowest viable path; richer modeled transfers should extend the origin codebook and proof contract, not loosen this origin.

## 12. Decision and verification

**Accept scoped.** `just test-all` **passed** on 2026-09-24: 286 release-profile Rust tests, 101 Python tests, fixture parse, Pyrefly, rules, dependency/gold and ADR checks. The entry fixture exercises direct, docstring, rebound, post-call, prior-predicate and closure cases, plus stored-link tampering. `just pilot build/store-stage3-entry-links-v2-2026-09-24` **passed** on a fresh store: snapshot `fc9dc0f3bc6007fc26ceae6619f2996a`, 102 links, 20/20 smoke briefs; total compile 60.0 s and peak RSS about 3831 MiB in that single host run. Neither check establishes an exact typed contradiction or served semantic API. The next correctness gate is an exact runtime origin with stable-value evidence, then a generation-pinned query that returns unknown when the proof is absent.
