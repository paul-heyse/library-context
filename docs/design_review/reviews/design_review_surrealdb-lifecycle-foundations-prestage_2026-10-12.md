# SurrealDB lifecycle foundations: pre-stage design review

**Decision: Accept scoped, at Proposed design maturity, 2026-10-12.** The proposed foundations have coherent owners and credible physical routes. They correct the original review's concrete defects without making optional database features or historical data preservation prerequisites. This accepts their architectural direction before implementation; it does not establish corrected production behavior, beta runtime compatibility or assembled qualification.

## 1. Review contract and boundary

Independent delegated **design/target** review of the consequential new boundaries in the former SurrealDB lifecycle coordinator, native, development and integrations plans dated 2026-10-12. Those reviewed documents are recoverable from Git at the baseline below through [historical recovery](../../README.md#historical-recovery); this judgment does not claim review of their successor Catalog plans. Baseline: `9a7ad7685c995293f60063bf66aa6f1532ab4f41`; the tree was clean when inspection began. The coordinator's subsequent concurrent architectural edits are not claimed reviewed here.

Standard: repository core **3.3**, efficiency heuristics **1.0**, code-intelligence profile **1.5**, and the library-context binding. The profile tightens fidelity and selected evidence requirements. Its current skill selects target-purpose judgment; the binding's legacy change/conformance label is immaterial to this design-tier review.

The subject is LC00 and the proposed F01–F05 foundations, generated schema integration, independent remote test ownership and typed diagnostics. Review inputs include the coordinator/native worker's additional proposal for a narrow persisted completion-result helper. Source inspection covers decisive current seams; it does not repeat the enclosing codebase review. The [original review](design_review_surrealdb-lifecycle-target_2026-10-12.md) remains the source of F01–F05. Its findings' sole current disposition remains [coordinator §6](../../plans/catalog-creation-plan_2026-10-14.md#6-sole-disposition-and-execution-checkpoint).

Workload premises are several pinned libraries/releases, large originals, overlapping dependency views, high-degree evidence hubs, concurrent equal compiles and interrupted test campaigns. No cloud deployment or historical benchmark workload is presumed. Source evidence is sufficient for the pre-stage architectural decision. Builds, database queries, services, fixtures, probes and product tests are **not_run** by this reviewer.

## 2. Responsibilities and governing meaning

| Owner | Proposed operation and contract | Adjacent consumer / reason for change |
|---|---|---|
| Model | Own diagnostic vocabulary and independent completion axes; semantic fact/admission meanings remain unchanged | Native projection and host inspectors consume a safe DTO, rather than classify human prose |
| Native selected access | Translate a held exact reader and selection intent into protected physical candidates/rows, including contributor-held aliases | Serving retains operation/path interpretation but stops reconstructing physical membership |
| Native completion/audit | Independently observe bytes and complete inventories; retain exact validity premises; converge permitted equal completion | Compiler preparation shares observation, not a root's independent admission |
| Native request owner | Preserve original issuance, request and durable terminal result | Completion recaptures only after a conclusive allowed application outcome |
| Rust schema owner / beta adapter | Generate definitions once; invoke qualified development machinery | Native installation and beta application consume the same source; neither introspection nor a template is another semantic authority |
| Service/run/storage owners | Register, supervise, authenticate and dispose a wholly owned remote target | The beta worker may create private clients/random databases without becoming their final disposal authority |

The fact/fidelity distinctions affected by these operations are preserved:

| Family | Authority and fidelity | Identity / allowed interpretation |
|---|---|---|
| Catalog members and public access | Model-owned normalized/derived records | Exact selected physical copy; nominal equality does not authorize a foreign copy |
| Original evidence | Captured authored bytes plus independent source/layout/range associations | Equal bytes may share observation without collapsing source identity or evidence status |
| Completion, admission and publication | Independently earned physical completeness, semantic eligibility and visibility | A completeness cache, successful schema suite or committed request cannot manufacture admission |
| Diagnostics | Owner-issued observations of known outcomes | Unknown remains unknown; inspection/recovery routes confer no destructive authority |

No provider, normalization, behavioral analysis or evaluation stage changes meaning in this slice. Native selection is a physical projection over the held universe, audit is independent reconstruction of declared outputs, and beta schema testing is development evidence. None becomes a new semantic analysis or synthesis authority.

## 3. Native contracts, composition and alternatives

The source diagnosis is sound. Serving queries still bind logical view IDs and use `compiler_view_member.view` ([selection](../../../crates/lctx-serving/src/selection.rs), lines 179–205, 261–267 and 371; [evidence](../../../crates/lctx-serving/src/evidence.rs), lines 529–538). Exact held construction already protects physical references ([reader](../../../crates/lctx-native/src/reader.rs), lines 1701 onward), and [SelectedPayloads](../../../crates/lctx-native/src/selection.rs) owns the reusable selected mechanism. Extending that surface is preferable to adding a retired field, returning raw realization SQL to serving or allowing global alias traversal. Empty demands still validate the reader, and source/continuation evidence retains its pin through terminal drainage.

Current independent auditing checks complete counts/digests and canonical backing, but fetches backing/holds/endpoints per member ([audit](../../../crates/lctx-native/src/installed_view/audit.rs), lines 148–230). The proposed indexed windows and keyed set fetches preserve assurance while removing scalar transport granularity. A returned row limit alone is insufficient: key sets, endpoint expansion, decoding and retained results must remain charged. A supported large item may occupy its own window or partition subordinate work. A generic query engine or a limit that rejects ordinary supported high-degree inputs is a worse replacement.

`audit_original` checks each range and then rereads its entire blob (same file, lines 234–250). Operation-scoped observation reuse is the simplest suitable correction. Its authority is the live protected scope, not merely a cache key. An `(identity, incarnation, revision)` tuple is insufficient without the mapped writer/exclusion premises that make it valid. Each source/layout/range and each root's descriptor, aliases, inventory, final completion and admission still has its own checks. Shared contributor observation can be reused; root acceptance cannot. More persistent witnesses remain an investigation, rather than machinery required to repair demonstrable duplicate work.

Current `complete_blob` captures a revision, reads bytes and rejects a changed revision before considering an already completed equal blob ([originals](../../../crates/lctx-native/src/originals.rs), lines 194–213). Owned recapture and full revalidation of the same active incarnation is preferable to global serialization, moving the completed check ahead of validity or treating every conflict as retryable.

The additional typed-result proposal is a credible implementation of that contract. A narrow helper may persist `Applied` or `OriginalAuditPremiseChanged` atomically in the original effect receipt. The latter commits request terminality **without a blob mutation**. It is allowed only for the permitted same-active-incarnation premise change; owner/era fencing, layout/declaration mismatch, retirement and ABA remain refusals. The caller preserves the initial incarnation across recapture. It must not adopt a recreated object.

This requires extending the existing request owner, not wrapping its current unit-returning API with a follow-up guess. [Authorization](../../../crates/lctx-native/src/control/authorization.rs), lines 265–297, currently computes and discards the payload result, reconciles only the transaction disposition, then removes its recovery reference before returning. The new helper must acquire and validate the exact persisted application outcome while that reference is still held, including committed-after-error reconciliation and replay. A missing, unknown or mismatched outcome never permits a new effect. Replay returns the original code without executing the payload again. Request commitment, application outcome and cleanup failure remain distinct. Ordinary effect APIs need not become a generic arbitrary-result framework.

These preservation constraints are part of the accepted scoped design, not additional finding closures. Deterministic native interleavings remain necessary implementation evidence.

## 4. Development target and diagnostic contracts

The proposed service boundary appropriately contains beta-owned private clients and random suite databases. It is a campaign-level remote service reused across suites, not a new service per test or a fallback from ordinary shared validation. The beta is SurrealKit `1.0.0-beta.1`; SDK/server 3.3.0 and local patches are distinct identities. Exact feature closure, Any session behavior and parser/transformation equivalence remain LD00/LD01 investigations. This review does not transfer those Interface-checked library claims into runtime acceptance.

The existing service adapter is a suitable owner: [installation_unit](../../../scripts/surrealdb_service.py), lines 260–270, derives campaign units from checked UUIDs, and descriptor/runtime validation binds endpoint, generation, databases and credentials. One concrete adaptation is essential: `install`, lines 692–709, currently publishes the campaign's storage consumer only after `_install` succeeds. The lifecycle target must persist its purpose, recovery identity and storage consumer before launch, so setup failure does not leave an unowned service. Merely renaming the post-success consumer is insufficient.

The whole exclusively owned service is the recoverable inventory boundary when a beta report never appears. Disposal rechecks exact campaign identity under the existing admission/maintenance locks, drains actual workers/attachments/effects, stops the owned daemon and confirms executor drainage before discarding its datastore. An acknowledged logical DROP is neither physical reclamation nor proof of an abandoned operation's outcome. Whole-target shutdown/discard may settle disposal after failed logical cleanup without preserving old bytes. Unknown drainage prevents deletion. No shared/operator state is touched to prove these controls.

Generated files are appropriate interchange, provided native Rust definitions remain their sole source. Beta default-module test projection needs the complete requested dependency closure; ignored selectors cannot silently narrow coverage. File addition/removal, permissions, nested definitions, functions and indexes require actual equivalence controls. Explicit native application of an unsupported generated asset is a viable bounded alternative to widening beta transformation. Handwritten duplicate schemas and moving canonical installation under development watch/prune are not.

The diagnostic remedy also fits its actual seam. [Completion](../../../crates/lctx-model/src/domain/completion.rs), lines 41–130, already separates outcome axes. [Signal receipt writing](../../../crates/lctx-native/src/signal.rs), lines 159–171, reduces failures to step/text. A model-owned bounded projection enriched by native typed wrappers repairs that loss without making model depend on native implementation. Actual fixture/report consumers must migrate together; unknown causes retain absent identities and opaque classification. The observer may explain an owner-specific route, but cannot authorize it. No historical receipt reader or global lifecycle registry is warranted.

### Focused follow-up: reuse executable provenance without coupling installations

The coordinator subsequently proposed a campaign-only explicit `--server-generation` binding so a fresh target can reuse an existing qualified patched server executable. This is appropriate: [server descriptor validation](../../../scripts/surrealdb_server.py), lines 161–235, binds provenance to its original service tools directory, while [installed binary validation](../../../scripts/surrealdb_service.py), lines 122–144, currently requires that origin to equal the current installation. Rewriting provenance to the campaign would be false; rebuilding identical software for each campaign adds unnecessary work.

The bounded change should preserve the immutable original descriptor/provenance, record its checked origin and register this campaign's durable consumer/dependencies on the **original generation** before launch under storage admission. Retain that consumer through actual campaign drainage; release only its own consumer afterward. Revalidate complete descriptor, digest, version and supported patched recipe rather than trusting the version or origin string. Origin authorizes reuse of the executable bytes, never operation of the source service. New datastore, service generation, credentials and endpoints remain independent. Stable installation and server-handoff validation are unchanged. A failure after consumer publication retains a recoverable installation obligation until cleanup settles it.

The coordinator also proposed a **manifest-only vendoring patch** to exact beta.1: disable SDK defaults, retain explicitly needed remote protocols/Rustls and omit client-side `jwks`. This is a suitable remote-only dependency correction, rather than an API code fork. The inspected registry beta manifest unconditionally enables SDK `jwks`; [patched SDK features](../../../third_party/surrealdb/Cargo.toml), line 76, map it to the local engine's JWKS support. Remote server authentication does not require embedding that engine in the client. Preserve upstream source identity and exact manifest delta, compile actual Sync/tester/token-actor paths and do not change server authentication features. The reported local-engine/DiskANN compilation failure is not claimed independently reproduced or fixed by this static assessment.

## 5. Revealing change and failure scenarios

| Scenario / kind | Locality and preserved contract | Required settling evidence |
|---|---|---|
| Add deployment evidence / domain extension | Model and generated declarations evolve; selected hydration consumes them without an automatic lifecycle job per family | New small selected record/evidence case; no duplicated schema authority |
| Replace a beta schema transformation / mechanism substitution | Development adapter changes; native installer, model and product admission stay stable | Definition and permission behavior against the same generated source |
| Two equal compiles overlap / concurrency | Same-incarnation recapture belongs to completion; original effect owner settles each submitted result | Barrier-controlled equal completion/replay, retirement/ABA and uncertain-effect controls |
| Large originals and overlapping roots / growth | Bounded set access and held scoped observation amortize transport/full-byte work without dropping associations | Multibatch/high-degree cases, independent inventories and structural observation counts |
| Failure before beta report / interruption | Prepublished campaign identity covers the whole target; result and disposal remain separate | Setup failure, fail-fast, cancellation, idempotent disposal and wrong-target refusal |
| Child exits with Unknown remote state / failure | Host inspector preserves uncertainty and names the actual owner; it does not infer cleanup | Actual persisted receipt through the host/report path |

These are proposed verification requirements, not freshly passed controls. No latency, throughput or capacity claim is made. Source inspection establishes the physical amplification and credible correction without requiring a historical benchmark.

## 6. Independent judgments, gates and decision

| Judgment | Proposed boundary verdict | Reason |
|---|---|---|
| A1 — Localize change | Satisfied | Selection, completion, diagnostics and target ownership each retain a coherent owner and narrow consumer operation |
| A2 — Encode domain meaning | Satisfied | Physical copy, eligibility, application outcome, request finality, retention and uncertainty remain distinct and govern the proposed operations |
| A3 — Extend through composition | Satisfied | Existing native selection/request/stream and host service/run/storage capabilities are extended rather than replaced with competing authorities |
| A4 — Fit execution | Satisfied at design strength | Set-oriented access and protected scoped reuse address the concrete growth route; service-per-campaign contains private clients without per-test process amplification |

FP-01–FP-07 are satisfied for these proposed boundaries. DP-08/09/10/15/19/21/23/24 and CI-04/05/08/11/13 support the same judgments. This does not label the current defective implementation satisfied.

| Gate | Verdict for the proposed supported boundary | Evidence / limitation |
|---|---|---|
| G1 Authority | Pass | One model/schema authority; native owns selected access and effects; host owns external disposal |
| G2 Semantic fidelity | Pass | Associations, selected copies, aliases and independent outcome axes remain explicit |
| G3 Validity | Pass at design strength | Live protected premises and final comparisons are required; runtime controls remain outstanding |
| G4 Hidden behavior | Pass | Inspection is read-only; watch/prune and disposal require an explicitly owned target |
| G5 Consistency/recovery | Pass at design strength | Exact original request/result and whole-target recovery survive missing reports; uncertainty cannot initiate fresh effects |
| G6 Transformation/reuse | Pass at design strength | Scope-limited observations preserve independent consumers; schema transformation must prove equivalence or use explicit native application |
| G7 Capability claims | Pass | Beta/runtime compatibility and substitutions remain investigations, with explicit refusal rather than fallback |
| G8 Library leverage | Pass | Native bulk access and beta development capabilities are composed with their missing ownership guarantees |
| CI-G1 Fidelity | Pass, scoped | No new relation or heuristic meaning; unknown and partial remain distinct |
| CI-G2 Evidence closure | Pass at design strength | Exact selection and evidence retain publication/definition ownership through the journey |
| CI-G3 Evaluation integrity | Not applicable | No evaluator, oracle, heldout or production-analysis input change is proposed |

**New findings: none.** The source F01–F05 remain the scheduled defects; this review neither duplicates their IDs nor closes them. **New rule impacts: none.** The scoped direction consumes the already confirmed source-review RC01–RC06. Any additional rule change discovered in dependency, target or capability investigation requires its own disposition.

**Accept scoped** enables implementation of the foundations under the contracts above. Excluded from acceptance are optional retained witnesses/native substitutions, beta runtime equivalence, the integrated implementation and full lifecycle track. Reopen the pre-stage decision if the writer map cannot establish the protected reuse scope, a typed result cannot be retained through original-request reconciliation, or beta requires broader target privilege/ownership than the whole disposable-service boundary can contain. Integrated design review and the selected qualification endpoint remain required; neither is replaced by this static judgment.
