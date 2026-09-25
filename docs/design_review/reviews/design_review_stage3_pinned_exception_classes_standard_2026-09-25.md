# Stage 3 pinned exception classes — standard design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | ADR-0029, DESIGN §B5/§3.2/§4.0/§9.9, and the context-to-model-to-candidate-site implementation |
| Standard | Core 2.0, code-intelligence profile 1.0, library-context binding |
| Tier · purpose | Design · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the pinned-class boundary within its stated candidate-action scope |

I traced the committed exception rule through context extraction, model compilation, source application, and shared publication validation. The relevant workload is an analyzed call to `builtins.open` whose modeled `builtins.OSError` is not mentioned by source. I examined the absent-definition failure and a forged candidate-site row. I did not establish exception occurrence, subclass matching, handler completion, or a served claim; those remain Stage 3 work. No graph projection or heuristic changes in this slice.

## 2. Authority and identity map

| Fact or concept | Authority and fidelity | Identity and revision | Consumer |
|---|---|---|---|
| Authored exception action | Committed typed catalog; synthetic model | Catalog digest, model/rule/revision | Model compiler |
| Pinned class definition | Pyrefly dependency context; extracted fact | Context class node and fact IDs, producer/run | `model_exceptions` |
| Source call candidate | Pysa call resolution; candidate, not occurrence | Call site, Pysa fact, target and modality | `model_applications` |
| Candidate exception action | DataFusion derived join | Source call plus model rule and class IDs, snapshot | Future L2 matcher |

The catalog's dotted class name remains a display label; the context definition supplies class identity. A changed catalog digest changes the extractor producer build digest (`crates/cpg-extract/src/config.rs`), including model-only class retention. The context producer does not interpret a class as an actual raise.

## 3. Contracts and invariants

| Contract | Enforcement | Failure |
|---|---|---|
| An applicable authored class binds to exactly one pinned class definition | `resolve_exception_class` in `crates/cpg-schema/src/models.rs` | Missing or multiple matches reject model compilation |
| Conversion names both source and replacement pinned classes | Typed `model_exceptions` schema, `semantic:model-exception-shape`, class identity rule | Invalid or mismatched row rejects publication |
| Candidate site equals the source/model join | `modeled_exception_sites` DataFusion relation and shared source-equality validator | Forged or omitted row rejects publication |
| Open dispatch and modality survive application | Candidate site fields copied from model and application | No implied completed exception or catch |

An unreferenced model stays dormant; an applicable model whose class module is not in the pinned context fails, rather than emitting an unbound exception. This is a bounded supported scope. The schema migration is explicit: extractor output version 29, compiler output version 38, reviewed Arrow snapshots.

## 4. Derivation and execution

| Stage | Input and output | Method, exactness and reuse | Effect and cost boundary |
|---|---|---|---|
| Context capture | Committed exception names plus Pyrefly-described modules → retained class facts | Exact `(module, qualified_name)` class selection; catalog digest in producer identity | One catalog parse and a set lookup per definition; no source execution |
| Model compilation | Applicable target plus pinned definitions → class-bound rule | Unique class lookup; fail closed on absent/ambiguous | Occurs before candidate publication |
| Candidate application | `model_applications` × `model_exceptions` → candidate sites | DataFusion snapshot-local equality join; preserves modality/open remainder | One derived row per source/model action, validated before snapshot append |

No recursive algorithm or graph projection is added. The relation's exactness is about attribution of a *candidate* model action, not runtime occurrence. Cost and pilot cardinality remain unmeasured.

## 5. Journeys

Adding a new exception rule extends one typed catalog entry; its class must be available in the pinned context. For `open`, extraction retains `builtins.OSError` even without a source reference, compilation binds its class fact, and the source candidate cites that fact. Changing catalog bytes moves producer identity. A missing class aborts the attempt before publication; a doctored class or site fails shared validation. A class in a separate undescribed module needs a future explicit context expansion under ADR-0029's revisit trigger. No served claim uses these rows yet.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Catalog action, context class and Pysa candidate have distinct owners and IDs. | — |
| G2 Fidelity | Pass | Class identity, candidate modality and open remainder survive the join. | — |
| G3 Validity | Pass | Unique resolution and shared source reconstruction reject invalid rows. | — |
| G4 Hidden behaviour | Pass | Pinned catalog enters producer identity; no analyzed code executes. | — |
| G5 Consistency | Pass | Normal attempt validation precedes the snapshots append. | — |
| G6 Transformation | Pass | Join keys and candidate meaning are explicit; no completed fate inferred. | — |
| G7 Claims | Pass, scoped | DESIGN states focused testing and keeps handler/exit claims open. | Preserve this boundary in L2. |
| G8 Library leverage | Pass | Pyrefly context capture and DataFusion join/validation do generic work. | — |
| CI-G1 Fidelity | Pass | Synthetic model, context definition and possible source call remain distinct. | — |
| CI-G2 Evidence closure | Not applicable | These rows are not served. | Trace a claim when FORMAT 7 uses them. |
| CI-G3 Evaluation integrity | Pass | Gold is not an extraction input; catalog is an authored model. | Keep independent oracle separate. |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence or gap | Consequence | Correction | Verification |
|---|---|---|---|---|---|---|
| F01, deferred | Model classes in an undescribed context module cannot be retained by the current context walk. | CI-04, DP-15 · G7 | `context_facts` selects from existing `handles`; ADR-0029 states this limit. | Such an applicable model fails compilation, so coverage cannot expand to that rule yet. | Extend pinned Pyrefly module discovery only when a real rule needs it; retain failure meanwhile. | Focused cross-module model fixture. |
| F02, deferred | Exact class identity does not establish subtype catch or completed exception fate. | CI-06, DP-08 · CI-G1 | Candidate relation has no handler-match or completion proof. | Joining it directly to a catch by spelling would overclaim. | Build typed L2 class relation and exit semantics before use. | Positive and withholding handler fixtures. |

F01 and F02 are explicit limitations outside this slice's supported claim, not gate failures for a candidate action. DP-01/02/03/04/05/07/08/09/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/10/12 are satisfied in this scope by the identified mechanisms. DP-12, CI-08 and CI-11/13 await summary or serving consumers; CI-05/09 are inapplicable because this slice adds no graph or heuristic. No material SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Own code | Qualified feature | Fit |
|---|---|---|---|
| Pinned class extraction | Domain-specific retention predicate | Existing Pyrefly context walk | Reuses the pinned provider instead of reparsing a dependency. |
| Candidate action join and equality check | Declared SQL and typed rows | DataFusion join, Arrow schemas | Relational cardinality and validation stay in the adopted engine. |
| Class hierarchy and catch | Not implemented | Pinned context class relations may support it | Resolve exact provider fidelity before designing a bespoke hierarchy walker. |

## 9. Alternatives

| Alternative | Semantic risk and locality | Decision |
|---|---|---|
| Baseline dotted name | String can be mistaken for class identity. | Reject. |
| Simplest: fail every exception model until L2 | Avoids false claim but cannot support an attributed candidate action. | Insufficient. |
| Library-owned: resolve through the existing Pyrefly context, join with DataFusion | One pinned source of class identity; small domain-specific retention rule. | Select. |
| Reparse or import classes on L2 demand | Competing authority and ambient behavior. | Reject. |

## 10. Verification plan

| Claim | Label | Evidence and result | Remaining |
|---|---|---|---|
| Schema, rule and model/source reconstruction | Tested, 2026-09-25 | Focused release Nextest selection: 5 passed, 216 skipped; reviewed and accepted three schema/rule snapshots. | Integrated gate `not_run`. |
| Invalid source-site identity is rejected | Tested, 2026-09-25 | `an_attempt_publishes_every_table_and_readers_see_only_published_rows` focused case. | Add future cross-module class case on trigger. |
| Static correctness | Tested, 2026-09-25 | Release Clippy `-D warnings`, `cargo fmt --all -- --check`, `git diff --check` passed. | Pilot size/performance `not_run`. |
| Handler fate | Proposed | No test or consumer in this slice. | L2 positive and withholding cases before summary use. |

## 11. Authority changes and exceptions

ADR-0029 amends DESIGN §B5, §3.2, §4.0 and §9.9. The accepted candidate boundary does not alter ty as flow provider or promote Pysa call candidates to observed calls. Binding conflicts K1–K3 do not affect this decision; the repository's DESIGN and accepted ADRs govern. No standard exception is used.

## 12. Decision

**Accept this pinned-class candidate boundary.** The rule can now cite one context class identity and a source candidate, with absence and forgery rejected. F01 and F02 remain explicitly outside its claimed coverage and must be resolved before handler or escape verdicts are served.
