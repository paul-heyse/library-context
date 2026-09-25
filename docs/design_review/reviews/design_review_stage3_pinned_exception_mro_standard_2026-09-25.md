# Stage 3 pinned exception MRO — standard design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | ADR-0030, DESIGN §B5/§3.2/§9.9, `context_class_mro` and modeled handler class comparison |
| Standard | Core 2.0, code-intelligence profile 1.0, library-context binding |
| Tier · purpose | Design · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the provider-backed positive ancestor rule within candidate-catch scope |

I read the exact pinned Pyrefly `ClassDefinition.mro` and `ClassRef` source at revision `a07b7ba`, the context extractor, schema/rules, DataFusion candidate join and shared validation. The focused `OSError` → `Exception` case exercises a real pinned provider fact and an authored handler. I did not establish a negative class relation, clause selection, handler completion or a served claim. The integrated FastMCP pilot and Stage 3 gates remain outside this review.

## 2. Authority and identity map

| Fact | Authority and fidelity | Revision and identity | Consumer |
|---|---|---|---|
| Modeled raised class | Tagged committed catalog, bound to context class by ADR-0029 | Catalog digest, class node/fact, rule ID | Candidate source action |
| Class MRO | Pinned Pyrefly Pysa collector, report projection | Provider/run, child class node, ordered `(module, ClassId)` ancestor, fact ID | Class comparison |
| Handler class | Source lexical resolution plus pinned context definition | Handler syntax/reference and class node/fact | Class comparison |
| Positive ancestor candidate | DataFusion derived relation | Source call, model rule, handler clause and MRO fact, same snapshot | Future L2 fate kernel |

Pyrefly's MRO is an assertion under its pinned static context. It is not an observed runtime class mutation. The ancestor pair is only a join coordinate until it resolves to the handler's pinned class fact; display names do not decide the match.

## 3. Contracts and invariants

| Contract | Enforcement | Failure or unknown |
|---|---|---|
| Every retained context class has a resolved MRO row sequence or one empty/cyclic marker | Shared `semantic:context-class-mro-*` rules and Arrow checks | Reject missing, mixed or non-dense rows before publication |
| An ancestor candidate cites exactly the pinned MRO fact and handler class | Declared DataFusion join, codebook status and shared source equality | No match row means `class_relation_unknown`, never a negative |
| A model class can denote a family of possible subclasses | ADR-0030 and class-match codebook | MRO nonmembership cannot refute a catch |
| Candidate relation does not imply handler fate | Separate class/ancestry versus source occurrence and completion rows | L2/summary must withhold unsupported verdicts |

The extractor and compiler output versions move to 30 and 40. The MRO schema and append-only codebook snapshots are reviewed as migrations.

## 4. Derivation and execution

| Stage | Input → output | Mechanism, reuse and exactness | Effects, bounds and evidence |
|---|---|---|---|
| Context capture | Retained class's Pyrefly `PysaClassMro` → ordered raw facts | Existing pinned collector; one fact per ancestor or marker; producer version in run identity | No analyzed code execution; source fact attached to snapshot |
| Class comparison | Modeled class + handler class + MRO → candidate status | DataFusion equality join on child class and ancestor `(module, ClassId)`; Arrow typed output | Only a positive resolved relationship upgrades to `pinned_ancestor`; missing/cyclic stays unknown |
| Publication | Raw and derived rows → validated snapshot | Shared DataFusion semantic rules and source-equality validator | Reject tampered/missing MRO or candidate rows before snapshot append |

There is no graph projection or bespoke transitive-closure implementation: Pyrefly owns the ordered MRO, and DataFusion performs one bounded relational lookup. The amount of new context data and end-to-end cost are not measured before the Stage 3 pilot.

## 5. Journeys

For `open(path)` inside `except Exception`, the authored model nominates potential `OSError`; context capture retains its Pyrefly MRO, the handler type resolves to pinned `Exception`, and the candidate row cites the exact MRO ancestor fact. For `except TypeError`, the different pinned class remains unknown, because a model of `OSError` does not promise an exact runtime subclass. If the MRO is cyclic, its marker is explicit and no ancestor citation exists. A changed Pyrefly revision moves producer identity; a missing MRO row rejects publication. A new model class in an undescribed module still fails ADR-0029's earlier binding boundary.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Pyrefly owns MRO; the catalog owns the modeled action; the handler owns authored type. | — |
| G2 Fidelity | Pass | MRO ancestor, same class, bare, and unknown remain distinct typed states. | — |
| G3 Validity | Pass | Coverage, dense order, marker and child-identity rules plus shared reconstruction reject invalid rows. | — |
| G4 Hidden behaviour | Pass | Pinned context traversal only; no runtime import or source execution. | — |
| G5 Consistency | Pass | Raw facts and candidate rows are validated in one attempt before publication. | — |
| G6 Transformation | Pass | The join retains provider and model identities; absence is not reinterpreted as nonmatch. | — |
| G7 Claims | Pass, scoped | DESIGN and ADR claim a candidate class relation, not catch/completion. | Keep that boundary in L2 and serving. |
| G8 Library leverage | Pass | Existing Pyrefly MRO and DataFusion joins replace a bespoke hierarchy. | — |
| CI-G1 Fidelity | Pass | Static provider MRO, candidate model action and source handler retain separate provenance. | — |
| CI-G2 Evidence closure | Not applicable | No served answer uses this relation yet. | Trace its MRO fact in FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | Gold is not consulted by extraction or derivation. | Preserve independent oracles. |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | Cross-module ancestors absent from retained context definitions cannot become class-bound handler facts. | CI-04, DP-15 · G7 | The MRO pair is retained, but the candidate join requires a pinned handler definition. Such a case stays unknown. | On a real blocked model, expand pinned context capture and test the new module. |
| F02, deferred | A positive MRO relation alone does not select a clause or establish handler completion. | CI-06, DP-08 · CI-G1 | Source may dispatch elsewhere, earlier clauses may act, and handler bodies may branch/raise. Serving a catch now would overclaim. | Resolve L2 frame order/control and carry boundaries into L3. |

F01 and F02 are outside the supported positive class-relation claim. DP-01/02/03/04/05/07/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/10/12 are satisfied in this scope. DP-12 and CI-11/13 remain open for later summary/serving, not assertions of this slice; CI-05/09 are inapplicable because there is no graph or heuristic change. No SHOULD-level exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke scope | Library feature | Fit |
|---|---|---|---|
| Class ancestry | Typed persistence and binding only | Pinned Pyrefly `PysaClassMro::Resolved`/`Cyclic` | Supplies ordered ancestry under one provider revision. |
| Candidate lookup and validation | Domain meaning of positive versus unknown | DataFusion equality joins, aggregation and shared validators | Exact source attribution; no custom hierarchy engine. |
| Storage | One new authoritative Arrow contract | Existing Arrow/Delta attempt machinery | Reuses the snapshot and fact provenance model. |

## 9. Alternatives

| Alternative | Meaning and locality | Decision |
|---|---|---|
| Always unknown for different classes | Sound but cannot use an already resolved provider MRO. | Reject for the functional target. |
| Hand-authored exception hierarchy | Duplicates pinned class meaning and needs parallel maintenance. | Reject. |
| Pyrefly MRO with DataFusion join | Adds raw rows, keeps one provider authority and a cited positive match. | Select. |
| Infer nonmatch from MRO absence | Unsound for a model class denoting possible subclasses. | Reject. |

## 10. Verification plan

| Claim or risk | Label | Focused evidence | Remaining |
|---|---|---|---|
| Pinned `OSError` MRO contains `Exception` and yields a cited candidate | Tested, 2026-09-25 | `modeled_exception_handler_candidates_follow_exact_try_body_ancestry` on `model_handler_shapes` | Other providers and release-scale counts `not_run`. |
| Missing MRO cannot publish as complete context | Tested, 2026-09-25 | The focused test drops the raw MRO view and expects `semantic:context-class-mro-coverage` | Cyclic producer case is source-inspected, not executed. |
| Schema, codebook and publication ledger | Tested, 2026-09-25 | Focused release Nextest selection with reviewed snapshots | Integrated `just test-all` and fresh `just pilot` `not_run`. |

## 11. Authority changes and exceptions

ADR-0030 amends DESIGN §B5, §3.2 and §9.9. It extends the same pinned Pyrefly/Pysa context authority already used for definitions and signatures, and keeps ty as the flow provider. The binding's K1–K3 conflicts do not alter the decision. There is no standard exception.

## 12. Decision

**Accept the positive pinned-MRO candidate rule.** The final focused release Nextest selection passed 5/5 on 2026-09-25 after reviewing and accepting the schema, rule and append-only codebook snapshots. It may upgrade an attributed handler class relationship; F01 and F02 remain explicit Stage 3 boundaries. No refuted catch or completed exception fate follows from this slice.
