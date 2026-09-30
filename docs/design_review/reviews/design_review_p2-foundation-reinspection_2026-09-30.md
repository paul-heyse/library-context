# P2 foundation reinspection

## 1. Scope, outcome and coverage

**Design · target; independent reviewer; 2026-09-30.** Standard: core/template 3.2,
code-intelligence profile 1.3, and the repository binding selected by
[`standard.toml`](../design_principles/standard.toml). Subject: shared-main dirty tree based on
`9efce30`, specifically the foundation refinements in
[ADR-0092](../../adr/0092-facts-scope-and-callable-identity.md) and
[DESIGN §15](../../design/sections/semantic-model.md).

**Accept scoped, at Implemented and Interface-checked strength.** The selected-use root operation,
dual Signatures grains, provider-absent NotRequested state, native callable attribution,
transitive nominal callable checks, shared-vocabulary ownership and normalized build fingerprint
have coherent owners and inspected implementation routes. This enables continued A10/A11 work;
it does not certify those adapters or facts publication.

The earlier [foundation review](design_review_p2-foundation-refinements_2026-09-30.md) remains a
historical core 3.1/profile 1.2 assessment. Its F01–F03 are reexamined below, not retroactively
certified. The [cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md), §4.1.1 and §8,
owns execution and current disposition. STATUS, the architecture map and §15, ADR-0092, the active
cutover packages and adjacent acquisition/provider/admission/store consumers were read.

Excluded: complete call/type mapping, the unwired Types adapter, flow/document/deployment producers,
assembled coverage and driver behavior, resource/RSS qualification, operator transition, pilots,
P3–P5 normalization/analysis/serving and product evaluation. Their incompleteness remains explicit.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Owned meaning and operation | Consumer/dependency direction | Change boundary |
|---|---|---|---|
| `domain::admission` | Requested roots from ArtifactUse; family grains; exact expected coverage | Provider selection and sealed admission consume the model operation | Selection policy or new family grain |
| `domain::calls` | Native callable sum, nominal symbol kinds and caller/site module correspondence | Adapters construct values; shared validators and generated lowering consume them | New native caller category |
| `domain::assertion` | Support provider/context/source authorization, including callable subjects and destination referents | Assertion derives provide inputs; model and PG execute the same invariant | Authorization semantics |
| `domain::types` | Structural versus nominal terms; transitive native-owner/fidelity checks | Type supports invoke TypeIndex closure; PG reuses it | New term or native mapping |
| Acquisition | Frozen artifact roles and declared corpus ownership | `AcquiredInput::uses` supplies the same rows emitted to the model | Acquisition binding, not completeness policy |
| Provider adapter | Correspondence from one pinned analyzer session to raw assertions | Depends on the model and captured inputs; does not own roots | Analyzer revision/private API |
| Assembly | One vocabulary writer and declared contribution merge | Contributors precede writer through the existing stage scheduler | Additional contributor |
| Shared fingerprint helper | Normalized paths and source/resource bytes | Build scripts supply stage/provider build identities | Production input closure |

The domain operation, rather than a standardized record name alone, governs the requested universe:
`admission::analysis_roots` is called by both `Preflight::expected_coverage` and
`pyrefly_stage::roots`. Acquisition's path/manifest processing creates roles; it does not separately
define expected completeness. Source ownership remains an independently validated relationship.

| Fact/relationship | Provider and fidelity | Coverage and unknowns | Identity/consumer |
|---|---|---|---|
| Selected Python/document roots | Recorded acquisition uses; exact selection | Captured-only dependencies remain lookup context | SourceArtifact IDs; producer/admission |
| Supporting definitions | Pinned Pyrefly; native definition projection | Input Signatures means root-referenced supporting definitions, not every dependency definition | Qualified provider symbols; root resolution |
| Provider callable | Native analyzer ownership, separate from semantic occurrence owner | Module/class bodies and decorator applications remain distinguishable | ProviderCallable sum; raw call sites/supports |
| Named Callable/Overload term | Native nominal reference; anonymous callable stays structural | Foreign provider/context or non-function reference refuses transitively | ProviderSymbol ID inside TypeTerm; type supports |
| NotRequested Flow | No provider assertion or invocation | Catalog profile has explicit NotRequested coverage | Scope/family/context; admission/inspection |

## 3. Contracts, constraints and testing boundaries

`FACTS_REQUIREMENTS` assigns Signatures both Python-artifact and Input grains. Exact matrix
comparison occurs before availability aggregation. One Input row cannot compensate for an omitted
artifact row. ADR-0092 and §15 state the bounded Input projection. The current adapter's `keep`
selection collects native references and traced public names; future A10/A11 reference expansion
must finalize that projection after both call and type collection. Complete-empty projection is
valid; complete dependency analysis is neither requested nor claimed.

`ProviderCoverage::validate` requires an absent provider exactly for NotRequested, with no run or
failure reason. Attempted coverage requires real invocation attribution. The syntax boundary
index and admission key use the optional provider representation; no fabricated Flow provider is
needed to state absence of a request.

ModuleBody identity carries provider, context and module. The other callable arms reach native
symbols. `ProviderCallable::module` checks Function/Method versus Class subtype; `CallerCheck`
requires the caller's acquired module to be the site's module. Shared `SupportCheck` verifies
provider/context and source/acquisition/scope transitively. Legacy SymbolKind codes 6–8 remain
allocated and are rejected by current ProviderSymbol validation.

`TypeIndex::term_support` reaches Callable and Overload nominal symbols through nested terms and
checks Function/Method kind plus provider/context. Anonymous callable structure is preserved.
The existing opaque/truncated descendant rule continues to require DisplayOnly fidelity. These
dynamic rules belong to the shared model; generated foreign keys alone are not claimed sufficient.

Contract fixtures can exercise model invariants without acquisition or a server. Stored controls
use the real generation store with disposable PostgreSQL and inspect sealed contents before
publication. The test observer records successful sink writes instead of declaring a fake
TypePresentationSupport writer; its fixture-size retained copies are not resource qualification.

## 4. Composition and execution

| Operation | Question and semantic I/O | Method/policy and evidence | Limits/determinism |
|---|---|---|---|
| Roots and expected matrix | Which captured artifacts were requested? Uses → exact scopes/families/providers | Pure model operation; no graph projection or inference | Identity sets/order independent; invalid acquisition relationships checked separately |
| Native authorization | May this run state this caller/type? Native IDs and support → acceptance/refusal | Shared scope index, caller module check and TypeIndex walk | Charged validator state; opaque closure remains DisplayOnly |
| Vocabulary merge | Who writes shared identity rows? Contributions → sole-writer output | Existing declared stage handoffs and conflict checks | Contributor-before-writer ordering; finalized vocabulary cannot be read cyclically |
| Source identity | Which production inputs built this provider? Normalized paths/bytes → digest | Existing BLAKE3 plus framed fields; build-time I/O is explicit | Sorted/deduplicated workspace-relative paths; local Python cache excluded |

No projection, analytic or synthesis stage is added. The coverage calculation is exact over the
declared requested scope, not a behavioral analysis. New semantics need model operations and native
correspondence code, not another generic provider or workflow framework.

## 5. Change and failure scenarios

| Scenario and kind | Owning change and propagation | Evidence and remaining boundary |
|---|---|---|
| Select a previously captured-only dependency as an example / binding | Add ArtifactUse; provider roots and expected artifact coverage follow the same operation | Independent selected/captured-only, role-order and role-addition control in `domain_admission` |
| Add a dependency target or named type / composition | Adapter gathers nominal references; bounded supporting-definition projection includes them | Accepted contract and current `keep` route inspected; full A10/A11 reference expansion remains open |
| Analyze the same module under another provider/context / binding | Native callable identity and shared support authorization preserve the distinction | Four callable forms have provider/context mismatch controls in model and PG fixtures |
| Add a named callable inside another type / domain extension | New native mapping uses the same TypeIndex closure and generated lowering | Direct/nested provider/context/kind controls; no spelling-only reference |
| Add ty/document vocabulary contributions / composition | Contributor names its subset; assembly remains sole writer | Existing scheduler/contribution boundary inspected; no terminal-vocabulary reader added |
| Substitute a provider revision or change embedded runtime resources / mechanism | Adapter mapping changes locally; source/lock/resource fingerprint changes build identity | Mutation, new-module, relocation and cache controls inspected; executed receipts stated separately |

## 6. Correctness and fidelity gates

Verdicts are bounded to the inspected foundation, not assembled phase qualification.

| Gate | Verdict | Independent evidence/scope |
|---|---|---|
| G1 Authority | pass scoped | Root/grain policy in model; assembly owns vocabulary; typed identity owns lowering |
| G2 Semantic fidelity | pass scoped | Native caller forms, supporting scope, nominal/anonymous types and NotRequested distinctions preserved |
| G3 Validity | pass scoped | Caller/support source and provider checks plus transitive named-type checks; stored validation shares them |
| G4 Hidden behavior | pass scoped | Root derivation is pure; fingerprint I/O is explicitly build-time; no new ambient analyzer input |
| G5 Consistency/recovery | pass scoped | Existing generation validation and sole-writer runtime retained; whole-pipeline exhaustion/recovery excluded |
| G6 Transformation/reuse | pass scoped | Nominal identity and normalized source/resource closure preserved; no skip-on-cache-key introduced |
| G7 Truthful claims | pass scoped | Implemented inspection distinguished from test receipts; incomplete adapters and qualification excluded |
| G8 Library leverage | pass scoped | Existing derives/Arrow/store/runtime, standard collection/path operations and BLAKE3 reused |
| CI-G1 Fidelity | pass scoped | Selected roots do not imply dependency analysis; unknowns and NotRequested retain distinct meanings |
| CI-G2 Evidence closure | n.a. | No served claim produced here; raw support authorization is covered by G2/G3 |
| CI-G3 Evaluation integrity | pass scoped | No new path from gold, evaluation references or library skills into analyzer inputs |

## 7. Findings and applicability

No outstanding blocker remains in the inspected foundation. Reinspection of historical findings:

| Source finding | Reinspection evidence | Limits/current disposition |
|---|---|---|
| Foundation F01 | Qualified ModuleBody sum; callable Subject; transitive support authorization and same-source caller check | Model and PG controls inspected; complete provider mapping still A10 |
| Foundation F02 | Callable/Overload function IDs; TypeIndex owner/kind checks in recursive closure; anonymous structure retained | Model and PG controls inspected; A11 mapping is unwired |
| Foundation F03 | ArtifactUse operation shared by roots/admission; dual Signatures grains; ADR-0092 bounds Input completeness; exact matrix checks | Contract and representative implementation accepted; A10/A11 expansions and B1 assembly remain scheduled |

During inspection, source closure initially omitted embedded non-src resources. Expanding it also
briefly admitted Python bytecode caches. Both were corrected before this decision: the helper now
includes crate SQL/migrations/models and root scripts/specs, excludes Python caches, and the
mutation/relocation fixture exercises those distinctions. This is corrected inspection evidence,
not a claim of measured production reuse.

FP-01–FP-06 and applicable DP-01–DP-09, DP-13–DP-18, DP-21–DP-24 are satisfied for the named
scenarios. DP-19/20 and CI-08 have no new production lifecycle or resource mechanism here; their
existing whole-pipeline obligations remain outside this bounded acceptance. CI-01–04/06/10 are
satisfied for the foundation meanings; CI-05/07/09/11/13 are not new behavior in this slice.

## 8. Library fit and total complexity

Existing nominal IDs and derives provide structural declarations/codecs/PG foreign keys; dynamic
ownership remains an ordinary model invariant. The existing stage runtime handles contributed
vocabulary. Standard collections and BLAKE3 are sufficient for the deterministic source manifest;
no new manifest framework or hashing library is required. No new library API or version change is
asserted by this review. Native adapter API completeness needs its own A10/A11 controls.

## 9. Alternatives and tradeoffs

| Alternative | Judgment for the selected scenarios |
|---|---|
| Independent producer roots and path-only admission | Inferior: captured and requested universes can diverge under a role change |
| ArtifactUse roots with bounded Input support projection | Preferred: single scope authority without whole-dependency analysis |
| Full dependency analysis | Changes requested product scope/cost; unnecessary for current supporting references |
| Pseudo-symbol implicit callers or bare callable names | Inferior: loses native category or qualification and exposes filtering rules to consumers |
| Typed callable sum and shared nominal closure checks | Preferred: explicit distinctions and bounded validation/lowering propagation |
| Hand-maintained selected-source fingerprint list | Inferior: new production files/resources silently miss identity changes |
| Conservative source/resource directory manifest | Preferred for this scope; extra source/script changes may invalidate identity, but cannot silently reuse it |

## 10. Verification and uncertainty

All source/test-fixture inspection above is **Implemented/Interface-checked, 2026-09-30**.
This reviewer ran no compile/tests, integrated gates, formatting or pilots: **not_run**. The author
supplied the following focused **Tested** receipts dated 2026-09-30, after fixture/order repairs;
these are attributed results, not independent reviewer execution:

| Command | Author-reported outcome and scope |
|---|---|
| `python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test domain_admission --test domain_calls --test domain_types -p cpg-extract --test producer_fingerprint` | **passed**: admission 9, calls 13, types 12, fingerprint 1; selected roots, missing/duplicate Input and artifact Signatures coverage, native authorization and source/resource/cache identity controls |
| `python3 scripts/build_environment.py -- cargo test --release -p lctx-postgres --features testing --test domain_types --test domain_calls` | **passed** against disposable PG18: two call tests and one types test, including four caller forms with provider/context mismatch and direct/nested named callable provider/context/kind matrix |

Tests inspected include model `domain_admission`,
`domain_calls`, `domain_types`, shared `fixtures/callers.rs` and `fixtures/types.rs`, real-PG
`domain_calls`/`domain_types`, and `cpg-extract/tests/producer_fingerprint.rs`.

The negative fixtures provide existing foreign rows before requesting authorization, so they test
semantic ownership rather than dangling foreign keys. The role-control expectation names selected
versus captured-only artifacts independently. The fingerprint control supplies fixed source
fixtures, mutates each input, relocates bytes and introduces ignored cache versus new production
module. These are meaningful controls; this review does not certify their execution result.

Remaining uncertainty is adapter completeness, transitive supporting-definition expansion,
full scheduled coverage/admission, production sink/resource closure and Q evidence. Independent
raw known answers and profile pilots are still necessary at their planned boundaries.

## 11. Authority changes and dispositions

ADR-0092 and §15 hold the accepted refinements. Their Proposed evidence labels are conservative
until the author reconciles implementation/test receipts; this review does not edit accepted ADR
content, plan disposition or STATUS. Current execution status and closure of the historical F01–F03
belong in cutover plan §8, with this reinspection as evidence. No new ADR or compatibility layer is
requested. A new root policy, native caller category or source-resource layout reopens its owner.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scoped scenario evidence |
|---|---|---|
| A1 Localize change | satisfied | Scope/native/type/fingerprint changes have coherent owners; lowerings and adapter bindings follow explicit contracts |
| A2 Encode domain meaning explicitly | satisfied | Requested versus captured, native versus semantic caller, nominal versus anonymous callable, and unrequested versus attempted are explicit and govern selection/validation |
| A3 Extend through composition | satisfied | New facts/vocabulary contributors reuse model operations, shared validation and stage handoffs without another framework |

**Bounded decision: Accept scoped.** Continue A10/A11 against these contracts and add their
complete native-reference projection and raw known-answer controls. **Enclosing P0–P2 architecture:
not yet accepted or qualified.** The remaining producers, B1–B3, CLI/deletion scope, resource
envelope, pilots and assembled Q review retain their independent obligations.
