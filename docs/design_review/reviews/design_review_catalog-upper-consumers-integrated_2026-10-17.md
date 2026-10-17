# Catalog upper consumers: integrated target assessment

_Independent design-tier / target review, 2026-10-17. Standard: core 3.3, efficient-architecture heuristics 1.0, code-intelligence profile 1.5 and the library-context binding._

**Accept scoped, at source / Interface-checked strength.** The assembled UC01–UC05 implementation has a coherent route for removing repeated metadata reconstruction, discarded rich acquisition and repeated partition filtering while retaining domain and admission authority. No new material finding was established in the inspected scope. This is independent architectural and fidelity judgment, not an assertion that the functional controls passed, F05–F09 are closed, or the full Catalog product is qualified.

The [upper coordinator §6](../../plans/catalog-upper-execution-plan_2026-10-17.md#6-sole-disposition-and-current-checkpoint) remains the sole current disposition owner. The [consumer companion](../../plans/catalog-upper-execution-consumers-plan_2026-10-17.md) supplies the UC01–UC05 obligations. This review does not update either ledger.

## Scope, baseline and evidence

The baseline is `main` at `6d958e8a`, plus the uncommitted consumer implementation inspected on 2026-10-17, including the final retrieval captured-input refinement and native adapter control source. The examined changes span `cpg-core` uniform preparation, C0 metadata, native inventory adaptation, retrieval and its caller, Completion and Structural/Analytic/Summary windows; `lctx-model` expected-domain, uniform and native declarations; compiler-store scoped positions; and the adjacent independent uniform receivers in compiler-store and native. ADR-0186 and the amended semantic/storage sections describe the accepted target; agreement with the ADR is context rather than the criterion for the judgment.

The functional intent is a version-pinned API and evidence catalog with explicit uncertainty and same-realization evidence. The relevant workload has several analysis owners over unchanged exact inputs, overlapping selected roots, sparse and skewed partitions, private callable bodies, and multiple provider runs sharing a frame. The assessment concerns acquisition and preparation for those workloads. It does not certify full Catalog campaigns, embedding inference, submitted MCP runtime, US02 concurrency, UA algorithm changes, throughput, latency or capacity.

Method: source and type inspection, comparison of the uncommitted diff with the baseline, and inspection of control source. No build, test, service, production mutation, formatting or Git operation ran in this review. The implementers were completing controls concurrently; late verification evidence and any subsequent source changes require integration by the coordinator. The initial memory keyword search returned no relevant entry; no memory-derived fact is used here.

## Responsibilities and governing meaning

The implementation keeps three operations distinct: selecting protected exact inputs, preparing sufficient immutable properties, and constructing an owner's fresh semantic result. This distinction is useful rather than ceremonial: the uniform cache contains input preparation, native property consumption requires a separate checked semantic authority, and window positions cannot substitute for domain or publication admission.

| Owner | Responsibility and contract | Change boundary |
|---|---|---|
| `lctx-model::domain::analysis::expected` and `uniform_outcome::analysis_owner` | Expected roots, artifact classification, authored definitions, request state, fresh NotRequested domains and ordered source identities | A new property or owner demand changes its model operation and corresponding adapter, without another consumer classifier |
| `cpg-core::compilation::uniform` and `workspace::CompletedInputs` | Operation-local immutable fragment preparation; exact selected-source compatibility and retained source ownership | Acquisition mechanism and fragment lifetime; no result, admission or ordered-source reuse |
| `catalog::build::metadata_validation_inputs` and C0 adapter | Complete expected-domain properties and useful authored/provider metadata | Physical demand changes separately from rich selected kernels and receiving validation |
| `analysis::native::NativeInventory` plus normalized semantic admission | Finite assertion/support correspondence, qualification alternatives, fidelity and complete prerequisite selection | Native family/predicate changes remain model-owned; compact consumption cannot establish receiving validity |
| Compiler-store `scoped_positions` | Exact schema/port/backing mapping, physical positions and charged gathers | Representation may change without redefining root meaning |
| Retrieval and body/frame adapters | Model-selected input domains, ordered visitors, range demand and per-root/frame semantic integration | Consumer policy remains with its actual owner; shared hydration does not widen domains |

The fact/fidelity distinctions remain explicit:

| Family or derived form | Authority, identity and fidelity | Coverage / consumers |
|---|---|---|
| Artifact and frame metadata | Exact completed source plus model-owned identity/input/classifier properties; metadata is not source text | Common uniform preparation and complete expected domains |
| Native assertion/support/qualification properties | Actual normalized admission over the complete selected native dependency closure; assertion/support IDs and original fidelity remain distinct | Inventory output preserves qualifications and alternatives; Catalog still excludes Flow families |
| Uniform NotRequested descriptors | Fresh owner definition, context, declaration and ordered exact source identities | NotRequested and genuine empty domains remain distinct from requested work or absent knowledge |
| Retrieval and frame partitions | Exact selected port and physical row positions under the existing closure/consumer operations | Membership is selection, not a claim of complete program semantics; no new inferred or heuristic fact is introduced |

## Composition, execution fit and lifecycle

**Uniform preparation (UC01).** `Preparation::acquire` retains `CompletedInputs` beside its immutable model values and publishes an entry only after the entire load and cancellation check succeed (`compilation/uniform.rs:46–74`). Whole-fragment loaders enforce exact row count and EOF before reuse. `same_preparation_sources` checks model, profile, attempt identity, resource pool, requested/resolved epochs and actual retained source correspondence (`workspace.rs:1917`). Consumer program identity and role order are intentionally excluded from this preparation key: the preparation algorithm is fixed by the typed registry, while each invocation captures its own ordered sources and constructs its own declarations (`uniform.rs:154–180`). Authored fragments are separately keyed, so changed options do not discard unrelated immutable metadata.

The model's shared values retain their allocation charges with the last `Arc` borrower. Compiler scheduling hands the registry only to actual uniform consumers and drops its coordinator reference after the last remaining demand; launched tasks retain their own references (`compilation.rs`, `UpperPreparation` and `release_unused`). This is a bounded operation-local registry, not a second persistent cache. Serializing fragment construction avoids duplicate initialization. Wider overlap is a separate US02 decision; no claim of concurrent preparation speed follows.

**C0 metadata (UC02).** `load_metadata` now dispatches the model's metadata demand rather than all `CatalogData` rich inputs. SourceArtifact uses the existing classifier projection. Definitions, method parameters and provider runs are retained because they affect the operation. Public root loading still uses `CatalogData` and selected rich premises; complete expected coverage and invocation admission are performed independently before positive selected output (`catalog_core.rs`, `load_metadata` and `produce`). Sparse output therefore does not redefine the negative universe.

**Native inventory (UC03).** Before obtaining a session or consuming compact fields, the adapter selects `NativeInventory::checked_inputs` from the actual private checked authority and requires those exact descriptors in the current workspace (`analysis_prepare.rs`, `native_inventory`). That requirement derives from the complete model-owned native stage, including its dependency closure and support validators; it is not merely a list of assertion/support IDs (`analysis/native.rs`, `checked_inputs`; `analysis/preparation.rs`, `native_stage`). `compile_driver` establishes normalized semantic admission before upper execution and supplies that authority to the native stage.

The generated 75-pair inventory projects qualification ID/assumptions, assertion ID/qualification, and support ID/assertion/fidelity. Those fields suffice for the remaining inventory operation because rich row identity, native attribution and support predicates were earned by the matched semantic boundary. Profile and actual assertion-family presence still control pair acquisition. Missing or foreign qualifications, nonempty assumptions and wrong support correspondence still refuse. A complete no-output traversal precedes output declaration/emission, preserving late-failure atomicity. The full `visit` path and native support validators remain for independently received content. The public property visitor itself grants no checked input or publication authority.

**Retrieval (UC04).** Root and subject headers use set predicates; the adapter does not hide scalar reads inside the batched method. Ordinary compatible membership comes from prepared closure keys; the artifact-specific selection is lowered as partition-tagged `UNION ALL` work. The single selected rich union establishes actual row presence, and `finish_memberships` intersects candidate closure keys with it, preserving the former selected-table behavior for dangling references (`retrieval_preparation.rs`, `headers`, `collect_memberships`, `membership_query`, `load_window`). Artifact selection can legitimately name rows outside raw graph closure; `new_selected` therefore uses the actual filtered membership rather than pretending closure membership is sufficient.

The final `Preparation::prepare` captures and retains its model-declared exact `CompletedInputs`; the session and aliases used for window hydration come from that capture. Neither `render_window` nor `RenderWindow::render` accepts a caller-supplied input set, and original-range acquisition uses the captured input owner too. This removes the possibility of pairing one preparation's topology/session with another input set's nominal read permits. The capture retains its own descriptor charge; private finite fixtures carry no capture and refuse production window acquisition. The correction strengthens the existing acquisition boundary rather than manufacturing semantic authority.

The owning index validates declared projected schemas against canonical fields and binds selected membership to physical backing. It preserves every physical duplicate and source order, gives duplicate root requests separate partitions, and refuses use of ordinary closure gather on an explicitly selected window (`scoped_positions.rs:404–497`). Single-partition selected gather traverses its prepared positions directly. Original text chunks still follow model-owned selected ranges and the existing original-demand owner; contexts and supporting metadata do not become new public roots.

**Completion and frames (UC05).** Completion separately batches statement and whole-body roots; it hydrates each input union once and preserves the number and order of typed visits (`execution_scope.rs`, `input_visits`, `CompletionWindow`; `semantic_execution.rs`, `complete_base`). Structural and Analytic batch selected frames through `FrameWindow`, retaining exact prefixes and checking typed input permits when visiting. Summary batches deduplicated input/context frames, hydrates overlapping inventories once per exact port, retains source property projections, then performs the original ordered readers and per-frame model operation (`semantic_summaries/scope.rs`, `hydrate`, `read_record`; `semantic_summaries.rs`, `produce`). Its SCC/worklist/proof/adoption algorithm is unchanged by this acquisition migration.

The 32-root windows bound simultaneous root count, not the size of a skewed root; actual backing, preparation and gather reservations remain necessary. Holding one selected union can use more transient memory than a scalar root, but avoids repeated shared hydration and releases at window end. This is a credible local tradeoff for overlap and sparse partitions. It does not establish whole-library feasibility, eliminate remaining algorithmic growth, or qualify capacity under concurrent full compilation.

## Change and failure scenarios

| Scenario | Assessment and owner |
|---|---|
| Two owners share exact common sources but have different methods, model roots or genuine empty domains | Shared properties do not carry a result; fresh model declarations and ordered receipts retain each owner's meaning. Definition changes are isolated to authored fragments. |
| Context/profile/model/pool/epoch/view changes | Actual source and operation compatibility prevents reuse; private checked authority must independently match the native closure. Source order can vary without becoming a preparation dependency because fresh ordered declarations remain separate. |
| New native family or stronger predicate | The finite generated pair declaration and model stage closure remain the extension owner. A predicate not carried by admission reopens property sufficiency; it cannot be discharged by a nominal ID. |
| Failed or cancelled preparation, extra/early EOF or late support failure | Partial uniform fragments never enter the registry; inventory validity completes before output begins. Cancellation drops private state and charged owners rather than publishing a successful prefix. |
| Overlapping, duplicate, absent or skewed roots | Positions retain physical multiplicity and ordered rows; explicit retrieval membership intersects actual presence. Absent header requests still refuse, while legitimate empty selected input remains empty. Root count and byte reservations address different bounds. |
| Replacement of SQL hydration or Arrow selection | Owned semantic membership and typed visitor contracts remain the boundary. The acquisition/index implementation can change without altering model definitions, fresh outputs or independent receiving. |

## Standards judgment, alternatives and findings

| Architectural judgment | Verdict and basis |
|---|---|
| A1 Change locality | Satisfied in scope: semantic demand remains model-owned; physical selection/preparation is confined to the existing execution owners. |
| A2 Model adequacy and authority | Satisfied in scope: request state, native qualifications/supports, selected universes and ordered sources govern behavior rather than merely labelling output. |
| A3 Composition and local reasoning | Satisfied in scope: checked inputs, immutable properties, fresh outcomes and indexed windows compose without a new framework or duplicated admission authority. |
| A4 Execution fit | Satisfied for the bounded acquisition/preparation correction: unchanged common work is amortized, discarded reads disappear, headers cross in sets and selected gathers avoid per-root union masks. Whole-product execution fit remains outside this assessment. |

FP-01–FP-07 are satisfied within the stated scenarios. In particular DP-01/03/04/08/09/19/20 hold through the semantic owners, complete checked dependency matching and charged source/backing lifetimes; DP-10/14/16 are supported by the eliminated repeated work and reuse of existing primitives. CI-02/03/04/05/06/11 remain satisfied in scope: no relabelled relation, collapsed physical evidence, unknown-to-absence conversion, widened domain or new claim beyond the existing model was identified.

| Gates | Source verdict | Scope evidence |
|---|---|---|
| G1 Authority, G2 Semantic fidelity, G3 Validity | pass | Model-owned demands and domains; complete checked native prerequisite selection; independent receiving retained |
| G4 Hidden behavior, G5 Consistency/recovery | pass | Read-only selected acquisition; no operator effects or successful partial preparation; explicit retained owners |
| G6 Transformation/reuse | pass | Exact compatibility, schemas/ports/epochs, source order and physical multiplicity preserved |
| G7 Truthful capability claims | pass | Implemented/source claims separated here from unrun functional and unmeasured performance claims |
| G8 Library leverage | pass | Existing DataFusion relation execution and Arrow selection composed with a narrow model-specific membership index |
| CI-G1 Fidelity, CI-G2 Evidence closure | pass in changed boundary | No new semantic interpretation; original qualifications, supports and same-input receipts remain |
| CI-G3 Evaluation integrity | n.a. to changed boundary | No evaluator, gold or heldout input/parameter path changed; no whole-repository audit of those paths is claimed |

**New material findings: none.** Source findings F05–F09 are correction obligations considered here, not new IDs or duplicated disposition rows. Their current closure and focused evidence remain with coordinator §6.

The baseline scalar/full-decode route has simpler local acquisition but repeats work across owners and overlapping roots. A global rich cache or persisted proof flag would add invalidation/retention or cross-trust meaning absent a consumer need. The selected operation-local fragments and existing owning position index remove the diagnosed work with narrower validity and lifetime boundaries. A cardinality-one window remains the simplest path for a single root; keeping an independent legacy scalar loader would duplicate the contract without a current benefit.

The inspected lockfile pins DataFusion 55.1.0 and Arrow 59.3.0, matching the selected offline DataFusion capability skill. Its `arrow.select` brief supports indices for ordered gathers, including physical duplicates, and relational filters/projections before expensive transfer. The production gather uses Arrow's `take_record_batch`; bespoke code supplies model-specific membership and ownership rather than recreating that kernel. No new dependency or version change is needed. This comparison is Interface-checked; it is not a new runtime probe or broad datatype qualification.

## Controls, uncertainty and decision boundary

Control source covers exact EOF refusal; registry identity reuse and refused/cancelled initialization; foreign model/profile/pool/context/epoch/view selection; all 75 native property adapter schemas; exact pairs/fidelity/qualified alternatives, missing/foreign qualification and late invalid support; rich identity tampering on independent receiving; explicit selected positions outside raw closure, physical duplicates, schema/port/pool refusal and cancellation; Completion bodies and duplicate roots; analytical prefix partitions and visitor refusal; and projected Summary/native evidence. The corrected changed-view control uses an explicit Package contribution to grow the current composed view while the captured old view remains immutable, rather than attempting a forbidden second ordinary owner. The corrected C0 control completes the rich ProviderSymbol relation but excludes its port from metadata's actual captured access: the rich read permit refuses while metadata loading succeeds. A separate finite wrong-schema Package batch passed to the actual `CatalogData::visit(ProviderSymbol)` checks rich adapter refusal; it does not mutate captured aliases or establish native corruption rejection. Existing independently authored domain controls cover genuine empty uniform domains and Catalog expected frontiers. Several controls inspect intermediate acquisition/index semantics, so their existence alone must not be promoted to end-to-end consumer acceptance.

These final test-only setup refinements and the Summary hydrator's `Vec`-to-slice parameter adjustment were inspected on 2026-10-17. They do not alter the production semantic assessment. The coordinator reported the preceding native run `a82dcf` as three passed and two failed during test setup; this reviewer did not inspect that receipt or infer success from the corrections. Its rerun remains coordinator-owned.

| Check | Outcome in this review |
|---|---|
| Read-only `git diff`, targeted `rg`, source/type inspection, pinned `reference.py show arrow.select` | passed as inspection/navigation on 2026-10-17; no functional execution implied |
| Build and selected functional controls | not_run by reviewer; coordinator owns execution and final receipts |
| Services, full Catalog, assembled qualification, embedding/MCP and quantitative measurement | not_run / outside authorized review scope |

The final native inventory integration control source was inspected after it landed. It creates actual completed native views for the full Catalog prerequisite closure, obtains actual checked inputs, verifies that qualification-only weakened authority refuses without publishing inventory, and verifies exact empty premise/qualification output with valid authority. This is meaningful adapter-boundary source coverage, but it has not been run by this reviewer and does not establish nonempty native projection end to end. Separate model controls challenge positive pair/fidelity/qualification semantics and all 75 property adapter schemas. The coordinator owns the control's execution receipt; a later repair to trust, selection or lifetime behavior warrants focused re-review.

**Authority changes (slot 11): none.** The recommendations require no further rule replacement. Accepted RC03/RC04/RC07 and ADR-0186 remain the decision context; this review neither edits them nor treats acceptance as verification.

**Bounded decision (slot 12): Accept scoped.** A1–A4 and the applicable source gates hold for UC01–UC05 acquisition/preparation and adjacent receiving contracts. The result permits coordinator integration and focused verification. It does not close wider UN/UA/US obligations, historical upper timeouts or native finality. Protected historical Unknowns and cancelled build attachment `637d96c8…` retain their original obligations; this source assessment supplies no drainage or cleanup evidence. The enclosing architecture and full Catalog/assembled qualification remain unproven.
