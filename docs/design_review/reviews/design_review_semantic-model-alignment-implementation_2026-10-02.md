# Semantic-model alignment implementation review

## 1. Scope, outcome and coverage

**Decision: Accept scoped, at Implemented evidence strength.** The inspected execution and serving boundaries follow the accepted alignment target after three bounded corrections made during this review. This is an architectural assessment of source, not functional acceptance, performance evidence or Q0 qualification.

| Field | Value |
|---|---|
| Standard | Core 3.2, code-intelligence profile 1.3, repository binding |
| Tier · purpose | Design · conformance |
| Reviewer · date | Independent delegated design reviewer, 2026-10-02 |
| Baseline | HEAD `ed2d68d7043c5d3769edd5efdaab928f12d326d3`, with uncommitted implementation and concurrent coordinator edits |
| Target | D0–D2 and E0–E5 in the four alignment plans |
| Method | Read-only inspection of declarations, implementations, adjacent consumers and controls; no writes, builds, tests, probes or services |
| Exclusions | N0–N3 receive separate native review; O4 producer-script capture and ADR-0115 were not independently assessed here; integrated gates, real-library qualification, operator activation and Q0 are outside this review |
| Prior assessment | The original incremental-alignment source review and its stable findings remain authoritative for their recorded baseline and disposition |

The decisive inspected boundary comprises:

- `crates/cpg-core/src/{compilation,consumed_rows,analytic,semantic_summaries,structural,synthesis,stage_runtime}.rs`;
- `crates/lctx-model/src/domain/{dependency_closure,stages,model}.rs`;
- analytics policy, definition and kernel consumers in `domain/analytics/`;
- closure consumers in catalog evidence, selection and analytics build modules;
- `domain/synthesis/{frames,automatic,production,seeds}.rs`;
- `domain/serving/{schema,requests,responses,packets,mappings,dispatch,failure}.rs`;
- PostgreSQL packet reads, packet/capability/evidence/operation hydrators and selection/catalog/retrieval prepared owners;
- existing `generations/runtime.rs` and the envelope bridge in `python/lctx_storage/src/serving.rs`;
- `python/lctx_mcp/src/lctx_mcp/wire.py`;
- the inspected model, stage and transport controls named in §10.

The final reinspection captured these mutable boundaries:

| Source | SHA-256 |
|---|---|
| `domain/serving/mappings.rs` | `c885cc1db37dbd3d627ca12bb7d8af23424da7408f567870c73c031d0cfbd8f4` |
| `generations/packet_reads.rs` | `ca0b9bd0e98a31ecb0c969ab640d88476f691c7330d45e1f8d8aa3de91a16741` |
| `lctx_mcp/wire.py` | `871a1ef0c1c2d45635de3253e8edf74bcc93ff7816528aebee0d4e817be90d59` |
| `domain/serving/packets.rs` | `43f1d88a3affb8ebb24e09f16b8afe63c8df0d2fe302489d9614201108a92e39` |

Subsequent material changes require reassessment of their affected boundary. These fingerprints do not certify the remainder of the dirty tree.

## 2. Responsibilities, dependencies and semantic ownership

The division of authority is coherent within this scope. The model owns analytic policy, dependency requirements, packet bindings, numeric code meaning and public failure categories. The compiler owns finite execution placement and stage dispatch. PostgreSQL owns acknowledged sources, prepared data, canonical reads and generation lifetimes. Python constructs the pinned transport representations and delegates admission to the original Rust execution grant.

| Phenomenon | Authority and consumers | Assessment |
|---|---|---|
| Stage phase, publication boundary, graph needs and runner | Private finite `UpperStage` binding in `compilation.rs`; schedule construction and execution | One binding governs the inspected orchestration |
| Exact validator universe versus sufficient read grant | `DependencyClosure`; C1, C2 and Analytic stage declarations | Distinct products retain distinct meanings |
| Consumed relation at a particular epoch | `ValidationInput`, acknowledged `CompletedRelation`, `ConsumedInputs` | Runtime selection remains source-bound |
| Retained analytic settings | `analytics/policy.rs`; method records, definition identity and kernels | Authored values share one owner |
| S0 common analytic parents | `frames::AnalyticParents`; frame admission and automatic synthesis | Exactly three common relations are shared |
| Canonical packet permissions | Sealed `PacketOutput`, finite `PacketKind`, prepared dependencies | Hydration and metadata derive from typed bindings |
| Recognized public failure | Rust `FailureKind` and `PublicFailure`; Python tool/resource adapters | Fixed messages replace backend detail |

The affected fact and fidelity boundary remains the existing typed model: extracted facts and resolved normalized relations feed model-qualified analyses; heuristic analytic results retain their technique and invocation records; synthesis and serving retain evidence identities. This inspection found no new promotion of heuristic results into extracted facts, no replacement of unknown with absence and no changed relationship multiplicity. Provider extraction fidelity itself was not re-audited.

Sources: `compilation.rs:21`, `dependency_closure.rs:5`, `consumed_rows.rs:9`, `analytics/policy.rs:8`, `synthesis/frames.rs:34`, `serving/mappings.rs:193`, `serving/failure.rs:4`.

## 3. Contracts, constraints and testing boundaries

Closure construction retains requirements by relation, epoch and order. Grant lowering uses the checked publication order rather than boundary enum codes. Incompatible transport, availability or validator policies refuse construction. Inferred ordinary facts may be omitted under the explicit lower-layer policy; directly declared facts remain inputs. Dependencies on unfinished ordinary outputs refuse.

`ConsumedInputs` resolves typed permits before loading, distinguishes different acknowledged epochs and coalesces aliases only for an identical acknowledged source. Each returned source receives a fresh `StageSession`, avoiding name-based registration collisions between epochs. Missing typed dispatch refuses at `finish()`.

Packet permission checks precede canonical reads, including empty attempted reads. Prepared selection, catalog identity and retrieval owners also check their declared read capabilities. These checks supplement existing canonical decoding, receipt and generation enforcement.

Failure admission preserves the original grant and actual request ID. It makes one complete-envelope attempt and falls back to a fixed safe exception if that attempt cannot be admitted. It does not promise a universal byte bound for early framework errors.

Sources: `dependency_closure.rs:38`, `dependency_closure.rs:62`, `dependency_closure.rs:116`, `consumed_rows.rs:42`, `consumed_rows.rs:81`, `analytic.rs:36`, `synthesis.rs:35`, `packet_reads.rs:18`, `selection.rs:61`, `retrieval_service.rs:76`, `wire.py:79`, `wire.py:116`.

## 4. Composition and execution

E0 derives the graph-needs union from scheduled bindings and prepares it at the first graph consumer. Prepared graphs continue to be borrowed through source-bound access rather than transferred into an unrelated task lifetime. The final scheduled declaration is checked before execution.

E4 uses borrowed synchronous closures in a Tokio blocking region on a multithread runtime, with explicit inline placement otherwise. Summary, structural and analytic adapters use the shared boundary. The independently spawned RSS sampler can progress while a blocking-region kernel runs. Normal completion joins the sampler; drop signals it to stop. Opaque kernels drain synchronously, and this implementation introduces no hard-cancellation claim.

E3 supplies method parameters and definition identity from the retained policy. Kernel consumers read the same damping, tolerance, iteration, community-grid, seed and other policy values. Fixed algorithm mechanics remain represented in the policy recipe and source identity; this is a retained-policy owner, not a general configurable algorithm framework.

E5 retains one production inventory of analytic frames, analytic invocations and technique results. Automatic synthesis borrows those parents. Replay validation retains its independent inventory and checks.

The inspected analysis records continue to carry their existing question, projection, method/settings, result class, invocation and evidence scope. Ranking/community/concept/neighbour kernels retain their declared approximation classes. No new numerical equivalence or speed claim is established by this review.

Sources: `compilation.rs:378`, `compilation.rs:546`, `compilation.rs:584`, `stage_runtime.rs:30`, `stage_runtime.rs:54`, `stage_runtime.rs:118`, `analytic.rs:130`, `analytics/policy.rs:24`, `analytics/policy.rs:40`, `synthesis/automatic.rs:113`, `synthesis/production.rs:49`, `synthesis/seeds.rs:205`.

## 5. Change and failure scenarios

| Scenario | Observed change locality and preserved meaning |
|---|---|
| Add an upper stage or change its graph requirement | Finite binding owns declaration, placement and dispatch; scheduler validation remains separate |
| Require one vocabulary relation at two epochs/orders | Exact requirements remain distinct; one sufficient grant does not erase the consumed universes |
| Change a retained analytic policy value | Parameters and definition identity change through the policy owner; kernel arguments use that owner |
| Add a nested packet or prepared dependency | Typed bindings compose children and prepared capabilities; attempted reads remain checked |
| Substitute CPU placement | The execution helper owns runtime-flavour handling; semantic kernels keep borrowed inputs |
| Healthy semantic refusal after ID admission | Original grant can still admit the fixed failure envelope |
| Guard loss, deadline or exhausted budget | Envelope admission can independently fail; fallback occurs once and does not acquire a fresh grant |

A healthy refusal does not blanket-poison `RequestExecution`. Its CPU and query paths retain the original execution while work drains, and confirmation remains independently enforced. Therefore admitted failure envelopes have a meaningful supported case. Guard, deadline and budget failures can prevent their admission; that is the accepted boundary, not evidence that every error can use structured metadata.

Sources: `generations/runtime.rs:187`, `generations/runtime.rs:227`, `generations/runtime.rs:250`, `lctx_storage/src/serving.rs:485`, `wire.py:195`, `wire.py:258`.

## 6. Correctness and fidelity gates

These verdicts concern the inspected static architectural boundary.

| Gate | Verdict | Evidence and limit |
|---|---|---|
| G1 Authority | pass | Finite stage binding, owned policy, typed packet declarations and Rust failure categories govern their consumers |
| G2 Semantic fidelity | pass | Epoch/order distinctions, numeric codes, required-field meaning and recognized failure categories are retained after corrections |
| G3 Validity | pass | Construction, permit selection and attempted-read rejection have defined enforcement points |
| G4 Hidden behavior | pass | Store effects remain outside model operations; CPU placement and telemetry are explicit |
| G5 Consistency and recovery | pass | Source-bound graph/read lifetimes and original-grant failure admission retain their existing ownership contracts |
| G6 Transformation and reuse | pass | Closure lowering preserves exact requirements; shared parents and immutable packet metadata have bounded semantic scopes |
| G7 Truthful capability claims | pass at Implemented strength | Source routes exist; runtime acceptance and performance remain unclaimed |
| G8 Library leverage | pass | Tokio and pinned MCP mechanisms supply placement and transport behavior; no competing generic framework was introduced |
| CI-G1 Fidelity | pass in changed boundary | Existing unknown, model qualification and heuristic distinctions remain represented |
| CI-G2 Grounding | pass in changed boundary | Packet scopes preserve canonical evidence identities and existing verification |
| CI-G3 Evaluation isolation | n.a. | No evaluation inputs, criteria or gold paths changed in the assessed boundary |

Runtime outcomes and enclosing qualification are not established by these static verdicts.

## 7. Findings and applicability

Three corrections were requested and reinspected during this review. No material architectural finding remains open within the assessed boundary.

| ID | Diagnosis | Reinspected correction |
|---|---|---|
| I01 | Packet binding construction rebuilt the complete model and allocated owned child/prepared bindings during requests without accounting for that work | Bindings and canonical declaration metadata now use immutable process-wide `OnceLock` storage; `PacketLease` borrows a static binding; child permission checks traverse shared references |
| I02 | A known oversize request-ID refusal lost `resource_refused` when resource fallback inspected only `exc.kind`; the own resource-refused `ValueError` branch likewise became unavailable | The locally constructed refusal now carries the recognized kind; the recognized branch emits the Rust-owned fixed resource-refused message |
| I03 | Several generated descriptions assigned absence meaning to required fields, or described condition truncation as page truncation | Required reason/signature-form descriptions and bounded condition-rendering description now match their types and behavior |

I01 concerned resource lifecycle and repeated metadata preparation, not the legitimacy of dynamic canonical proof support. The correction retains explicit `CanonicalProof` capability checked against canonical declarations. `lowered()` expands complete source metadata for identity/discovery, rather than each permission check.

Sources: `serving/mappings.rs:264`, `serving/mappings.rs:293`, `serving/mappings.rs:350`, `packet_reads.rs:7`, `wire.py:86`, `wire.py:214`, `serving/packets.rs:14`, `serving/packets.rs:71`, `serving/packets.rs:183`.

FP-01–FP-06 are satisfied within this boundary. The decisive supporting rules are DP-01–DP-03, DP-06, DP-08–DP-12, DP-17–DP-20 and DP-24. Corrections I01–I03 address respectively preparation/resource ownership, failure fidelity and discoverable contract meaning.

## 8. Library fit and total complexity

Tokio's borrowed blocking-region mechanism fits kernels that cannot transfer graph/access lifetimes into detached workers. The explicit inline fallback is necessary for current-thread and outside-runtime callers. An owned `spawn_blocking` conversion would require additional owned state and would weaken the present borrowing boundary unless separately designed.

Pinned MCP tool-result and resource-error mechanisms fit D2's two public surfaces. Python retains transport shaping; Rust retains failure meaning and admission. Neither a new transport stack nor a duplicate Python semantic model is justified.

Immutable shared declaration metadata is the simplest appropriate replacement for repeated packet/model construction. No provider registry, projection DSL or additional crate boundary is required.

## 9. Alternatives and tradeoffs

The implementation removes duplicated closure walks, independently authored policy values and open stage-name dispatch while retaining the accepted operations. Its finite structures improve ownership without converting kernel mechanics into a universal declarative system.

For packet reads, per-request owned metadata adds avoidable preparation and accounting burden; shared immutable declarations preserve the same permission question. For failure handling, blanket exceptions would discard structured admitted outcomes, while promising all early errors are bounded would exceed the present transport contract. The selected original-grant attempt plus one safe fallback preserves both limits.

No alternative requires broad redesign to resolve the inspected concerns.

## 10. Verification and uncertainty

**Reviewer-run functional checks: not_run.** No compile, test, probe, PostgreSQL operation or benchmark was executed by this reviewer. Coordinator receipts must identify their commands, tree and outcomes separately.

Inspected controls exist for:

- closure epoch/order preservation, publication ordinal ordering, inferred-facts policy and unfinished-output refusal in `lctx-model/tests/dependency_closure.rs`;
- retained-policy parameter and identity changes in `lctx-model/tests/analytics_settings.rs:94`;
- inline placement, borrowed closure behavior, independent heartbeat progress and sampler completion in `cpg-core/src/stage_runtime.rs:155`;
- numeric discovery descriptions and selected packet composition/permission checks in `lctx-model/tests/serving_contracts.rs:734`;
- existing original-grant envelope admission, resource bounds and canonical transport behavior in `lctx_mcp/tests/current_transport.py:430`.

Code existence is Implemented, not a passing test receipt.

Remaining functional acceptance evidence includes the plans' actual tool/resource failure envelopes with admitted IDs, failed-envelope fallback once, attempted empty undeclared reads, representative epoch-separated loader consumption, adapter lifetime/cancellation behavior, and existing synthesis/analytic semantic controls. Inspection of helper assertions alone does not replace those cases. No performance measurement is required to accept these functional opportunities.

Native semantics, producer capture, broad extraction fidelity and Q0 are unexamined breadth here, not inferred defects.

## 11. Authority changes and dispositions

No new architectural decision is requested by this review. The corrections fit the accepted alignment target.

The coordinator alignment plan remains the single disposition owner for scheduled work and links to the source review. This review does not close the original source findings by architectural acceptance alone. Their implementation and acceptance evidence belongs in the existing plan rows.

D0–D2 and E0–E5 functional completion remains subject to the existing controls and repository gate timing. Proposed obligations, Implemented routes and Tested receipts must remain separate.

## 12. Architectural judgment and decision

| Judgment | Verdict | Evidence |
|---|---|---|
| A1 Localize change | satisfied | Stage placement, policy values, dependency closure, packet permissions and failure categories have coherent owners and bounded consumers |
| A2 Encode domain meaning explicitly | satisfied in assessed scope | Exact epochs versus sufficient grants, unknown versus absence, analytic policy versus mechanics, canonical packet reads and recognized failure outcomes remain explicit and govern behavior |
| A3 Extend through composition | satisfied | Finite stage bindings, nested packet bindings, prepared dependencies and narrowly shared synthesis parents compose without adding parallel authorities |

**Bounded decision: Accept scoped / Implemented.** This permits the coordinator to complete focused controls and integrate the assessed implementation without further architectural expansion.

**Enclosing architecture: not certified.** N0–N3 require the separate native assessment; O4 and its authority changes require their own evidence; runtime qualification and Q0 remain outside this review. No speed benefit, real-library acceptance or operator activation is claimed.

Reopen this assessment for material changes to publication ordering, source selection, graph/access lifetime ownership, packet capability construction or the one-attempt failure-envelope boundary.

## Appendix A. Native N0–N3 conformance reinspection — 2026-10-02

**Decision: Accept scoped / Implemented.** A separate independent native reviewer inspected
HEAD `628b018db511792f5c8f8159d7ab80319adf1135` plus the uncommitted native boundary. Source
inspection establishes no functional pass, performance result, Q0 completion or activation.

The model owns recorded/private Entry replay, structural guard correspondence, Local
contributions, checked stability/rebase, Summary witnesses and finite continuation DAGs.
PostgreSQL retains declared invariant execution, receipt-bound hydration, original guard
ownership, actual stored-row checks and request effects. Borrowed classification rows produce
an owned public formal index; pure prepared semantics grant no repository admission.

| Inspected correction | Source boundary |
|---|---|
| Three context-construction paths retain the exact typed cause; unexamined paths preserve it without assignment/restriction | `native_requests/preparation.rs`, `evaluate.rs` |
| Formal domain charges before each novel member/domain/owner/formal pair; stored membership precharges before collection | `preparation.rs`, `generations/native_service.rs` |
| Every retained context requires coverage before checked preparation returns | `preparation.rs` |
| Invalid formal resolution retains Error::Contract | `native_service.rs` |
| Pure CPU preparation releases the lease mutex, retains charged inputs and original guard, then reconfirms before publication | `native_service.rs` |
| Delivery independently rejects unstored path, original condition and proof references | `native_service.rs` |
| Native identity captures inventory and moved preparation | `native_requests/mod.rs` |

A1–A3 are satisfied in this source boundary: the model owns semantic change, explicit governing
operations retain refusal/formal eligibility, and storage composes operations without reauthoring
meaning. No stronger heap/runtime or enclosing architectural claim follows.

Source-written controls now cover pure required/default/missing/crossed context preparation and
missing coverage, public formals without native contexts, duplicate/foreign inputs, charge release,
exact numeric refusal packets and live default transport. They were **not_run by the reviewer**;
the coordinator owns actual results.

The reviewer identified one test expectation error: the neutral Partial assertion applied to both
defaulted and computed cases. **Integrator correction, 2026-10-02:** it now applies to defaulted
only; computed preserves Available section status with an Unknown/ConditionTransferUnsupported
path. Production semantics did not change for that control repair.

| Inspected production source | SHA-256 |
|---|---|
| `native_requests/preparation.rs` | `b1e6d9b01bd557cba2b5bb2a5d4ec83517f0f22a0568310c49eab0aa440f55fb` |
| `native_requests/evaluate.rs` | `d759836234025d3ea152f5dd18b7e398706a442de0000221efb4409093c1aa61` |
| `native_requests/mod.rs` | `da4962a76a9fd4812f1fb289c40a5b9daf41fc868454452d69d414a76935bf14` |
| `generations/native_service.rs` | `f81bdaf26512dba14cad6e8223af96d1f4e146f3d27d49509abf544f092e0976` |

Reassess materially changed native boundaries. The coordinator's F01/F02 rows retain sole current
disposition; this appendix does not close them by architectural acceptance alone.

## Appendix B. Closure, typed dispatch and prefix follow-up — 2026-10-02

**Decision: Accept scoped / Implemented.** The original execution/serving reviewer reinspected
HEAD `761687bd6b9e3c4a97a11ca813a63a7f46348478` plus the uncommitted follow-up repairs. This
appendix supersedes the earlier closure assessment only for the three boundaries below. All
other scope exclusions, evidence limits and disposition ownership remain as recorded above.
No build, test, probe or service ran during this reinspection.

Follow-up qualification exposed two concrete distinctions absent from the original assessment:
complete upstream invariant premises need not be executable upper-stage reads, and a vocabulary
relation's later Catalog contributor need not belong to an earlier requested prefix. The repairs
preserve those distinctions without changing Catalog's Behavioral-only Ty execution policy.

| Reinspected boundary | Assessment and source |
|---|---|
| Exact requirements versus sufficient grants | `dependency_closure.rs:57` records root/direct names; invariant traversal at `:92` retains full inputs; grant projection at `:106` omits only inferred ordinary facts. Direct roots/direct grants and every vocabulary epoch remain granted. Unfinished-output rejection still precedes omission. |
| A1 consumed inventory versus typed dispatch | `analytic.rs:52` uses the model-owned `projection_outputs!` inventory. It covers the previously unmatched ProjectionGapSubject, ProjectionGap and ProjectionSourceCoverage declarations. Consumed roots remain authoritative and `ConsumedInputs::finish()` still refuses missing dispatch. |
| Analysis dependencies versus later Catalog vocabulary extensions | `compilation.rs:401` compares the producer's scheduled publication ordinal with the requested prefix. Contributors beyond that prefix do not cause false dependencies; visible Catalog contributors and unprefixed ordinary Catalog outputs still refuse. The final Schedule supplies the comparison order; runtime acknowledged-source enforcement remains separate. |

The lower-layer omission is justified by existing enforcement, not by treating missing Flow
results as available evidence. `compilation.rs:537` takes the facts checkpoint before upper work.
`generations/receipts.rs:344` freezes and validates its full relation scope, including unrequested
empty tables without a writer. `generations/stage_validation.rs:121` derives checkpoint-covered
relations; `:260` requires invariant premises to be declared or checkpoint-validated, preserves
their original ordered replay, and skips only fully covered invariants. Missing checkpoint proof
still refuses. No runtime permit, completion receipt or Flow writer is fabricated.

The existing execution plan §4 and ADR-0116 clarify this validation-premise/grant distinction.
These repairs satisfy A1–A3 and the affected G1/G3/G5/G6 obligations at static Implemented
strength: the model remains the owner, checkpoint proof remains enforced, and ordinal-aware
composition avoids introducing a second publication-order authority.

The new `extra_invariant_epoch_and_fact_premise_survive_grant_projection` control retains an
ordinary invariant premise without its inferred grant, preserves an explicit earlier vocabulary
epoch/order beside the owner's later epoch, and restores the ordinary grant when directly
consumed. The new `earlier_vocabulary_prefix_does_not_depend_on_later_catalog_extension` control
checks the valid earlier-prefix case. Both controls were inspected but **not_run by this reviewer**.
The existing dependency negative checks an ordinary Catalog output. A targeted visible Catalog
vocabulary contributor rejection, and a dedicated real-store missing-checkpoint-premise refusal,
were not found in the bounded inspected controls; those remain acceptance evidence gaps rather
than new architecture requirements. Coordinator receipts may establish them separately.

| Reinspected source | SHA-256 |
|---|---|
| `crates/lctx-model/src/domain/dependency_closure.rs` | `581792b8f3c902f321c0d72f5d6852540392dbdb81d276328180f3331ee4946a` |
| `crates/cpg-core/src/analytic.rs` | `473d9eb5357da5d7af67e0d9e0c655721f0d59827e91c932da23e5736e7cc0e1` |
| `crates/cpg-core/src/compilation.rs` | `5f57c82ec311eb0b1c155a475b05db9b10a29d57f4b5c67f06b2ce1a5edea325` |
| `crates/lctx-model/tests/dependency_closure.rs` | `7a4de8395c5bf8cecfccfdd83e488630187c7ed3d7fb156667f112631ffe70d8` |

This follow-up does not establish functional acceptance, close original source findings, qualify
the whole dirty tree, or alter stopped Q0. Reassess material changes to these fingerprinted
boundaries; acceptance outcomes remain with the coordinator's plan receipts.

## Appendix C. Final serializer and control follow-up — 2026-10-03

**Decision: Accept scoped / Implemented.** The independent reviewer reinspected HEAD
`761687bd6b9e3c4a97a11ca813a63a7f46348478` plus the current uncommitted corrections. No material
regression was found within the boundaries below. Appendices A and B retain their dated evidence;
this appendix records the subsequent source changes and controls. No build, test, probe or
service ran during this review.

`native_requests/preparation.rs` replaces the formal-domain tuple with `FormalScope`, retaining
the owner/formal membership rules and charges before novel insertions. Adjacent resolution still
requires one owner and rejects duplicate or foreign formal inputs. Borrow simplifications retain
the borrowed inventories; the `dependency_closure.rs` let-chain retains the existing requirement
merge precedence. Pure evaluation and the PostgreSQL native-service files remain unchanged from
Appendix A's fingerprints. This is a bounded regression assessment, not a new native qualification.

`wire.py:29` now selects the HTTP writer using the installed MCP 2.2.0 protocol families:
handshake HTTP uses Pydantic `model_dump_json(by_alias=True, exclude_unset=True)`, modern HTTP
uses compact `json.dumps` over the JSON-mode dump with aliases and excluded None values, and
stdio retains Pydantic serialization plus its newline. Static comparison covered the installed
`mcp/server/streamable_http.py:410`, `_streamable_http_modern.py:184`, `stdio.py:199` and
`mcp_types/version.py:33`, plus protocol routing and exception/result conversion. The recognized
resource-refusal subclass retains its coarse kind. Tool errors preserve `isError` and failure
metadata without success structured content; resource `MCPError` preserves `ErrorData.data`.
Original-grant envelope admission and single safe fallback remain intact. Healthy semantic
refusal does not itself poison a grant; guard, deadline or budget refusal can prevent the error
envelope and select that fallback. The existing universal early-error byte-bound gap remains
outside D2's admitted-envelope guarantee.

The source-written `current_transport.py:649` and `:736` controls cover four recognized kinds,
tools/resources, legacy/modern protocols and HTTP/stdio. Legacy requests initialize; modern
requests carry protocol and client metadata directly. They compare actual writer bytes with the
complete admitted envelope, including Unicode request IDs. `failure_seam.py` forces operation
outcomes while retaining the real opened generation, service, original grant and native envelope
admission. These are meaningful transport/admission controls, not evidence of backend failure
generation. The fallback control at `current_transport.py:832` checks one refused tool envelope
attempt with the original grant and no recursive replacement. All were **not_run by this reviewer**.

The two missing source controls recorded in Appendix B are now present:
`compilation.rs:905` includes rejection when the Catalog vocabulary contributor lies within the
requested prefix, and `lctx-postgres/tests/stage_validation.rs:173` checks real-store refusal of
an inferred fact premise without checkpoint proof or an explicit acknowledged source. Their
presence addresses the control-design gaps; passing acceptance evidence remains with the
coordinator. Complete invariant requirements, grant-only omission and checkpoint enforcement
remain as assessed in Appendix B.

| Reinspected source | SHA-256 |
|---|---|
| `crates/lctx-model/src/domain/native_requests/preparation.rs` | `4323fca41a472a5575b7c1f2ba0d5513f6614d8bf4eccb73b3cafa98ef4ca896` |
| `crates/lctx-model/src/domain/dependency_closure.rs` | `88c046ca4ae1afba770b84eda0f9f24172f4cc35a6e8556a7c0413934b0aa455` |
| `python/lctx_mcp/src/lctx_mcp/wire.py` | `ef56975bd655a3b58c742ee9c7a798a9dfb8f9472d7c68a2e21631954e6ee069` |
| `python/lctx_mcp/tests/current_transport.py` | `e00af21997beb65433c02f00fd2688a34dffa0296de7b7bc37ce3eb9c6896c79` |
| `python/lctx_mcp/tests/failure_seam.py` | `3a74cec07840829628d418fa5b96f557fa48f0ffc29b339ca0ca6c2909b90b55` |
| `crates/cpg-core/src/compilation.rs` | `946fcbaaa380410e1481703bdf0196a93acdfadeaf299427e5a82bfd23f4f02d` |
| `crates/lctx-postgres/tests/stage_validation.rs` | `b7b556e4b7bcf9cd8b607ececabca3a7f18f2588726586a16f1c9389319db40a` |

A1–A3 and the affected gates remain satisfied at static Implemented strength within these
interfaces. No Tested/Measured claim, original finding closure, whole-tree certification or
stopped-Q0 change follows. Material changes to these boundaries require reassessment; the
coordinator retains acceptance and disposition ownership.
