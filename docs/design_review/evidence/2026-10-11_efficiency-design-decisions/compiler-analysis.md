# Compiler and analysis design decisions

**Investigated 2026-10-11; Proposed design, bounded Tested probes.** Source baseline
`439f3d2dd73135cfada43a58e90361a56a96668e`, main and the owned
`/home/paul/library-context-wt/efficiency-decisions` checkout. This evidence supports the new
[compiler plan](../../../plans/efficiency-alignment-compiler-plan_2026-10-11.md#4-decisions-resolved-for-implementation-2026-10-11)
and [analysis plan](../../../plans/efficiency-alignment-analysis-plan_2026-10-11.md#4-resolved-analysis-decisions-2026-10-11).
The coordinator owns finding disposition. C01–C08/A02/A04/A06 remain **Planned**, including when a
bounded decision probe passed. No product implementation is integrated and no throughput/RSS claim
or real-library qualification is made.

The inquiry used worker/executor and execute-plan contracts, core principles 3.3 with the efficiency
heuristics, semantic-model §15, current source review/audit, and the pinned DataFusion/Python-analyzers/
Rust-graphs skills. Older plans and protected gold/heldout populations were not consulted. The actual
provider probe uses a generated two-module fixture and no database, service, managed environment sync,
FastMCP compilation or fixture wrapper. Product prototypes exist only in the bounded worktree.

## Actual input and provider inventory

| Producer / owner | Result-affecting inputs observed in source | Chosen dependency/profile boundary |
|---|---|---|
| Acquire; `cpg-extract/src/acquisition.rs:440–535,696–833` | Captured revision/artifacts/chunk bytes; origin; installed library/requirement/lock/installer; distribution name/version/first-party/artifact hashes/RECORD digest; file ownership/role; corpus library link; interpreter/platform; derived block/receipt source correspondence | Exact acquisition descriptor and capture settings. ProfileNeutral; output production has no profile branch. |
| Pyrefly with canonical latest-Ruff supplier; `pyrefly_stage.rs:272–489,495–620,1519–1635` | All captured import-readable roots and supporting files; role-selected roots; linked installed environment/lock/interpreter/platform; canonicalized resolver settings; SyntaxLimits nodes/depth/source_bytes/callbacks; Ruff typing_modules/custom_builtins/row/node limits; supplier builds; authored model requirements and behavioral dependency/MRO enrichment | Full contribution ProfileSpecific. Separate neutral semantic source context from behavioral model selection; no whole-provider cross-profile claim. |
| Documents; `document_parser.rs:497–524,537–715` | Module/Occurrence/Syntax/Declaration/PublicName/ExportOrigin/ProviderModule exact views, captured selected docs/derived blocks, fixed Markdown limits, source context | ProfileNeutral algorithm after semantic-context split; all declared input views remain exact. |
| Deployment; `deployment.rs:49–72,321` | SourceArtifact/ArtifactUse/EnvironmentFingerprint/Package/Release/InputAcquisition/CorpusLibrary; selected receipt/configuration bytes and fixed parsing limits; source context | ProfileNeutral algorithm after semantic-context split; exact evidence dependencies. |
| Assemble; `assembly.rs:58–69,90–176` | All contributed vocabulary, exact captured analysis context and acquisition identity; catalog Flow NotRequested branch | ProfileSpecific; single vocabulary writer, not another analyzer constructor. |
| TyFlow; `ty_flow.rs:89–119,584–709` | Behavioral-only; canonical occurrence/placement/role correspondence, exact selected sources, native interpreter/environment/context and flow limits | Behavioral-only request. Catalog has explicit Flow NotRequested from Assemble, not a sixth executing catalog provider. |

The current executable composition is `cpg-extract/src/contracts.rs:7–18`; production enters it from
`cpg-core/src/facts.rs:92`. Actual catalog execution is Acquire/Pyrefly/Documents/Deployment/Assemble;
behavioral additionally executes TyFlow. There is no independently named TyType provider in the
examined `cpg-extract/src`/`lib.rs`/`contracts.rs` scope. Pyrefly `type_records` supplies native type facts.
This is a bounded source inventory, not an assertion about future providers.

`bundle::CapturedInputs` owns AcquiredInput values and native configuration. Current
`contracts::captured_sources` retains only input/context/configuration. Acquire additionally emits
InputOrigin/InputAcquisition, InputDistribution/DistributionVerification, ArtifactOwnership/
UnownedArtifact, ArtifactUse, CorpusLibrary and DerivedArtifact. Consequently source-byte/context
identity alone cannot describe its output. The new exact acquisition descriptor must be derived from
these same emission inputs, canonicalize semantic sets and name corpus links by input identity rather
than ephemeral vector position. It includes absence/membership and selected import closure, not just
files on which facts were emitted. Absolute temporary capture paths remain outside semantic identity.

## C01–C03: semantic ownership, eligibility and held installation

**C01 selected:** extend existing `lctx-model::domain::producer_contract`; move pure facts inventory
there, leave analyzer constructors/build identities/copy ownership in extraction and scheduling above.
The current lower owner already supplies the types; another contract crate has no independent
consumer advantage. D02 owns the independently justified native/compiler-store physical split and
embedding trait boundary. Feature gating alone does not narrow `producer_fingerprint.rs`'s declared
normal/build dependency/source-directory closure. Keep extractor fingerprints independent of readers;
native/compiler fingerprints remain conservative until responsible physical closure is established.

**C02 selected:** full request equality at whole-provider grain. Complete and explicit uniformly
NotRequested outcomes have different eligibility premises. Complete requires exact declared output
inventory including empty relations and admission; uniform NotRequested requires exact domain,
method definitions, unrequested availability and its new model validator. Partial/Unavailable/Failed/
poisoned/abandoned/truncated candidates miss. Corrupt equal-key candidates fail rather than being
misclassified as ordinary request misses. A provider-specific partial policy is not selected.

Current `admission.rs:698–711` rejects generic ProviderOutcome::NotRequested reconciliation; neither
that enum nor absent rows can supply the new compact coverage contract implicitly. The product-model
probe exercises this exact restriction and confirms complete-empty reconciliation separately.

**C03 selected with native owner:** `HeldInstalledView` includes exact logical view/completed identity,
physical record/incarnation, selected logical→physical contribution mapping, codec,
`view_realization_id`/member namespace, `view_closure_identity`, durable installation-completion ID and
hold owner. Completion freezes only this completed view's contributor/dependency closure, not all
unrelated active producers or the N06 final admitted attempt. This avoids intermediate-read/finalization
cycles. Compiler preflight compares request/outcome/admission and receiver prerequisites before effects;
native then binds immutable references atomically. Protection remains through handoff/rejection or
recovery cleanup. A naked logical digest, row count, old attempt admission or first global contributor
match is rejected. Actual native race/retirement/import controls belong to N04/N09, not this memory sink.

## C04: pinned expressions and exact physical member demand

Context7 first resolved `/apache/datafusion`; its current documentation was a discovery lead. Exact
55.1.0 source and the pinned `df.pushdown` brief establish the selected contract: scan filters are a
conjunction, Inexact retains residual evaluation, Exact requires complete predicate enforcement, and
limits cannot move below residual filters. Source entry is
`lctx-surrealdb/src/compiler_provider.rs:192–206,800–858` and pinned
`datafusion-expr-55.1.0/src/expr.rs`.

Extract only mandatory finite id equality/positive IN from coerced Expr, either equality operand order;
intersect recognized AND sets. NULL equality/empty or all-NULL positive IN gives explicit empty demand;
NULL members of a mixed positive IN contribute no matching key. OR supplies no driver. Preserve all
residuals and columns until evaluation. Arbitrary column/literal casts are not stripped: unrecognized
coercions retain the existing scan; exact identity cases can be recognized only by the locked typed
contract. The chosen extractor is conservative rather than attempting a second expression optimizer.

N04's direct physical address is
`H(native-view-member/v2, held.view_realization_id, relation, key)`. It commits mapping/incarnations and
codec; bare logical view is insufficient when equivalent content has distinct physical references.
Validate the handle even for empty/key-only/zero-column demand. Dense access uses the existing sorted
run. The finite independent SQL three-valued reference compared 55 combinations over a five-key domain;
actual Expr coercion/native installation/projection/refusal controls remain future implementation work.

## C05: exact receipts and a concrete configuration/context split

The selected consumer matrix is in compiler §4 C05. Source facts and receipt-bearing Structural/Local/
Summary/synthesis/catalog/retrieval outputs keep exact provenance. The initial pure subset is condition/
topology preparation, numeric window scoring, and embedding inference over exact preprocessed input;
regenerate fresh current receipts/uses through explicit adapters. Normalized kernel CompleteDomain
products retain every source/qualification/nominal ID present in their canonical rows.

Decisive source: `analysis/family/invocation.rs:4–20` makes sources part of AnalysisInvocation's key;
its source validator checks current exact SourceSnapshots. `compilation_product.rs:103–113` hashes
section bytes in value_identity. `normalize/kernel_products.rs:84–100` serializes canonical complete
row bodies including IDs. Thus request-independent value identity does not erase source IDs already
in rows. The actual model probe changes dependency provenance alone (same value hash), then changes
section source IDs (different value hash). No stale row can acquire new attribution by relabelling.

The chosen split is `CapturedSemanticContext` (canonical resolver configuration, interpreter/platform,
environment/lock and Ruff interpretation) versus `ModelContextSelection` (catalog/requirements and
behavioral selection). The former owns source observation contexts; the latter is an exact dependency
of full Pyrefly/flow enrichment. Acquire drops irrelevant analysis-profile coupling and remains exact
over its complete acquisition descriptor. Documents and Deployment declare neutral algorithms after
migrating their context, retaining exact actual input views; an input difference still misses. Pyrefly
full output remains profile-specific because requirements add dependencies and transitive MRO evidence
(`pyrefly_stage.rs:1519,1606–1625`). Assembly explicitly differs on catalog Flow availability. This is
an implementation dependency split, not permission to remove one profile field from current hashes.
Validator/model/result contracts and current publication profile remain separate exact identities.

The O-F18 cutoff is relation-local: declared RelationValue consumers hash exact ordered OutputContent,
local semantic result contract and outcome/completeness/domain, including empty content; they exclude
sibling outputs, producing contributor/request/build and whole Spec lineage. ActualSource remains an
independent exact held dependency for fresh source/invocation/proof receipts. Pure compute hits never
replay old receipt-bearing rows. This requires migrating current selected_product_dependencies and
completed_product_tokens, not adding a debug graph cutoff.

Current `analysis/preparation.rs:108–150` hashes all configuration outputs into one stage; CatalogCore
`catalog/build.rs:869–874` grants all configuration relations. Replace these with five independently
requested groups: authored catalog, method definitions/parameters (per-method subgroup requests in a
single collection writer), projections, AnalyticsConfiguration, RetrievalDefinition. Product Specs
hash selected groups/local parameters, never the whole PreparedCompilation. CatalogCore selects its
Catalog definition/parameters and genuinely consumed groups. Retrieval selects RetrievalDefinition
and actually used evidence/synthesis method groups; unrelated structural/optional analytics settings
are excluded. Encoder, service, recipe and projection config remain separately selected by their
consumers. Add separated CatalogCore payload construction and Retrieval rendering products keyed by
RelationValue, with typed source-role/row-key bindings resolved by the fresh emitter; source receipts
and validators remain exact. An actual changed synthesis payload still invalidates Retrieval.

Required future control: techniques off plus unrelated seed/bounds configuration change preserves
CatalogCore/Retrieval configuration tokens and compute requests while regenerating current exact
receipts. Sibling definition/build-only changes with identical relation content also cut off; consumed
parameter/source-ID/outcome/completeness changes invalidate. This is a specified consumer/configuration
migration, not an observed integrated cache hit.


## C06: compact domain-total NotRequested and demanded empty unions

Choose one model-owned uniform unrequested descriptor over exact input/context/source/definition and
method/domain identities. All four consumers—frontier expected coverage, FrontierCheck.finish,
compilation publication assessments and scoped serving availability—call the same scoped operation.
Inside the exact unrequested domain, answer NotRequested; outside it, answer Unknown/no-scope as
actually declared. Requested+NotRequested and changed domains refuse. Requested complete with explicit
empty values remains Complete; Partial retains scoped residuals. Mixed scopes retain ordinary rows.
No consumer synthesizes coverage from missing rows. The plan's four-consumer truth table is the
implementation contract and requires coordinated model/codebook/validator cutover.

Build a union once only when a real consumer demands its complete contributor inventory. Explicit
zero-row contributors and demanded empty unions remain completed objects; no demand creates none.
Same key/value duplicates coalesce, same key/different payload conflicts. The finite references cover
empty/unknown/outside domain and explicit empty/conflicting union cases; production migration is not run.

## C07: existing pool, preserved separate Ruff walks, finite acknowledgment

`pyrefly_stage.rs:105–107,700` already owns a OnceLock AllThreads pool and joins its root transaction
before ordered extraction. Dependency batches currently use an inline joined transaction at1587;
reuse the same pool when migrating those bulk independent dependency waves, retaining joins before
serial correspondence extraction. No per-root pool or new long-lived analyzer process is selected.
`ruff_context.rs:35–131` already parses once; diagnostics and semantic-observe walks stay separate.
This investigation selects preparation/index reuse without a fused diagnostic walk. Consequently it
makes no unsupported noqa/deferred-rule parity claim for an unimplemented fusion.

N02 owns two acknowledgments. `IngressReceipt(sequence,charged_bytes)` transfers finite charge ownership
to the sink. `await_committed`/finish returns NativeWriteReceipt with original request and acknowledged/
reconciled disposition; only terminal write/recovery cleanup releases bytes. Acceptance closes before
drain, including accepted sends after caller failure/cancel. A queued send is not native completion.
Provider final outcome follows drain; no new unbounded queue or worker cap.

Captured-input integrity selects a complete-tree bracket, not a claim about read-only permissions.
Move the `facts.rs:24–26` opening byte check into CapturedTreeRunGuard and include exact closed manifest
membership/absence, no-symlink paths, sizes and hashes. Retain capture ownership through all readers.
After Pyrefly/Documents/Deployment/TyFlow's final joined tree read, StageContext::close verifies the
whole captured closure before writer finish/sink.finish and contribution/product completion. Pyrefly
`pyrefly_stage.rs:688` reads its resolution tree; analyzed-root CanonicalSyntax verification at828 and
Natives' path/artifact indexes do not replace this closure check. All import-readable captured support
files are covered, including ones with no emitted root facts. Ambient files remain forbidden; pinned
built-in analyzer resources belong to the supplier contract.

Remove Acquire's `acquisition.rs:856` extra full pass because `CapturedInput::emit_chunks` already uses
ArtifactCapture::finish to verify each emitted artifact's full digest/length (`capture.rs:401–437`).
Assembly has no tree reader and no closing pass. Mismatch poisons the provider/attempt before Complete,
retains prior contributions only as failed-attempt staging/recovery, and drains accepted ingress.
The supported contract is ordinary exclusive owner control and persistent mutation detection, not
adversarial modify-and-restore protection. A native verified VFS is the stronger future alternative,
not an unselected requirement for this increment. Whole closure post-read/membership/tamper/cancel
controls remain not_run; the actual provider probe does not establish the new bracket implementation.


## C08: locked Postcard envelope and allocation-before-decode policy

Context7 resolved `/websites/rs_postcard_postcard`; local source is **postcard1.1.3**, already in
Cargo.lock. `src/ser/serializer.rs:194,262` and `src/de/deserializer.rs:393,453` establish length-plus-byte
encoding and borrowing. Choose it over unlocked bincode, JSON base64 expansion, RawValue JSON coupling
or a second Arrow IPC body/schema migration. Exact canonical section bytes are preserved.

Use the plan's format-2 borrowed frame and preflight exact current request/count/names/order/lengths
and row/section limits before owned Serde allocation. Borrowed slices reject malicious huge counts,
truncation and trailing bytes. Admission separately covers retained input plus request/section-owned
containers and body copies before allocation; owning only the encoded buffer is insufficient.
Destination typed-body/ID/order/conflict and semantic admission checks remain independent. The probe
uses the selected borrowed frame, keeps section bytes identical, rejects u64::MAX section count before
owned allocation and refuses insufficient decoded budget. It is a physical codec fixture, not a native
cache integration or proof of all product-schema migrations.

Observed fixture: 5,702 section bytes → 5,743 Postcard envelope bytes versus 19,907 JSON envelope bytes.
This is a **Measured encoding size for one synthetic fixture**, not throughput, resident memory or
representative cache capacity. Keep cache enabled by default, bounded root-window lifecycle batching,
per-entry holds/generation/readback and quota-fenced admission. Select quota-counter-based planning
plus indexed stable-key eviction; reject mutable offsets and ordinary per-insert SUM scans. Counter
repair belongs to explicit recovery. Native races/retention remain future actual controls.

## A02/A04/A06: selected complete alternatives

**A02:** retain ordered adjacency once per immutable selected frame; reuse existing graph entity index
and unresolved-owner/subsystem preparation. Borrow raw edge descriptors; preserve current visited-node
step validation including invalid suffixes before budget truncation. Per-seed frontier/path charges
follow bounds. A bounded next-edge heap still rescans degree every seed and needs an extra edge to
prove Arcs versus exhaustion. Full sorted preparation better matches repeated Structural seeds, with
one admitted O(E) representation. Independent finite references compare 10,128 ordered BFS/witness/
stop cases and a 100,000-edge hub. They do not test the unchanged production traversal. Parallel ArcIds,
first witnesses, depth/vertex/arc accounting and error exposure are future migrated controls.

**A04:** retain component-order F64 accumulation over admitted F32 projections. DataFusion55.1.0
`datafusion-functions-nested/src/inner_product.rs:167–217` uses the same scalar F64 sum after List<F64>
coercion. It can agree on admitted inputs; replacement is rejected for the complete route's extra
pair/list/F64 buffer construction while still needing model best-window/tie/ID/work/FiniteF64 checks.
This is a valid simpler-kernel choice, not a claim no library can score correctly. Its cosine_distance
would renormalize an already rounded projection; `embedding/projection.rs:38–58` specifies F64 norm then
one F32 round. The finite discriminator shows F32 reductions/score rounding can collapse near ties;
it does not accuse DataFusion's F64 route of that failure. Prepare input correspondence once, bound
retained candidates by top-k, evaluate all exact pairs and retain frame-local co-use refusal/method.

**A06:** current `embedding_realization.rs:439–507` already deduplicates and flushes64 by budget;
analytic_embedding.rs:211 uses windows64, retrieval.rs:343 uses roots32 but eagerly prepares per
partition (`retrieval.rs:44–65`). Extend that Session with queue_ready/flush_ready; do not add an
independent scheduler. While one root Output exists, model-checked PreparedWindowUse captures exact
preprocessed input/window/spec/token/availability premises currently checked by
`retrieval/consumption.rs:132–223`. Queue only distinct ready text; publish mandatory root rows and drop
rich payloads. After flush, finalize exact per-window uses/refusals against compact expectations and
independently validate final published records. Keep configured Session spec/service, seeded analytic
winners and cancellation/drain ownership. Do not retain all32 rich roots or rerender them. A wide root
can flush repeatedly. The plan specifies exact constructor/finalizer and consumer migrations; actual
embedding/service transport remains not_run, and fake transport could establish composition only.

## Commands and outcomes

| Command / scope | Outcome, 2026-10-11 |
|---|---|
| `cargo test --release -p cpg-extract --test efficiency_decision_probe -- --nocapture` in bounded worktree | **passed**; actual provider population control plus actual model identity/reconciliation control; raw/native-provider.log |
| Same probe's earlier assumption that an empty behavioral source yields Complete | **failed**, revealing expected Partial under behavioral model requirements; retained raw/native-provider-eligibility-counterexample.log. Corrected scoped expectation retains Partial as reuse-ineligible. |
| `python3 -I …/compiler-analysis/finite_decisions.py` | **passed**; independent finite traversal, SQL three-valued demand, scalar numeric, expanded availability/empty-union and relation-local configuration references; raw/finite-decisions.json |
| `cargo run --release --offline --manifest-path …/compiler-analysis/portable-codec/Cargo.toml --target-dir …/portable-codec/target` in bounded worktree | **passed**; locked codec fixture, borrowed preflight and separate decoded admission; raw/portable-codec.log |
| Product native read/attach/cache/coverage/schema migration and actual embedding transport | **not_run**; future implementation/integration scope, not established by these probes |
| Repo quality/generators/whole-suite/real-library campaigns | **not_run**; coordinator assignment excludes them |

Native baseline populations: catalog209 declared relations/603 rows, behavioral245/4,569; outcomes
Acquire Complete, Pyrefly Partial, Documents/Deployment/Assemble Complete, and behavioral TyFlow
Complete. Relocation/repeat/reversed file acquisition order and one-row transfers preserve complete
observed populations. Origin changes, same-origin role changes, same-byte RECORD changes, settings
and import-cycle changes each alter them. Current CapturedSourceBinding stays equal for origin/role/
RECORD changes, directly demonstrating PD01. Acquire population identity agrees across profiles.

A smaller `x=1` plus empty companion source fixture has all-Complete **catalog** provider outcomes;
its reverse/one-row run agrees. Behavioral Pyrefly remains Partial even on that minimal fixture and
empty sources; this is not promoted to eligibility. No complete behavioral whole-provider reuse claim
is made. No persisted hit is executed: C02 semantic request/eligibility and N04 attach remain Planned.

Probe inputs/source live in compiler-analysis/native_provider_probe.rs and portable-codec/;
raw outputs live under compiler-analysis/raw/. The sink holds typed Batch allocations under the real
ResourceBudget and bounds auxiliary observation size. It is deliberately a finite in-memory sink;
it exercises native provider computation and typed emission, not persistent completion/admission.
No shared fixture, operator database, protected evaluation or service state was touched.

To reproduce the actual-provider control, copy `compiler-analysis/native_provider_probe.rs` to
`crates/cpg-extract/tests/efficiency_decision_probe.rs` **only in the owned baseline worktree**, then
run the Cargo command above. The standalone codec manifest/source/lock are self-contained; pass its
worktree-local target directory as above. Source copies match the passed native and codec logs.
These are decision probes, not additional product/test targets to integrate into main.
