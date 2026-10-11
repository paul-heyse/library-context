# Efficiency alignment: compiler, capture and reuse

**Proposed, 2026-10-11.** Companion to the new
[coordinator](efficiency-alignment-plan_2026-10-11.md), the sole owner of progress, finding
disposition, baseline and operator decisions. Sources are original O-F07/O-F16–O-F30/O-F46 and
audit A-F02/A-F04/A-F05/A-F07/A-F16/A-F20. Existing plans were not consulted or superseded.
No implementation or product execution occurred during authoring.

## 1. Target and foundation assessment

Compile over an immutable, completely described captured input. Reuse eligible completed provider
results before invoking providers; attach admitted native contributions by reference; demand only
the exact relation keys needed by the consumer. Separate relation value identity from exact evidence
identity so unchanged values can cut off propagation without attributing old receipts to new inputs.
Keep portable reuse enabled by default while reducing its encoding, crossing and retention costs.

The existing provider sink, declaration and stage contracts already support composition. A new generic
provider framework is unnecessary. The lower contract boundary must make semantic inventories usable
by cold admission without importing executable analyzer constructors. Native owns physical installation,
incarnation and protection; the compiler owns request/spec equality and outcome eligibility. The model
owns availability, coverage, identities and provenance, including explicit empty and NotRequested.

Current native view members already have direct `(view, relation, key)` addresses and a unique view/key
index. The required foundation is a durable, validated installation-complete fact, not a new lookup
architecture. Existing sorted full-run access remains appropriate for dense demand. Current profile
and configuration keys are conservative but too broad; narrowing them is safe only after dependencies
and evidence-bearing outputs have explicit contracts.

A patch release with unchanged source bytes but a different input role is a revealing case. Capture
must distinguish byte identity from result-affecting acquisition metadata. A second revealing case
changes source spans while retaining relation values: a downstream value-only computation can be
reused, but receipts claiming the old exact sources cannot be relabelled. These cases drive the design.

## 2. Packages

<a id="C01"></a>
### C01 — Separate provider contracts from executable composition

**Sources:** O-F46; A-F12. Entry points include `cpg-extract` provider `bundle.rs`, `cpg-core`
workspace/facts admission and `scripts/producer_fingerprint.rs` (locate the current fingerprint
implementation before editing). Preserve `ProviderSink`, declarations and `ProviderStage` unless
an actual consumer requires a change. Move captured input/configuration contracts and the semantic
provider inventory below executable adapters. Composition remains with the compiler entrypoint.

Expose the narrow native candidate/held-inspection/attach/completion operations needed by C03/C04.
The native layer must not decide whether two captures are semantically interchangeable. Cold facts
admission must not construct or link analyzers merely to learn their output contracts.

D02 owns physical crate migrations and dependency validation; C01 owns the semantic boundary. The
fingerprint walker currently follows normal/build local dependencies and source directories. Feature
gates alone do not narrow that identity. Retain all responsible model, ingestion, codec and derivation
implementation in their owning fingerprints; a reader-only change should not invalidate extraction.

**Closure:** publisher, embedding and contract-only compiler consumers build through their actual lower
contracts without unrelated analyzer implementations. An analyzer/ingestion/model change still changes
the responsible producer identity; an unrelated reader change does not. Verify actual dependency and
fingerprint behavior, not a diagram alone. C02 can specify complete keys before all crate moves finish.

<a id="C02"></a>
### C02 — Describe complete captured requests and eligible outcomes

**Sources:** O-F16/RC02, O-F19/RC05; A-F04. One authoritative inventory constructs both the product
request and contribution specification. Include selected bytes, additions/deletions and absence,
provider/supplier implementation, settings, interpreter/platform/environment context, upstream exact
or value dependencies, declared output contracts and requested availability.

**Planning discovery PD01:** `CapturedSourceBinding` alone is insufficient. Acquisition emits origins,
roles, ownership, distributions/RECORD and derivations from `AcquiredInput`/`LibraryInventory` in
addition to bytes. Two captures with identical files but different tree labels can emit different
InputOrigin; changing selected roles changes ArtifactUse. Include all result-affecting acquisition
metadata in capture/request identity, separately from byte content identity. Start from acquisition's
actual emission inputs rather than assuming a binding digest is complete.

Initially admit reuse only for complete results and valid explicitly NotRequested results whose
specification guarantees substitutability. Failed, abandoned, poisoned and resource-partial outcomes
are not generally reusable. Any future provider-specific partial policy must establish equivalent
coverage and outcome semantics. Start at whole-provider grain; per-input reuse requires independent
input/context correspondence and is not a prerequisite for this track.

**Closure:** unchanged capture avoids provider execution; changing bytes, absence, origins, roles,
settings, implementation or a relevant upstream view misses. Same bytes under a new role must not
reuse old attributed facts. Fresh attribution is honest, not a new receipt attached to old claims.
The exact declaration inventory includes empty outputs. Use controlled inputs, not protected evaluation
payloads, to distinguish these cases.

<a id="C03"></a>
### C03 — Preflight held candidates, then attach by reference

**Sources:** O-F16/O-F17/RC03; A-F04. Consume N04's exact-incarnation hold and installation-complete
contract. Find and hold a candidate, check request/spec/outcome, admission and recovered closure,
then validate the receiving attempt's prerequisites before any effects. Attach/bind its immutable
content through native operations. Keep protection until durable handoff or completed rejection cleanup.

Terminal previously-unadmitted results enter admission-for-reuse under N09's recovery owner. Prior
attempt admission is not required, but current complete candidate admission is. Distinguish ordinary
ineligibility (miss), corruption (failure) and unresolved effects (recoverable/unknown). Never turn a
partial prefix or a row count into evidence of complete installation.

Replace N-row attachment copies with view-local logical-to-physical contribution references. Migrate
all contribution enumeration, selected backup, detached export/import, recovery, audit and retirement
consumers together. A first global contributor match is not authority for the selected view.

**Closure:** a hit attaches without copying all row pointers; rejected candidates leave no receiver
mutation; a retiring or incomplete candidate cannot pass; shared content survives one borrower's
release; imported and recovered candidates establish destination-local protection. Actual native
controls are required for physical claims, with pure controls for semantic eligibility.

<a id="C04"></a>
### C04 — Use exact view-member demand without reconstructing the universe

**Sources:** O-F21/O-F22; A-F02. Recognize mandatory id equality/IN demand in the actual pinned
DataFusion expression/coercion contract; retain residual predicates. An OR branch is not mandatory
key demand. Verify expression handling against the locked library before changing it.

Use the existing member address `H(view, relation, key)` for bounded multi-get after N04 establishes
validated view completion. Small exact requests must not grow with foreign attempt history or scan
all sorted membership. Key-only requests return compact keys without payload hydration; hydrate only
where the consumer or residual needs values. Dense demand may still use one sorted run.

Retain deterministic sorted/distinct output, explicit empty requests, view/shape validation, virtual
and absent relation meaning, alias resolution, nominal conflicts, charges and drain. An empty input
set does not permit an invalid view to pass unnoticed.

**Closure:** sparse, dense, duplicate, missing, virtual, foreign-backing and incomplete-view cases;
mandatory versus non-mandatory filters; key-only versus payload demand; equal results/refusals against
an independently enumerated small admitted view. Query-shape/source evidence establishes selectivity;
latency measurement is external. This replaces the earlier owner-by-key open design question.

<a id="C05"></a>
### C05 — Separate value change from exact provenance and profile meaning

**Sources:** O-F18/O-F19/RC04/RC05; A-F05. Split unrelated configuration outputs before narrowing
reuse. Define relation-local canonical value identity, output contract/outcome/completeness, exact
source evidence, and current attachment as distinct facts. Dependencies declare whether they consume
exact views or proven value equivalence. Metadata/provenance consumers retain exact dependencies.

`AnalysisInvocationID` and source receipts include exact source identity, and their checks verify it.
Receipt-bearing outputs cannot be reused by refreshing a spec label. Either keep their exact key or
regenerate the provenance-bearing result through a defined adapter while reusing an actually separable
value computation. Make the choice per consumer and migrate its validator with it.

Use explicit profile-neutral versus profile-specific declarations. Removing one profile field from a
hash is insufficient: native context includes profile/requirements and changes availability; provider
runs retain genuinely requested families. Only providers and downstream outputs whose complete inputs,
coverage and output meaning are neutral can cross catalog/behavioral profiles.

Give cutoff one actual owner: the per-relation dependency/request construction and product lookup
path above. The existing `compilation/reuse.rs::DependencyGraph::affected` currently feeds a debug
count, not execution. Remove that unused cutoff graph as consumers migrate; preserve its real selected-
premise and double-binding conflict checks at the authoritative request/binding boundary, with equivalent
controls. Keep useful hit/miss tracing there. Do not retain charged graph traversal as a second apparent
cutoff authority or delete its meaningful validation along with its diagnostic-only reachability work.

**Closure:** unrelated config changes stop invalidating unaffected values; exact span/evidence changes
still update receipts; profile-neutral work reuses while requested availability and provider attribution
remain correct. A value hit cannot turn NotRequested into complete coverage or preserve a stale claim.

<a id="C06"></a>
### C06 — Form needed unions and compact uniform non-requested outcomes

**Sources:** O-F23/O-F24/O-F25/RC08; A-F05/A-F07. Completed descriptors always list every declared
output, including empty ones. Form union views when a declared consumer needs them, not after every
vocabulary contributor. A completed empty relation is a first-class result, never an absent relation.

For uniform NotRequested over an exact declared domain, retain a compact outcome with invocation,
exact source/definition and domain-total meaning. Do not manufacture per-kind invocation/output work
merely to repeat that outcome. Mixed or partial scopes retain their distinctions; no-scope and unknown
are different from domain-total non-request. The model owns scoped answers without inferring them
from missing rows.

Migrate frontier enumeration/coverage premises, `FrontierCheck.finish` scoped-set reconstruction,
compilation publication assessments and scoped serving availability together. Prepare expected inputs
once per compile using indexed inputs/profile/method extensions. This is invocation-local preparation,
not a persistent cache over arbitrary source-owner sets; cold admission remains independent.

**Closure:** equal final unions with explicit empty and conflict controls; exact complete, partial,
NotRequested and absent/unknown scope answers at all four consumer boundaries; changed domains cannot
inherit old total coverage. Scheduling fewer units must not suppress requested semantic work.

<a id="C07"></a>
### C07 — Reuse provider preparation while preserving capture and correspondence

**Sources:** O-F07/O-F26–O-F30; A-F07/A-F16/A-F20. Build path/use/derivation indexes once with
multi-owner meaning. Reuse Pyrefly span/path session preparation while preserving owned versus inherited
facts; use one joined pool across dependency waves rather than one per root. No fixed worker cap.

Index Ruff canonical events by site/name and preserve multiplicity, unique-site fallback, ambiguity
and unlocated facts. Fuse compatible Ruff observations only after establishing diagnostic key-multiset,
noqa and deferred-rule walk parity against the locked fork. A single walk is not worth altered fact
correspondence. Per-root serial post-processing remains until merged-input/lifetime parity is shown;
a new long-lived analyzer process is unnecessary for the current target.

Fuse SHA-256 with capture copying where the exact copied bytes are the hashed bytes. Keep point-of-use
integrity for tree analyzers: a private/read-only directory alone is not immutable ownership. Corpus
origin describes the selected captured tree: exact revision equality or base revision plus modified
content identity. Neither checkout HEAD nor unrelated dirty files decide selected-source provenance.

N02 owns scheduling against acknowledgement of persisted work; provider completion retains accepted
sends through sink failure, cancellation and drain. Do not move an unbounded queue behind a new batch API.

**Closure:** inherited/multiple-name/ambiguous correspondence; suppressed and deferred diagnostics;
joined-worker failure/drain; selected-file tamper versus unrelated dirty work; changed input roles;
same bytes through fused hash/copy; faithful source origin. Retain CLI parity oracles as independent
controls, not production facts providers. Do not open protected evaluation inputs.

<a id="C08"></a>
### C08 — Keep portable reuse, reduce its physical overhead

**Sources:** O-F20/RC18; A-F05. Keep the portable cache enabled by default. Batch lookup and retention
through charged root windows and shared lifecycle operations, preserving per-entry holds, generations,
readback and refusal. Distinguish quota-fenced admission from SUM-based eviction planning; their current
roles do not establish two competing admission authorities.

Replace JSON byte-array expansion with opaque section bytes under a deliberate format cutover. Carry
one checked canonical batch until its consumer/native ingress no longer needs it instead of decoding
and canonicalizing equivalent forms repeatedly. The cache remains optional: missing entries recompute;
corrupt entries are diagnosed under the existing policy, never accepted as valid compilation.

Bound eviction planning without making unmeasured claims about host capacity. A native pointer cache
is an alternative for a future need, not a prerequisite for correcting this cache. Preserve ordinary
warm reuse and lifecycle ownership rather than removing the feature to remove its costs.

**Closure:** bounded batch shape, stale generation and corrupt-section handling, quota races,
retained/absent/virtual outputs, equivalent cold compilation, and consumer release. Changing portable
encoding requires destination validation and explicit retirement of obsolete project formats.

## 3. Integration and extension boundaries

C01/D02 settle lower contracts; C02 establishes semantic keys. C03 requires working N04 protection and
completion, then enables genuine provider hits. C04 consumes the same native completion guarantee.
C05 is required before broad downstream cutoff/profile reuse. C06/C07/C08 have independent useful
slices and do not wait for the whole native plan. Shared workspace/ingress edits need one integration
owner even when their semantic prerequisites are independent.

A new provider declares its real acquisition/settings/dependencies and eligible outcomes. A new fact
family extends the model inventory and its consumer union without multiplying unrelated unions. A new
validator changes its admission identity without pretending old protected content was checked by it.
These extensions use existing owners, not another pipeline orchestrator.

Functional exit requires producer-hit/miss, receipt/coverage, direct selected access, portable-cache
and lifecycle controls through migrated consumers. Full real-library cold/warm and memory/crossover
campaigns remain external follow-ups in the coordinator. No performance result is claimed here.
