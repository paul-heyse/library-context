# Operation-shaped native execution plan

**Independent design-tier / target review · 2026-10-09**

**Decision: Accept scoped at Proposed strength.** The revised plan provides a credible correction for vocabulary-wide physical enforcement and repeated final canonical preparation. It preserves typed semantic authority, independent cold admission and actual-state reconciliation. One finalization-order defect was corrected in the proposed target during review; implementation and verification remain open.

This accepts the proposed finalization and physical-lowering architecture for the stated sequential pre-final compilation scope. It does not accept independent overlapping pre-final completion/read composition, close the source findings, qualify the existing implementation, or establish a runtime improvement.

## 1. Scope, outcome and coverage

| Field | Reviewed boundary |
|---|---|
| Subject | [Populated execution companion](../../plans/populated-journey-execution-amplification-plan_2026-10-09.md), revised [persisted coordinator](../../plans/persisted-graph-execution-plan_2026-10-07.md), and their affected correction/compilation/coroutine integration |
| Baseline | HEAD `3c76f1721b425118ae3d58aa6c612ab042446c4e`; root-owned documentation changes, including the final §3.3 handoff correction |
| Standard | Core/template 3.3; efficient-architecture heuristics 1.0; code-intelligence profile 1.5; library-context binding |
| Tier · purpose | Design · target |
| Reviewer · date | Independent delegated design reviewer · 2026-10-09 |
| Maturity | Proposed implementation target; current interfaces and decisive existing source inspected |
| Functional outcome | Complete pinned compilation, attributed catalog/facts, native admission/publication, portable export, connected evidence tools and SQL backup/restore |
| Workload | Growth in records, declared kinds, contributors and aliases; scope skew; concurrent final read cursors; cancellation and physical corruption |
| Exclusion | Independent overlapping pre-final read/completion remains source-review F03’s deferred scenario |
| Method | Read-only source, model, owner, library-source and document inspection; no builds, tests, probes, database operations or process intervention |

The [source populated review](design_review_populated-journey-execution-amplification_2026-10-08.md) supplies diagnoses and investigative leads. It is not the acceptance standard. The judgment below follows the loaded standard and functional target.

Decisive inspection covered native schema/codec/lifecycle/cold validation, artifact admission and detached readmission, workspace completion and semantic admission, external ordering and acknowledgement ownership, ordinary/detached publication, actual-row reconciliation, and staging-to-fresh restore.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Responsibility and preserved boundary |
|---|---|
| `lctx-model` | Typed record meaning, nominal identity, sum validity, contribution/view/binding contracts, outcome and failure distinctions |
| `lctx-surrealdb` | Mechanical bodies/scopes, fixed envelopes, physical adapter selection, native mutation/read admission and compact scratch preparation |
| `cpg-core::Workspace` | Exact completed descriptors, compilation attestation, semantic/frontier admission and the workspace/native visibility handoff |
| Artifact owner | Admitted canonical families, manifest, originals and explicit portable transport |
| Publisher | Derived edges/search, independent actual-state agreement, executable realization and committed unselected publication |
| Backup/restore | SQL staging isolation, independent imported admission and fresh physical reconstruction |

Dependency direction remains mechanism → model. The selected adapter belongs in the native mechanism, rather than introducing SurrealDB types into model declarations.

[Completed contracts:10](../../../crates/lctx-model/src/domain/completed.rs) distinguish specifications, completed contributions, exact views, current/boundary bindings and transported state identity. Those distinctions govern the inspected workspace/native operations; they are more than output metadata.

| Fact/fidelity family | Authority and interpretation | Preserved consumer obligation |
|---|---|---|
| Provider observations | Captured producer/profile/model/settings and exact dependency views | Preserve attribution, disagreements and coverage |
| Canonical entities/assertions | Typed graph records and nominal keys | Preserve kinds, references, alternatives, isolates and parallel relationships |
| Compiler backing/originals | Model callbacks; separate original byte owner | Preserve complete intermediate values and exact original bytes |
| Analysis/behavioral products | Declared projection or finite transfer model | Preserve precision, settings, complete required universes and explicit outcomes |
| Serving projections | One admitted content/executable realization | Preserve evidence closure and journey-bound references/cursors |

No new semantic classifier, writable canonical authority or manually maintained record whitelist is proposed.

## 3. Contracts, constraints and testing boundaries

The enforcement change is deliberate. Fixed `SCHEMAFULL` envelopes retain top-level types, uniqueness and enforced relationship endpoints. Complete record-shape validity moves to supported typed ingress and independent physical reconstruction.

That boundary is adequate for the supported writers because the plan requires rejection before their physical writes and full comparison at cold/publication boundaries. Arbitrary private raw SQL loses statement-time closed-body rejection. The plan exposes this limitation rather than claiming equivalent database enforcement.

Existing [body codecs:70](../../../crates/lctx-surrealdb/src/codec.rs) validate records and preserve textual/opaque binary distinctions. Existing [reconciliation:30](../../../crates/lctx-surrealdb/src/reconciliation.rs) reconstructs expected complete rows and edges from canonical values and compares actual stored state. These provide credible foundations for PJ2; sharing production callbacks does not replace independent corruption controls.

| Contract | Required invariant and failure boundary |
|---|---|
| Selected physical lowering | Unknown relation fails closed; local sum rules remain complete; grouped ordering and canonical identity remain exact |
| Supplied scopes | Derive from selected declared fields using existing conversion semantics; independently reject stored disagreement |
| Final content freeze | Close content mutations atomically, drain admitted mutations, reject pending/failed/uncertain state and establish exact descriptor agreement |
| Semantic admission | Consume the drained frozen state and exact compilation or independent restored-manifest premises |
| Prepared pointer runs | Complete completed membership plus required aliases; deterministic deduplication; no ready result before successful source/worker terminality |
| Independent cursors | Separate offsets and charges; retain scratch through outstanding blocking work and the last consumer |
| Publication | Derive permitted projections, drain writers, certify actual content/definitions and publish the same realization |
| External transport | Imported validity/preparation markers are never trusted; independently admit reconstructed content/state |

The finalization distinction is consequential: [current `end_writes`:944](../../../crates/lctx-surrealdb/src/compiler.rs) closes all native admission. The plan correctly introduces an earlier mutation-only freeze while keeping reads available, and retains later global drain/seal/removal.

## 4. Composition and execution

The selected composition removes established repeated work without deleting necessary assurance.

For F01, fixed native bodies and supplied scopes remove the vocabulary-wide body unions and stored scope `VALUE` programs visible in [schema generation:179](../../../crates/lctx-surrealdb/src/schema.rs). A prepared declaration-derived adapter selects the actual relation once per group. Keeping synchronous typed leaves avoids moving the same vocabulary expansion into generic async machinery.

For F02, [canonical scan setup:549](../../../crates/lctx-surrealdb/src/compiler.rs) currently discovers completed owners, memberships and aliases and sorts their pointers for each scan. [Header admission:45](../../../crates/cpg-core/src/native_canonical.rs), payload admission, export and sealing have a shared final immutable universe. Preparing compact pointers once is therefore a useful lifetime boundary.

The plan retains actual row reads and their checks. Pointer reuse establishes selection, not body validity, graph agreement or semantic truth.

| Stage | Universe/method and assurance |
|---|---|
| Final canonical preparation | Exact completed physical pointers and required aliases; complete deterministic sort/deduplication; bounded spill |
| Header/payload consumers | Same prepared universe, independent cursor and actual native hydration |
| Semantic/frontier checks | Exact frozen completed descriptors; model-owned invariant/reference contracts |
| Publication | Canonical family/state identity, originals, actual graph/search and executable-definition agreement |
| Serving | Existing declared projections, qualification and pinned evidence relationships |
| Behavioral computation | Existing finite transfer/fixpoint semantics; no reachability substitution |

The scratch refactor is feasible but substantive. [Current ordered state:263](../../../crates/lctx-surrealdb/src/ordered_rows.rs) combines final scratch and mutable reader ownership; [acknowledged ordering:31](../../../crates/lctx-surrealdb/src/acknowledged_candidates.rs) retains blocking commands and their terminal observer. PJ3 must separate immutable scratch from cursor state while preserving that ownership. Merely adding shared rewind access would violate the proposed contract.

Removing the post-readmission reload in [detached publication:182](../../../crates/lctx-publisher/src/lib.rs) is also coherent. Initial external admission remains independent; publication can consume the authority already established by [detached readmission:751](../../../crates/cpg-core/src/artifact.rs).

A4 is supported qualitatively for this correction. Full necessary hydration/reconciliation and retained SQL restore still have work proportional to their actual guarantees. Their existence is not itself amplification. Phase timing and residual-access investigations appropriately remain distinct from structural diagnosis.

## 5. Change and failure scenarios

| Scenario | Assessment |
|---|---|
| Add an unrelated record family/field | One model addition and derived adapter construction; existing-row DDL/body/scope programs need not expand |
| Add a local sum arm | The selected record retains complete arm validation; selecting only the outer relation is insufficient |
| Substitute native physical lowering | Codec, scopes, realization identity, reconstruction and all writers migrate together; no compatibility authority |
| Grow overlapping contributors/aliases | Prepare the exact deduplicated union once; retain alias-only companions and independent membership checks |
| Complete-empty final state | Empty prepared runs still carry authority and successful terminality; empty cannot mean incomplete |
| Interleave/cancel final cursors | Separate offsets; one cancelled cursor cannot delete shared scratch or release another cursor’s charges |
| Freeze during admitted mutation | Drain the result, establish workspace/native agreement, then admit that state or fail |
| Corrupt flexible body/scope/original | Full cold/publication reconstruction rejects; header/digest agreement alone cannot pass |
| Restore under another database name/build | Preserve captured supplier/outcome contracts and semantic state; rebuild current physical/executable realization |
| High-degree/cyclic analysis | Preserve complete intermediates, direction, multiplicity and finite semantics; output limits do not narrow the computation universe |
| Introduce independent pre-final overlap | Reopens source F03; this review does not accept that composition |

The irreversible final freeze is a credible simpler alternative to a mutable-generation cache because no selected consumer requires further compilation into an already admitted attempt. Refusal of such mutation is an explicit contract change, with a new attempt as its continuation route.

## 6. Correctness and fidelity gates

These are judgments about the specified target, not executed product checks.

| Gate | Verdict | Evidence and scope |
|---|---|---|
| G1 Authority | Pass | Model declarations remain authoritative; adapters, spools and search are derived |
| G2 Semantic fidelity | Pass | Nominal keys, complete bodies, sum distinctions, NULL/missing behavior and explicit outcomes retained |
| G3 Validity | Pass, Proposed contract | Typed pre-write rejection and full independent cold/physical checks are explicit |
| G4 Hidden behavior | Pass | Freeze, persistence, preparation, external import and derived publication effects are declared |
| G5 Consistency/recovery | Pass, Proposed contract | Frozen-state handoff, acknowledgement/drain, scratch lifetime and committed-effect preservation specified |
| G6 Transformation/reuse | Pass, Proposed contract | Exact frozen dependencies and complete alias-aware universe; actual-state assurance remains |
| G7 Truthful claims | Pass | Proposed remedies separated from attributed receipts and unmeasured benefits |
| G8 Library leverage | Pass | Reuses native envelopes/indexes, existing codecs/spill kernel, Tokio and tracing; no new generic execution/cache framework |
| CI-G1 Fidelity | Pass | Provider attribution, uncertainty, model-relative behavior and heuristic distinctions preserved |
| CI-G2 Evidence closure | Pass, bounded | Complete canonical admission and realization-bound evidence/cursors retained |
| CI-G3 Evaluation integrity | Pass, bounded | No production oracle input, protected tuning or evaluator-meaning change introduced |

## 7. Findings and applicability

<a id="f01"></a>

### F01 — Final admission must certify the state produced by the freeze handoff

**Finding strength: Proposed target defect identified and corrected before publication, 2026-10-09.**

The initial target placed semantic checks before mutation admission closed and admitted mutations drained. That ordering did not establish that the checks and prepared inventory described the same final state.

The adjacent ownership boundary matters as well. [Producer publication:2869](../../../crates/cpg-core/src/workspace.rs) completes native output, persists bindings and then exposes workspace descriptors. Native counters alone cannot establish the whole handoff. [Compilation attestation:418](../../../crates/cpg-core/src/workspace.rs) also includes current workspace content identity.

This is not a reproduced current production-output failure. Existing [workspace immutability:379](../../../crates/cpg-core/src/workspace.rs) and its completion gate already exclude ordinary producer publication after compilation finishes.

**Principles:** FP-02, FP-04, FP-05; DP-03, DP-08, DP-09, DP-19; A2; G3/G5/G6.

**Correction examined:** revised companion §3.3 now:

- fences the complete workspace/native publication handoff with the existing async completion gate;
- drains admitted content mutation and checks actual pending/failure state;
- compares actual frozen bindings/owners with workspace descriptors;
- checks the full normal compilation attestation after the handoff;
- preserves the separate restored-manifest admission route;
- performs final semantic/frontier admission over the frozen state;
- releases the visibility gate before semantic scans and preparation.

**Disposition:** resolved in the Proposed target by static inspection. PJ3’s revealing handoff/attestation controls remain implementation obligations; this resolution is not Tested closure.

No remaining blocking target-design finding was identified. Source-review F01/F02 remain Open at [coordinator §8](../../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition). Source F03 retains its own deferral and trigger.

FP-01–FP-06 are satisfied within this proposed ownership/composition scope. FP-07 is satisfied for the selected correction’s operation and growth scenarios. Relevant DP-01–DP-05, DP-07–DP-10 and DP-15–DP-24 have explicit preservation or correction mechanisms. Relevant CI attribution, fidelity, universe, completeness, evidence and pinning obligations remain satisfied at Proposed strength. This is not certification of every enclosing algorithm or consumer.

## 8. Library fit and total complexity

**Interface-checked, 2026-10-09.** Lock/source inspection confirms SurrealDB 3.3.0 and Tokio 1.53.2 at the decisive boundaries. The pinned SurrealDB capability skill and engine source were consulted.

| Capability | Fit and burden |
|---|---|
| `SCHEMAFULL` object `FLEXIBLE` | Fits complete mechanically lowered bodies; leaves semantic closedness to selected typed ingress/reconstruction |
| Supplied typed scope arrays | Retains existing scope access/index meaning while removing unconditional derived-value evaluation |
| NULL/NONE processing | Pinned field cleanup preserves NULL but removes object-valued NONE, including flexible descendants; plan preserves explicit nullable/inactive NULL fields |
| External ordering | Existing bounded kernel already supplies framing, deduplication, spill and conflicts; separating final run/cursors is a bounded lifecycle refactor |
| Tokio ownership | Existing borrowed finite leaves and acknowledged blocking operations fit; detached replacement is unnecessary |
| Tracing/run outputs | Coarse phase attribution has a concrete diagnostic consumer; no general telemetry framework or per-row event stream is needed |
| DataFusion | Existing compute/transfer and runtime sharing remain; eager MemTable caching does not substitute for exact native pointer preparation |

The pinned [field-processing source](https://docs.rs/crate/surrealdb-core/3.3.0/source/src/doc/field.rs) supports the selected FLEXIBLE/NULL/NONE and VALUE distinctions. It does not establish whole-journey speed or production acceptance of the new schema.

## 9. Alternatives and tradeoffs

| Alternative | Judgment |
|---|---|
| Current broad unions/VALUE | Real database rejection, but unrelated vocabulary expands ordinary preparation/dispatch |
| Fixed envelopes + typed ingress/reconstruction | Selected; removes that coupling while openly changing unsupported raw-SQL enforcement |
| Discriminator-directed ASSERT | Viable if a smaller qualified program is demonstrated; a giant sequential replacement would relocate the defect |
| Per-kind tables | Native closedness, but proportional definitions/index/routing burden without a named locality consumer |
| Frozen compact pointer runs | Selected; useful immutable lifetime, bounded spill and no mutable invalidation system |
| Mutable-generation preparation cache | Additional invalidation/alias/lifetime ownership without a current selected consumer |
| Native set-based membership | Conditional candidate; exact plans, membership and deduplication must settle its fit |
| Different intermediate persistence | Eligible larger alternative; retained native access/inspection consumers currently justify the selected placement |
| Data-only restore | Eligible larger alternative; complete independent transport would be required |
| More concurrency/larger deadlines | Does not remove the demonstrated repeated work |

Retained native placement and SQL staging are acceptable here because they retain actual completed-view inspection and imported-executable isolation. This is a technical tradeoff judgment, not acceptance merely because RC03/RC04 were retained.

## 10. Verification and uncertainty

| Claim | Evidence/outcome |
|---|---|
| Proposed route has credible existing owners/interfaces | **Interface-checked**, source inspection 2026-10-09 |
| Finalization-order correction is explicit and coherent | **Proposed**, revised §3.3 and acceptance row inspected |
| New fixed schema rejects malformed data at promised boundaries | **not_run**; PJ2 controls required |
| One preparation and independent charged cursors behave correctly | **not_run**; PJ3 controls required |
| Representative runtime/phase observations | **not_run**; PJ1 controls required |
| Quantitative runtime/peak-memory benefit | **not_run**; no measured claim |
| Fresh builds/tests/probes | **not_run**, read-only review assignment |
| Historical populated pass | Attributed coordinator receipt only; not rerun or promoted to new-schema qualification |

Static evidence suffices to select this proposed architecture. No probe is required to settle the architectural choice now. Implementation must still challenge the altered enforcement and lifetime boundaries, particularly local sum validity, supplied scopes, full body corruption, empty/alias inventories, cancelled cursors, final handoff and restored-state provenance.

The plan/coordinator preserve all source §§3/5–11 routes: selected F01/F02 remedies, F03 exclusion, phase/runtime investigation, candidate-owner access, browse preparation, aggregate live state, graph adapters, residual demand, intermediate placement and data-only transport.

PC6/CU6/BC3/BC5 remain independently open where required evidence is absent. Active verification remains release until BC3 qualification. Matching receipts can be shared only for their actual source/profile/cases. Deferred timeout follow-up and assembled qualification are not silently reactivated.

## 11. Authority changes and dispositions

| ID | Rule impact and route |
|---|---|
| RC01 | Replace broad closed native body enforcement with fixed envelopes, complete typed ingress and independent full physical admission. Existing source RC01 is operator-accepted; PJ0 must record the superseding decision and update §15.11/§6 owners before production migration |
| RC02 | Replace stored scope VALUE replay with mechanically supplied typed scopes and independent agreement checks. Existing source RC02 is operator-accepted; PJ0/PJ2 own the decision/schema/consumer cutover |
| RC03 | Native placement remains retained. A later selected intermediate-placement alternative would require its own decision |
| RC04 | SQL staging-to-fresh transport remains retained. Data-only replacement remains triggered investigation |

If RC01/RC02 were retained unchanged, pointer reuse could still correct source F02, but source F01’s residual vocabulary-wide native programs would remain. The selected target therefore depends on those confirmed rule changes.

This review introduces no additional operator rule choice. F01 above corrects the proposed finalization contract; enduring ownership belongs in the architectural owners through PJ0. Scheduled source disposition remains solely at the persisted coordinator.

## 12. Architectural judgments and decision

| Judgment | Verdict | Scoped reason |
|---|---|---|
| A1 Localize change | Satisfied | Model, physical adapter, finalization, scratch and publication owners constrain propagation; unrelated extensions avoid existing-row programs |
| A2 Encode domain meaning explicitly | Satisfied, Proposed | Typed records and exact completed contracts govern behavior; final freeze, descriptor handoff, provenance and trust boundaries are explicit |
| A3 Extend through composition | Satisfied, scoped | Admission/export/sealing/restoration reuse exact frozen authority and owned cursors; independent pre-final overlap remains excluded |
| A4 Fit execution to workload | Satisfied, Proposed correction | Removes established vocabulary-wide enforcement and repeated final discovery/sorting; retains necessary complete reads, assurance and bounded lifetimes |

**Bounded decision: Accept scoped at Proposed strength.** The final revised target has no remaining blocking design finding within sequential pre-final compilation and concurrent final-read scope.

**Enclosing architecture:** the implemented source still needs the scheduled F01/F02 correction and its acceptance. This review does not convert the existing populated receipt into whole-plan, release, performance, real-library or product qualification.

**Next responsibility:** the coordinator can complete document publication and execution planning. PJ0 establishes enduring contracts before dependent implementation; PJ2/PJ3 implement the physical and lifetime corrections; PJ1 supplies representative diagnostics; PJ5 performs the combined acceptance when its scope is authorized. Source F03 and retained larger alternatives keep their stated revisit triggers.
