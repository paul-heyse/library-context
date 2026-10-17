# Catalog upper execution: analysis preparation and algorithms

**Proposed implementation plan,2026-10-17; no execution evidence.** This is a standalone companion to the [coordinator](catalog-upper-execution-plan_2026-10-17.md), whose [§6](catalog-upper-execution-plan_2026-10-17.md#6-sole-disposition-and-current-checkpoint) alone owns current dispositions. Source: [upper execution target review](../design_review/reviews/design_review_catalog-upper-execution-performance-target_2026-10-17.md), F10/F11 and I07/I08; baseline d37a14add3e18d6a101aa4710dcd279b5445e896. Existing plans are not superseded, reconciled or resumed by this set.

## 1. Target and foundations

Make selected concept work follow relevant attributed incidence and exact namespace scopes, rather than repeatedly searching common rich inventories. Preserve existing finite semantics while independently deciding whether concept enumeration or Summary component execution warrants further change.

The [analytics owner](../design/sections/analytics.md) and [behavioral owner](../design/sections/behavioral-analysis.md) govern interpretation; lctx-model owns the actual kernels and settings. Semantic [§15.10](../design/sections/semantic-model.md#section-15-10) governs declared graph projections. Core3.3/heuristics1.0/code-intelligence1.5 apply through the coordinator. This plan primarily addresses FP-07/A4, DP-10 and H5/H9/H12/H15/H17/H22/H28 without weakening fidelity or completeness.

Source-inspected foundations,2026-10-17:

- analytics/attributes.rs already has charged Correspondence and parameter/class caches. Its facts path still scans type observations per object; native_attributes.rs repeatedly searches metadata, variants, signature ports and supports. Extend the existing preparation owner, not another independent classifier.
- attributes.rs:1156–1259 constructs common facts then repeatedly filters them for each namespace. build.rs:958–974 invokes Concepts and RelationalConcepts separately. Share base **preparation**, not their distinct lattice results.
- concepts.rs:132–247 uses equal-length bitsets and explicit incomplete enumeration. Its algorithm is not diagnosed as wrong merely because concept enumeration can be expensive.
- analysis_graphs.rs coalesces exact projection hydration and retains a graph-bound SCC schedule. summary_production.rs and summary_worklist.rs already use callee-first SCCs, indexed dependencies and a semantic progress frontier. Do not replace them with naive graph iteration or database reachability.

Paths above are under crates/lctx-model/src/domain/ unless cpg-core is named. These findings establish avoidable access/preparation work; they do not establish the cause of run50e4b4 or a numerical benefit.

## 2. Resulting contracts

**Correspondence.** An immutable, charged frame preparation supplies exact candidate ranges by occurrence/owner/context, callable/context, signature and assertion/support identity. It retains alternative observations and their original sources. Candidate lookup does not itself certify a premise: the model-owned consuming predicate checks what that operation requires, at the same semantic scope as before.

**Attributed incidence.** Base facts and selections are indexed by object, then referenced by each exact access-module/namespace scope. One object can legitimately participate in several public exposures. Multiple evidence sources for the same entity/attribute remain independently attributable. Shared base preparation includes complete object/context/policy dependencies; RCA-only relationships extend that base without altering FCA's incidence.

**Algorithm execution.** Concepts retain support, bound, canonical identity, implication meaning and explicit partial status. Summary retains stated model, conditions, source/proof identity, SCC semantics, native attribution and failure distinctions. A faster algorithm or execution placement is eligible only if these contracts remain satisfied; no cutoff, seed, work bound or negative universe changes are implicitly authorized.

**Lifetime.** Preparations borrow or own exact immutable frame inputs on the actual attempt budget. Queues, indexes, retained candidate lists, partition handles and outputs are charged before growth and released after their final consumer. A slice or shared buffer retains its actual backing obligation. Cross-attempt caches are not introduced.

## 3. Packages

<a id="ua01"></a>
### UA01 — Indexed attribute correspondence

**Source:** F10. **Prerequisite:** UE00 shared preparation/preservation contract; no native cutover or whole consumer companion required. **Owner:** lctx-model analytics preparation and its native-attribute consumers.

Extend Correspondence so object/type, owner/occurrence, callable/variant, signature/type-port, class/member, capture and assertion/support questions use exact keyed candidate ranges or existing nominal IDs. Enumerate every whole-inventory search in the reviewed facts/native_attributes path and migrate its consumers together. Resolve class candidates once for their full context, preserving ambiguity and missing outcomes. Remove replaced repeated scans and duplicate preparation after all those consumers move.

Do not eagerly validate unrelated rows. Existing contextual resolution and premise validation must still occur when the original consumer would rely on them; the new index cannot reject a valid selected object because an irrelevant object is malformed. Retain duplicates where they represent distinct supports, exact-versus-qualified distinctions, and class-member authority independent of class flags. An index's candidate ordering must preserve meaningful error and canonical emission behavior.

**Closure:** independent hand-specified attributed incidences for several objects/contexts, competing observations, repeated supports, missing classes and irrelevant-invalid rows; shuffled input yields the same required output. A source/operation-count control demonstrates that lookup visits relevant candidates rather than every observation for every object. Resource refusal before index growth and release after the last borrower remain revealing cases.

<a id="ua02"></a>
### UA02 — Partitioned base incidence for FCA/RCA

**Source:** F11. **Prerequisite:** UE00; UA01's relevant index contract before integrating shared attribute acquisition, not its unrelated class/capture work. **Owner:** analytic facts/concept construction.

Build attributed base facts/selections once for an exact selected object/context/policy domain. Index by object or distribute stable references into exact namespace scopes in one pass. Scope counts, context construction and publication consume these references, eliminating repeated scans of the common inventory. Reuse the base between FCA and RCA where dependencies match; derive call/handoff relational additions separately. Keep method-specific results, scopes, caps and source receipts independent.

Select indexed references versus copied compact incidence according to actual lifetime/backing requirements, not a universal zero-copy policy. The construction must preserve evidence multiplicity even if the mathematical incidence relation deduplicates entity/attribute pairs. Avoid retaining rich native rows solely to share small fact records.

**Closure:** a known finite context with multiple namespaces and overlapping exposures produces the exact attributed incidences, concepts and implication records; both methods retain their differences; absent/empty contexts and cap-triggered partial output remain explicit. Inspect that the base is constructed once only when dependencies really coincide. No whole-frame result cache is needed.

<a id="ua03"></a>
### UA03 — Decide concept enumeration and basis construction

**Source:** I07. **Prerequisite:** UE00 and a known exact incidence/basis contract; evaluate the optimized UA01/UA02 input shape before choosing a larger algorithm replacement. **Owner:** model concept kernel with existing library-capability routes.

Read the current closure/enumeration/basis implementation and inspect suitable established alternatives, including In-Close-style enumeration where relevant. Compare the complete required operation: canonical concepts, support, implication basis, bounded enumeration, deterministic partial-prefix meaning, conversion/preparation and live state. Inspect the pinned rust-reasoning/graph/bitset references when they cover the capability; fetch current primary documentation through Context7 for an actual uncovered library API, and qualify the exact chosen release before adoption.

Decision outcomes:

- **Change:** a supported alternative preserves the required full and partial meanings and removes a demonstrated repeated operation or representation burden. Specify and implement it in this package, migrate its actual callers and delete the replaced kernel/path. A dependency change follows ordinary exact-pin policy and affected controls.
- **Retain:** no alternative has a stronger complete conforming route, or current consumers require semantics the candidate cannot preserve. Record the comparison and concrete reopen trigger in the coordinator; do not invent a replacement merely to close the inquiry.
- **Unresolved:** name the missing semantic or implementation premise and dependent kernel work. Do not mark an unqualified replacement implemented; focused-stage handoff carries the remaining obligation explicitly.

No benchmark campaign is required to choose between these outcomes. Use first-principles examined work and exact semantic cases; any quantitative speed claim needs separate evidence. A changed supported universe, cap meaning or canonical implication contract is a new rule impact outside the eight approved physical directions and must be resolved before dependent implementation.

**Closure:** independently specified isolates, identical/empty attributes, overlapping extents, asymmetric incidence, cap-bound cases and shuffled input; compare concepts by membership and implications by their declared meaning. Retain an independent small oracle/reference where production-generated expectations could reproduce the same mistake.

<a id="ua04"></a>
### UA04 — Decide Summary component retention and overlap

**Source:** I08; contributes to F13 only if a compatible independent grain is established. **Prerequisite:** UE00 plus the current exact graph/SCC/proof owner contract. US02 supplies worker/integration ownership before any concurrent implementation. **Owner:** Summary production/adaptation, not a new graph engine.

Trace actual cross-component borrowers of proofs, evidence, output rows, mutable actual-owner application and successor state. Determine whether a settled SCC can release heavy preparation or publish through the existing private producer without later mutation, and whether independent condensation components can compute privately without changing the canonical fixpoint/reduction contract.

Prefer early release of proven dead state over durable resumability or a cached whole-frame answer. Use the existing callee-first schedule and progress worklist. Where multiple components depend on common premises, preserve shared backing until their final borrower; where actual-owner adoption serializes semantics, retain that boundary and overlap only preparation or proven independent pure computation.

Choose **change** only with an explicit dependency/lifetime trace and compatible joined integration; implement the chosen bounded change here. Otherwise **retain** the current kernel with a reasoned reopen trigger. If a necessary premise remains unknown, record it as unresolved rather than accepting assumed independence. Existing protected native cleanup is neither solved nor bypassed by component release.

**Closure:** acyclic components, independent branches, mutually recursive SCCs, self-loops, shared callees, repeated/dominated proof progress, late refusal and cancellation. Required verdicts, conditions, evidence closure and canonical identity stay the same; actual reservations release only after proved last use. Streaming a prefix cannot masquerade as complete Summary output.

## 4. Integration, validation and completion

UA01/UA02 remove diagnosed work independently of whether UA03/UA04 select additional changes. UA03/UA04 are investigations with real decision outcomes, not unconditional algorithm replacements. [Scheduling US02](catalog-upper-execution-scheduling-plan_2026-10-17.md#us02) owns shared independent-compute integration; this companion owns the analytic/semantic eligibility of each proposed grain. [Consumers UC05](catalog-upper-execution-consumers-plan_2026-10-17.md#uc05) owns remaining Summary/Structural/Analytic **input-window acquisition**, which is distinct from these model-kernel decisions.

During implementation, compile touched crates and run selected pure model/analytics controls without a database where possible. Actual persisted adapters need only focused controls on the installed stable logical fixture after authenticated readiness. Keep normal threading, current assertions and selected timeouts; no full Catalog run, assembled qualify, embedding inference or submitted MCP runtime is part of this plan's endpoint.

UE90 integrates the changed route controls and applicable hygiene once; UE99 updates current owners and hands remaining full validation to later planning. Close F10/F11 only with migrated consumers, removed repeated work and revealing functional evidence. A reasoned retain outcome can complete UA03/UA04's investigation without claiming an unimplemented optimization or closing an unresolved semantic premise. Record every outcome only in coordinator §6, using the source IDs and actual evidence/date. The proposed design's performance benefit remains unmeasured.
