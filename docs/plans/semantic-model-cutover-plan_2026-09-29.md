# Plan: the semantic model cutover

**Active execution owner, 2026-09-29.** [ADR-0085](../adr/0085-typed-semantic-domain.md),
[ADR-0086](../adr/0086-immutable-postgresql-generations.md), and
[ADR-0087](../adr/0087-clean-semantic-reconstruction.md) govern this plan.
The target is [DESIGN §15](../design/sections/semantic-model.md).

## 1. Outcome and completion

Ordinary typed Rust domain definitions in `lctx-model` own atomic units, attributes, relationships,
identity and invariants. A bounded derive lowers those definitions to Arrow and metadata;
`lctx-postgres` lowers the validated model to PostgreSQL. PostgreSQL is the only relational store.
DataFusion is compute, not another schema authority. There are no compatibility readers, adapters,
legacy identity maps, old-store flags, or dual writers.

**Authorized scope:** reconstruct phases 0–2. Analysis, catalog and MCP availability is suspended
until phases 3–5 implement those capabilities on the same model. A facts-only generation advertises
that frontier and cannot answer higher-layer requests with empty tables. Product work stays paused.
The forward plan retains product context and unrelated findings.

**Qualification:** compile checks and focused tests during execution; formatting and the integrated
active-scope gate once all phase 0–2 functional work is implemented. A phase's decision is not evidence
of implementation. PostgreSQL qualification uses a real disposable server, including concurrency.
Independent semantic fixtures, oracles, benchmark assets and held-out isolation are preserved.

## 2. Baseline and reopened work

Baseline `a265758` contains an empty model membership registry, relation metadata separate from row
definitions, untyped IDs, mutable table specifications, partition publication and compatibility
machinery. The earlier phase-0 completion claim is **reopened**. The
[core review](../design_review/reviews/design_review_cutover-core_2026-09-29.md) is Revise.
Existing code is reusable only where it meets the new contract; it is not architectural authority.
The historical baseline had 57 raw families, 22 normalized families and 121 analysis/catalog families.
Every raw field must be mapped to a current typed owner before its source is deleted.

## 3. Global mechanisms

### 3.1 Typed domain ownership

`Id<T>` distinguishes targets. Entity keys, qualified assertion keys and support/run keys have
separate meanings. One generated typed key supplies equality, hashing and ID derivation. Conditions,
modality and approximation distinguish assertions; repeated supports never strengthen a conclusion.
Conflicting payload for an equal key is rejected unless an explicit domain merge owns it.
Field type, key participation, reference role and provenance are orthogonal. Collections of references
become relationship rows. Tagged sums enforce the active payload, absent inactive arms, and the
difference between an absent arm and a present optional value. Subtype references enforce the subtype.

A root manifest declares membership once. Only a privately constructed `ValidatedModel` admits
storage and execution. Generated declarations, Arrow schemas/codecs and relation inventories derive
from domain definitions. The derive handles actual shapes rather than becoming a general DSL.
`serde_arrow` 0.15.1 uses explicit schemas and Arrow 59; SeaQuery 1.0.2 lowers SQL in the store owner.
Salsa remains internal to ty 0.28.2. No new incremental or inference engine is introduced without a
consumer. SQLx, pgpq, the provider fork and the bounded BDD kernel keep their narrow responsibilities.

### 3.2 Immutable generation schemas

Each generation owns ordinary tables in its own schema. A stable control schema owns state,
manifests, receipts and selection. There are no parent partitions or attach/detach transitions.
Keys and foreign keys are generation-qualified; local cyclic references are supported by creating
tables and keys before references. Semantic and physical schema digests are distinct from producer
and content digests.

Lifecycle: staging → sealed → validated → published; failed and retired are terminal. Selection is a
separate pointer. Sealing waits for writes and removes writer access before stored-content validation.
A receipt binds sealed content to the complete required validator set. Publication changes state and
grants atomically. A reader validates model/schema identity and holds a lease for its lifetime.
Retirement requires an unselected generation and exclusive access, then removes schema and registry
state atomically. Lost leases invalidate readers. Failed attempts are never repaired in place.

### 3.3 Typed stage execution

Stage input/output declarations alone own production; relation definitions do not duplicate writers.
Model membership, writer uniqueness, required inputs, read-before-write and cycles (including self
cycles) are checked. Provider outcomes are Complete, Partial, Failed or NotRequested. Output assembly
preserves attribution and alternatives; consensus belongs to normalization. Each DataFusion stage
sees only declared inputs and shares one attempt memory pool. Boundaries accept `Batch<R>`.

## 4. Phases

<a id="phase-0--core-contracts-store-kernel-and-migration-tooling"></a>
### Phase 0 — Typed domain and executable contracts

| Package | Implementation and owner | Acceptance |
|---|---|---|
| P0.1 | Replace governing decisions and this plan; map raw fields to model owners | No active compatibility target; no dropped raw information |
| P0.2 | Domain groups: package/release/source/module; syntax/declaration; provider/context/run/assertion/support/coverage; call alternatives; type terms; flow/places/predicates/conditions; documents/package/deployment | Nominal references; acyclic source/occurrence keys; support separated from assertions |
| P0.3 | Bounded `lctx-model-macros` derive, generated key/schema/codec, root membership, `ValidatedModel` | Positive and compile-fail controls; invalid sums/subtypes/duplicate keys refused |
| P0.4 | Correct calls, structured paths, atom identity, transfer and verdict contracts | All C03–C13 controls below, retaining independent expected answers |
| P0.5 | Typed stages, provider outcome contracts, restricted sessions | Missing/double writer, unknown input, self-cycle, failure/not-requested controls |
| P0.6 | Production subset source → module → occurrence → assertion → support/coverage through permanent lowerings | Typed → Arrow → real PostgreSQL COPY → typed readback; fresh design review before scaling |

P0.4 requires structural length-tagged paths; occurrence-based atoms (including distinct with-items);
separate potential higher-order and direct invocation; one owner for receiver classification and call
normalization; complete target alternatives rather than row-count uniqueness. Transfer keys exclude
accumulating conditions. Composition extends paths only through identity, maps all bindings, preserves
local opaque guards and refuses unsupported substitutions. ControlInfluence and Selection are first-
class relationships. ScopeBoundary yields unknown; value approximation creates an obligation;
rendering truncation is not semantic uncertainty. Derivation premises are typed references.

### Phase 1 — PostgreSQL store and runtime cutover

| Package | Implementation and owner | Acceptance |
|---|---|---|
| P1.1 | `lctx-postgres` generated generation DDL and control catalog | Only validated model accepted; cyclic refs, codebooks, tagged sums and keys enforced |
| P1.2 | Seal/validate/publish/fail/select/retire lifecycle, COPY and receipts | Real concurrency controls: late writes, failed validation, failed publication, lease/retirement race |
| P1.3 | `cpg-core` provider sessions with pinned lease and digests | Readback fidelity; pinned read survives a different generation selection; digest mismatch rejected |
| P1.4 | Quiesce old project runtime; remove Delta, partition kernel, bundles, adapters, legacy IDs and runnable downstream orchestration | No alternative store or compatibility flag; independent semantic expectations retained |
| P1.5 | CLI model describe, store install/reset, generation list/select/retire, database/generation query; dev/test PostgreSQL and pin policies | Disposable database lifecycle; supported commands expose actual availability |

Do not delete unrelated PostgreSQL operational services or protected evidence. No stage may turn a
suspended analysis into a fabricated empty result. Provider integration stays outside the Python
storage wheel. Retirement must not rely on DETACH CONCURRENTLY recovery.

### Phase 2 — Complete attributed facts

| Package | Implementation and owner | Acceptance |
|---|---|---|
| P2.1 | Finish every raw domain family and old-field disposition in model | All existing raw information preserved; no references to future L1 entities |
| P2.2 | Convert `cpg-extract` and `cpg-flow` to typed producer bundles | Provider indices confined locally; preserve pinned acquisition, shared Pyrefly/Ruff parse, ty second parse |
| P2.3 | Single assembly owner and indexed source/span/kind/role join | Scalar oracle agrees; names, attributes, subscripts, nested/same spans and `.py`/`.pyi` controls; ambiguous stays unresolved |
| P2.4 | Canonical atoms/BDD; structured paths; attributed support and coverage | Shuffled-input determinism; unknown/partial/not-requested distinguished; no independent DNF |
| P2.5 | `compile <library> --through facts --profile catalog\|behavioral --database …` | Required provider failure aborts; explicit partial facts disclose limits; actual availability frontier |
| P2.6 | Delete obsolete extraction schemas/hashes/conditions and finally `cpg-schema`; rebuild facts generations | Both profile facts pilots and fixture corpus; no old runtime copy; MCP remains unavailable |

Content equality excludes timestamps and measurements. Fresh recomputation is required; stage cache
optimization is deferred. Once all functional packages are done, run `just fmt`, updated
`just test-all` (including real store and compile-fail tests), facts pilots for both profiles,
`just docs-check`, assembled design review and handoff. Report suspended product/MCP gates separately
as not_run, not as restored product qualification.

### Phase 3 — Normalized relations

Restore entities, normalization, effective callables, signatures, bindings, policy views and program
projections using the same typed model. Independent semantic controls remain required.

### Phase 4 — Analysis and catalog

Restore transfers, obligations, summaries, behavioral evidence, catalog and retrieval. Findings remain
open until assembled consumers demonstrate the intended semantics. A removed implementation does not
retire its capability obligation.

### Phase 5 — Serving

Generate views and inventories from the model; restore native executor/MCP against one leased
published generation. No bundle import or copied serving schema. Qualify end-to-end journeys before
resuming PR6 or new features.

### 4.1 Execution decisions

ADR-0085–0087 supersede the earlier execution decisions. No compatibility, parity-to-bug requirement,
partition migration or upfront ablation is authorized. Independent behavior controls remain required.

### 4.1.1 Detailed remaining execution order

The operator accepted this remaining execution refinement on 2026-09-29, starting from `04a99a6`.
ADR-0085–0087 remain governing; ADR-0088 records the selected chunk-preservation policy. All packages
are one functional implementation scope. Compile/focused checks accompany slices; formatting and the
integrated gate wait for the complete scope. No package is complete merely because it is listed here.

| Sequence | Executable contract and owner | Exit evidence / deletion |
|---|---|---|
| P0-A | Input/source metadata plus canonical 1 MiB ArtifactChunk relationships; complete-content verifier, input manifest, acquisition/distribution/use/corpus boundaries | Empty, non-UTF-8 and >64 MiB readback; corrupt/missing/duplicate/misplaced/noncanonical refusal; source changes during capture abort |
| P0-B | Qualified assertions contain context/condition/modality/approximation; concrete typed supports contain invocation/surface/evidence/origin/mode/fidelity; representative syntax, lexical, symbol/type, call, flow, document and deployment domains | Same proposition may gain supports without strengthening; equal key/conflicting payload refuses; all actual shapes lower through typed/Arrow/PG; no future entity references or opaque relationship JSON |
| P0-C | Port pure call/binding/place/BDD/transfer/control/selection/verdict/derivation policies to nominal types | Full alternative completeness, receiver Unknown, complete binding sets, identity-only path extension, retained opaque guards, stable merge keys, typed premises; C04–C12 controls; later producer/consumer findings remain open |
| P0-D | Validated stage read/write capabilities; one writer, required inputs, cycle checks; fresh catalogs sharing one attempt runtime; exact provider/family/scope matrix and scope/input validation | Undeclared reads/writes and catalog leakage refuse. Complete-empty/Partial/Unavailable/NotRequested remain distinct. Required unexpected failure aborts. Production facts admission requires complete model/schedule/coverage receipts; subset cannot advertise facts |
| P0-D resources | Shared reservations for producer batches, indexes, validator state, COPY, wire conversion and Arrow/typed overlap; model/store use library-neutral budget interface | Preserve 64 GiB default and eight compute partitions with explicit overrides; 4096-row/8 MiB transfer targets, admitted oversized-row reservation; fail without publication; measure parser/runtime overhead separately |
| P0-E | Real pinned extraction → typed source/module/occurrence/assertion/support/coverage → permanent codecs/COPY/sealed validation/readback | Disposable subset conformance harness cannot select or claim complete facts; assembled independent P0 design/target review before scaling; later lifecycle and C04/C13 consumer closure stay open |
| P1-A | Generated install/check/reset and live schema/type/null/key/ref/check/index/grant verification; service-only installation owner | Fresh/repeated install cannot recreate legacy partition/serving schemas. Preserve embedding cache and attempt/event telemetry for current operations consumer; remove snapshot/bundle discovery. Offline transition copies only retained service data |
| P1-B | Lifecycle lock order: installation coordination → selection where relevant → generation → relations. Exact sealed/admission receipts; atomic publish/grants; explicit selection | Atomic abort/retire removes schema and generation registry/receipts/events; selected/leased refuses. AlreadyAbsent only when all owned runtime objects absent; orphan state is inconsistent. Interrupted attempts abort, never repair into publication |
| P1-C | Generation-bound provider sessions and streams, pre-decode digests/admission, lease/pool lifetime, terminal loss and cancellation | Pin survives different selection; no reconnect/substitution; cancel/drain or quarantine. Extend owned provider fork's row-only batching with byte bounds and reservation hooks; update exact pin; no unrestricted discovery/federation |
| P1-D | CLI model/store/generation/query over protected database configuration | Quiesce identified project readers; remove Delta dependencies/persistence/bootstrap, partitions, bundles, adapters, old flags/IDs and runnable downstream closure in Phase 1. Retain only pure raw definitions/producers needed by P2. Compilation explicitly unavailable until complete new producers exist |
| P2-A | Migrate acquisition/source → Ruff syntax/lexical → Pyrefly symbols/signatures/calls/types → ty flow → documents → deployment/receipts | Each group completes §4.1.2 field dispositions, typed bounded producer bundles and independent controls, then deletes old schema/hash authority. Shared Pyrefly/Ruff parse, ty second parse and pinned acquisition retained; no captured code execution |
| P2-B | One assembler attaches by indexed source/span/kind/role, preserves alternatives/support, validates coverage/input ownership, writes typed batches and seals/validates/publishes | Catalog profile records flow NotRequested; behavioral adds flow facts, not analyses. Compile through facts never auto-selects. Beyond-facts requests fail before work. No normalized consensus or fabricated higher-layer output |
| P2-C | Delete cpg-schema and obsolete model/identity/condition authority after final consumers move; remove enumerated obsolete runtime copies | Rebuild both pinned profiles and fixture corpus; input order/transport batching invariance; original bytes retained; independent fixtures/oracles/protected benchmarks/services preserved; surviving downstream obligations carried to P3–P5 |

Operator interface (no legacy aliases): `model describe --format text|json`; `store install|check|reset`;
`generation list|show|select|clear-selection|retire|abort`; `query --generation <id> <SQL>`; and
`compile <library> --through facts --profile catalog|behavioral`. Database commands take
`--database <protected-config-path>` and preserve environment/default config discovery. Query is
read-only. Inspection exposes actual profile/frontier, coverage and contract/content digests.
Embedding/analytics/bundle/old-store arguments leave facts compile. Installation/reset touches only
inventoried project-owned semantic objects and retained service contracts, not unrelated data.

Qualification after all functional packages: `just fmt` once, updated complete `just test-all`
(real PG and compile-fail included), both facts pilots, fixture corpus, `just docs-check`, assembled
Phase 0–2 review and handoff. Suspended analysis/catalog/MCP gates remain explicitly not_run.
Content equality excludes telemetry, timestamps and generation IDs. No incremental cache or new
reasoning engine is added. A capability's deletion does not close its retained obligation.

### 4.1.2 Raw-field migration inventory

**Proposed mappings; source inventory inspected 2026-09-29.** This is the deletion checklist for P2,
not a second executable schema. All 57 `for_each_table!` entries are listed, including inline
macro fields. The destination domain owns all listed payload fields unless an explicit transformation
below applies. No table is marked migrated merely because its owner appears here.

Common transformations: `snapshot_id` becomes generation containment, never a semantic field;
`fact_id` becomes qualified proposition identity plus separate typed supports. Legacy `*_id` node/fact
references are reconstructed as nominal references to the corresponding domain object; no old ID
reader or lookup bridge survives. Optional fields retain unknown/absent meaning through domain sums
or validated options. Display names remain presentation and cannot replace source/provider identity.

| Raw relation | Remaining fields (including legacy identity/reference fields) | Destination owner |
|---|---|---|
| `facts` | `run_id`, `table_name`, `origin`, `extraction_mode`, `modality`, `fidelity`, `model_id` | attribution: qualified assertions and typed support records |
| `runs` | `run_id`, `release_id`, `context_id`, `producer_id`, `families`, `config_digest` | attribution: ProviderRun/RunFamily, AnalysisContext, Provider |
| `contexts` | `context_id`, `python_version`, `python_platform`, `search_path`, `site_package_path`, `config_digest`, `environment_digest`, `lock_digest` | attribution: ProviderRun/RunFamily, AnalysisContext, Provider |
| `producers` | `producer_id`, `tool`, `revision`, `build_digest` | attribution: ProviderRun/RunFamily, AnalysisContext, Provider |
| `releases` | `release_id`, `library`, `requirement`, `lock_digest`, `distributions`, `installer`, `label` | input: InputOrigin/Acquisition, Package/Release, DistributionVerification and InputDistribution |
| `distributions` | `context_id`, `name`, `version`, `artifact_sha256`, `record_digest` | input: InputOrigin/Acquisition, Package/Release, DistributionVerification and InputDistribution |
| `captured_artifacts` | `artifact_id`, `release_id`, `context_id`, `path`, `source_kind`, `source_digest`, `byte_len`, `body`, `alignment`, `provenance`, `observations` | input/source plus deployment: original SourceArtifact bytes and typed interpretation/task observations |
| `source_files` | `module_node_id`, `release_id`, `module_name`, `path`, `is_package`, `is_stub`, `content_digest`, `byte_len`, `utf8`, `distribution`, `role`, `text` | input/source: SourceArtifact, Module, ArtifactOwnership and ArtifactUse |
| `context_modules` | `module_node_id`, `module_name`, `origin`, `path`, `distribution`, `version` | syntax/types: provider-qualified dependency symbols, signatures and ancestry observations |
| `context_definitions` | `symbol_node_id`, `module_node_id`, `module_name`, `kind`, `key`, `name`, `qualified_name`, `is_top_level`, `signature_count` | syntax/types: provider-qualified dependency symbols, signatures and ancestry observations |
| `context_parameters` | `symbol_node_id`, `module_node_id`, `signature_index`, `form`, `ordinal`, `kind`, `name`, `required` | syntax/types: provider-qualified dependency symbols, signatures and ancestry observations |
| `context_class_mro` | `class_node_id`, `module_node_id`, `ordinal`, `ancestor_module`, `ancestor_key`, `ancestor_name`, `cyclic`, `linearization_complete` | syntax/types: provider-qualified dependency symbols, signatures and ancestry observations |
| `declarations` | `node_id`, `module_node_id`, `parent_node_id`, `qualified_name`, `name`, `kind`, `start_byte`, `end_byte`, `name_start_byte`, `name_end_byte`, `docstring`, `docstring_start_byte`, `docstring_end_byte`, `is_overload`, `decorators` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `export_syntax` | `node_id`, `module_node_id`, `kind`, `imported_module`, `imported_name`, `alias`, `level`, `resolved_module`, `start_byte`, `end_byte`, `dunder_all_literal` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `public_names` | `access_path`, `access_module`, `name`, `origin_path`, `origin_module_node_id`, `via_dunder_all`, `origin_module`, `origin_name`, `access_module_node_id`, `origin_symbol_kind` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `parameter_syntax` | `node_id`, `function_node_id`, `ordinal`, `name`, `kind`, `default_text`, `default_start_byte`, `default_end_byte`, `annotation_text`, `start_byte`, `end_byte` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `parameter_docs` | `function_node_id`, `module_node_id`, `name`, `text`, `start_byte`, `end_byte` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `pysa_functions` | `module_node_id`, `module_name`, `function_key`, `name`, `name_start_byte`, `name_end_byte`, `is_overload`, `is_staticmethod`, `is_classmethod`, `is_property_getter`, `is_property_setter`, `is_stub`, `is_def_statement`, `defining_class`, `overridden_base`, `defining_class_module`, `defining_class_key`, `overridden_module`, `overridden_key`, `signature_count` | types: provider-qualified callable signatures, class traits and ancestry |
| `parameter_semantics` | `module_node_id`, `module_name`, `function_key`, `signature_index`, `form`, `ordinal`, `kind`, `name`, `required`, `annotation`, `annotation_classes`, `annotation_classes_exhaustive`, `annotation_scalar` | types: provider-qualified callable signatures, class traits and ancestry |
| `class_ancestry` | `module_node_id`, `module_name`, `class_key`, `class_name`, `name_start_byte`, `name_end_byte`, `relation`, `ordinal`, `ancestor`, `mro_cyclic`, `ancestor_module`, `ancestor_key` | types: provider-qualified callable signatures, class traits and ancestry |
| `pysa_classes` | `module_node_id`, `module_name`, `class_key`, `class_name`, `name_start_byte`, `name_end_byte`, `is_synthesized`, `is_dataclass`, `is_named_tuple`, `is_typed_dict` | types: provider-qualified callable signatures, class traits and ancestry |
| `call_syntax` | `node_id`, `module_node_id`, `owner_node_id`, `start_byte`, `end_byte`, `callee_start_byte`, `callee_end_byte`, `in_annotation`, `positional_count`, `keyword_count` | calls: syntax sites/arguments and separately qualified target alternatives |
| `arguments` | `node_id`, `call_node_id`, `ordinal`, `kind`, `keyword`, `start_byte`, `end_byte`, `value_start_byte`, `value_end_byte` | calls: syntax sites/arguments and separately qualified target alternatives |
| `syntax_nodes` | `node_id`, `module_node_id`, `owner_node_id`, `parent_node_id`, `kind`, `field`, `ordinal`, `start_byte`, `end_byte`, `detail` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `scopes` | `node_id`, `module_node_id`, `kind`, `owner_node_id`, `parent_scope_id`, `start_byte`, `end_byte` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `bindings` | `node_id`, `scope_id`, `module_node_id`, `name`, `kind`, `ordinal`, `site_node_id`, `start_byte`, `end_byte`, `value_start_byte`, `value_end_byte`, `static_branch`, `static_polarity` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `references` | `node_id`, `name_node_id`, `scope_id`, `module_node_id`, `name`, `parent_node_id`, `field`, `start_byte`, `end_byte` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `reference_resolutions` | `reference_id`, `binding_id`, `captured`, `builtin_name`, `reason` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `type_terms` | `node_id`, `kind`, `display`, `detail`, `literal_json`, `class_module`, `class_key`, `variable`, `anchor_module`, `anchor_start`, `anchor_end` | types: structural terms/arguments, qualified type/body/record-field observations |
| `type_term_args` | `parent_node_id`, `role`, `ordinal`, `child_node_id`, `name`, `parameter_kind`, `required` | types: structural terms/arguments, qualified type/body/record-field observations |
| `type_observations` | `module_node_id`, `subject_node_id`, `role`, `declared`, `term_node_id` | types: structural terms/arguments, qualified type/body/record-field observations |
| `function_implementations` | `function_node_id`, `module_node_id`, `body_kind`, `is_abstract_method`, `is_in_protocol_class`, `is_in_type_checking_block`, `is_overload` | types: structural terms/arguments, qualified type/body/record-field observations |
| `record_fields` | `node_id`, `class_node_id`, `module_node_id`, `record_kind`, `name`, `ordinal`, `term_node_id`, `declared`, `start_byte`, `end_byte`, `has_default`, `init`, `alias`, `kw_only`, `required`, `read_only` | types: structural terms/arguments, qualified type/body/record-field observations |
| `record_field_syntax` | `field_node_id`, `class_node_id`, `module_node_id`, `name`, `start_byte`, `end_byte`, `annotation_text`, `value_text`, `value_start_byte`, `value_end_byte` | types: structural terms/arguments, qualified type/body/record-field observations |
| `documents` | `node_id`, `release_id`, `path`, `content_digest`, `byte_len`, `title`, `parsed` | documents: source-backed document/section/code/link/mention/component records |
| `passages` | `node_id`, `document_node_id`, `ordinal`, `level`, `heading`, `heading_path`, `start_byte`, `end_byte`, `text` | documents: source-backed document/section/code/link/mention/component records |
| `code_blocks` | `node_id`, `document_node_id`, `passage_node_id`, `ordinal`, `language`, `meta`, `start_byte`, `end_byte`, `code`, `content_digest`, `module_path` | documents: source-backed document/section/code/link/mention/component records |
| `doc_links` | `passage_node_id`, `ordinal`, `url`, `title`, `text`, `start_byte`, `end_byte` | documents: source-backed document/section/code/link/mention/component records |
| `mentions` | `passage_node_id`, `class`, `source`, `form`, `access_path`, `qualified_name`, `start_byte`, `end_byte` | documents: source-backed document/section/code/link/mention/component records |
| `doc_components` | `document_node_id`, `passage_node_id`, `ordinal`, `parent_ordinal`, `depth`, `name`, `form`, `start_byte`, `end_byte`, `inner_start`, `inner_end`, `lead_start`, `lead_end` | documents: source-backed document/section/code/link/mention/component records |
| `doc_component_attributes` | `document_node_id`, `component_ordinal`, `ordinal`, `name`, `value`, `value_kind` | documents: source-backed document/section/code/link/mention/component records |
| `pysa_calls` | `payload_id`, `module_node_id`, `module_name`, `caller_key`, `site_kind`, `callee_kind`, `site_detail`, `start_byte`, `end_byte`, `phase`, `higher_order_index`, `target_kind`, `target_module`, `target_key`, `target_name`, `receiver_class`, `receiver_module`, `receiver_key`, `implicit_receiver`, `implicit_dunder_call`, `is_class_method`, `is_static_method`, `unresolved_reason`, `is_attribute` | calls: syntax sites/arguments and separately qualified target alternatives |
| `flow_uses` | `use_id`, `module_node_id`, `place`, `scope_kind`, `scope_start_byte`, `scope_end_byte`, `start_byte`, `end_byte`, `annotation` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_definitions` | `definition_id`, `module_node_id`, `place`, `kind`, `scope_kind`, `scope_start_byte`, `scope_end_byte`, `start_byte`, `end_byte`, `value_start_byte`, `value_end_byte` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_reaching` | `use_id`, `definition_id`, `condition_id`, `approximated`, `loop_carried` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_values` | `module_node_id`, `sink`, `sink_start_byte`, `sink_end_byte`, `use_id`, `identity`, `through_call`, `condition_id`, `approximated` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_value_calls` | `module_node_id`, `flow_value_fact_id`, `use_id`, `step`, `call_start_byte`, `call_end_byte`, `operand_start_byte`, `operand_end_byte`, `role` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_regions` | `module_node_id`, `scope_kind`, `scope_start_byte`, `scope_end_byte`, `start_byte`, `end_byte`, `condition_id`, `approximated` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_tests` | `module_node_id`, `scope_kind`, `scope_start_byte`, `scope_end_byte`, `start_byte`, `end_byte`, `condition_id` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_test_leaves` | `module_node_id`, `scope_kind`, `scope_start_byte`, `scope_end_byte`, `predicate_key`, `test_start_byte`, `test_end_byte`, `condition_id`, `atom_id`, `atom`, `leaf_start_byte`, `leaf_end_byte` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_test_types` | `module_node_id`, `leaf_fact_id`, `atom_id`, `use_id`, `use_fact_id`, `operand_start_byte`, `operand_end_byte`, `role`, `place`, `term_node_id`, `term_fact_id`, `origin` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_attribute_loads` | `module_node_id`, `start_byte`, `end_byte`, `name` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `conditions` | `condition_id`, `root_id`, `encoding`, `stated`, `display_truncated`, `boundary_reason` | conditions: canonical BDD/occurrence atoms; display-only projections retired |
| `condition_nodes` | `node_id`, `atom`, `low_id`, `high_id` | conditions: canonical BDD/occurrence atoms; display-only projections retired |
| `condition_literals` | `condition_id`, `conjunction`, `ordinal`, `atom`, `positive`, `place`, `argument` | conditions: canonical BDD/occurrence atoms; display-only projections retired |
| `coverage` | `run_id`, `scope_kind`, `scope_node_id`, `fact_family`, `status`, `reason`, `detail` | attribution: ProviderCoverage plus structured scope/subject boundaries |
| `boundaries` | `module_node_id`, `subject_node_id`, `fact_family`, `reason`, `start_byte`, `end_byte`, `detail` | attribution: ProviderCoverage plus structured scope/subject boundaries |

Specific transformations and deletion conditions:

- `facts.table_name` is replaced by concrete typed support targets. `origin`, `extraction_mode`,
  `fidelity` and provider surface remain evidence attributes; modality/condition/approximation qualify
  propositions and participate in their identity. Run identity does not qualify proposition identity.
- Acquired distribution strings split into package/version references. Source text, UTF-8, package
  and stub flags derive from retained bytes/path; original bytes must survive failed interpretation.
  Context distribution association must remain explicit, including unowned top-level input entries.
- Rendered flow `place`, predicate keys and string atoms are replaced by structural places and
  occurrence-keyed atoms. Keep original diagnostic text only as presentation. `conditions.encoding`
  and `condition_literals` have no independent semantic store: canonical BDD roots/nodes own truth;
  terminal truth, unknown/refusal and approximation must remain distinguishable.
- Calls retain direct/higher-order distinction, phase, target/receiver alternatives, unknown receiver
  and unresolved remainder. Target absence never certifies completeness. Span attachment retains
  ambiguity and provider identity; normalized entity ownership waits until L1.
- Documents/code retain byte-coordinate spaces and links to original artifacts. Optional parser
  failure cannot erase evidence. Annotation/default/literal payloads require lossless typed values;
  a display string alone does not establish structural semantics.
- `snapshots` is outside the 57 raw families: generation registry/content and relation receipts replace
  its publication metadata. Delta table versions and snapshot-key columns are deleted at cutover.

Nested captured evidence is part of the same inventory, not an opaque JSON escape hatch:

| Current structure | Fields retained in typed deployment/evidence records |
|---|---|
| `Checks` | `parse`, `binding`, `environment`, `execution` |
| `ContextRequirement` | `kind`, `expression`, `evidence` |
| `ScenarioDetail` | `spans`, `context`, `intent`, `checks`, `requirements`, `extraction`, `analysis_module`, `option_bindings`, `omitted_options`, `omitted_requirements` |
| `OptionBinding` | `site_id`, `ordinal`, `keyword`, `kind`, `expression`, `span_id`, `coordinate_space` |
| `DeploymentEnvironment` | `release_id`, `lock_digest`, `environment_digest`, `runtime_digest`, `interpreter_digest`, `python_version`, `platform`, `requirement`, `metadata` |
| `TaskReceipt` | `format`, `policy`, `task`, `runner_sha256`, `source_path`, `source_sha256`, `environment`, `command`, `tool`, `arguments`, `elapsed_ms`, `timeout_seconds`, `execution`, `tools`, `result`, `diagnostic` |
| `TaskObservation` | `target_artifact`, `receipt` |
| `AssociationSupport` | `edge_id`, `fact_id`, `support_fact_id`, `target_id`, `modality`, `phase`, `unresolved_reason`, `context_span_id`, `analysis_module`, `start_byte`, `end_byte`, `coordinate_space` |
| `DeploymentDetail` | `distribution`, `version`, `field`, `original`, `name`, `extras`, `marker`, `constraint`, `interpretation`, `diagnostic`, `environment_digest`, `lock_digest`, `task`, `referenced_path` |

`CheckStatus`, `ContextStatus`, `Intent`, `Alignment`, `EvidenceKind` and `EvidenceRef` become
typed codes/sums. Metadata maps, task arguments/commands/tools, requirements, option bindings and
evidence-reference collections become ordered/keyed relationship records. Preserve receipt format,
policy, runner/source hashes, execution result/diagnostic and omission counters. Scenario/association
conclusions are P4 work, but their raw evidence inputs cannot be dropped during P2. Captured
`observations` must be decoded and attributed by the new producer, without retaining an old-format
runtime reader. Malformed interpretation retains the original artifact with explicit failure coverage.

### 4.2 Execution status

**In progress: P0-A–P0-D foundations, native P0-E subset and bounded generation lifecycle corrections.**
No phase exit is qualified. The old pipeline remains active and the new generation path is not
connected to the production compiler or CLI. No compatibility adapter connects them.

**Implemented / Tested (2026-09-29, bounded):**

- Ordinary record/codebook/sum/Assertion derives, nominal IDs, generated keys and explicit Arrow
  codecs. Model admission enforces references, subtype membership and assertion/support companions;
  assertion qualification must participate in identity.
- Source metadata plus canonical 1 MiB artifact chunks preserve original binary evidence, including
  files above the row limit. Capture and sealed validation prove full length/digest and canonical
  ordering; failed capture cannot resume successfully. The extractor now freezes a complete selected
  input inventory before provider startup, reserves copy/chunk/metadata buffers, detects changed input
  and verifies the frozen bytes afterward. Its caller must quiesce acquisition; this is not an atomic
  filesystem snapshot or total-RSS qualification. Input manifests, acquisition/distribution/
  corpus relationships, ownership/use and source spans have shared stored invariants.
- Typed values, literal sets, places, occurrence-keyed atoms and canonical BDD nodes. Independent
  truth tables and malformed-node controls cover bounded operations and persisted hydration.
  Capture-safe substitution preserves unmapped opaque guards; display truncation is separate.
- Qualified syntax propositions and generated concrete supports retain context, condition, modality,
  approximation, invocation, surface, evidence, origin, mode and fidelity. Validators check condition
  contexts, evaluation/source input ownership, assertion/evidence scope and required support.
  Coverage-matrix controls exist; full compiler schedule/coverage admission remains open.
- A pinned Pyrefly retained-AST emitter produces structural occurrences and qualified identifier
  observations directly, without legacy rows/IDs. A disposable conformance harness exercises
  permanent PG lowerings and readback, relocation, changed-text refusal and incomplete coverage.
  The harness now uses the reusable captured tree. Emission/depth limits are not total traversal-work
  limits; acquisition/assembler integration remains open.
- Provider-qualified symbols, complete signature membership, typed call destinations/channels/receivers
  and complete alternative membership have generated lowerings and stored invariants. Native support
  must belong to the symbol provider. The whole-variant binder preserves all formals, defaults, empty
  aggregates, binding kind and positional/keyword projections; forged shape lookups refuse. Named
  policies exclude potential/higher-order invocation from dataflow and use the full declared set for
  uniqueness. P3 equivalence, constructor normalization and SQL projection remain open.
- Stable TransferKey vocabulary, qualified alternatives and condition-OR aggregation retain original
  evidence IDs. ControlInfluence and Selection remain distinct from value flow. Shared support checks
  both Place endpoints and populated call sites against scope/acquisition. Structural path composition
  extends only through identity; wildcard/unknown/numeric-alias cases preserve uncertainty. Whole-call
  root/binding composition and opaque caller rebasing remain open.
- Derivation fields declare their rule, nominal conclusion and premise roles in their Rust record.
  Model admission generates one cross-source cycle invariant; proof source identity survives explicit
  conclusions. Existing pinned petgraph supplies iterative cycle detection. PostgreSQL generates
  explanation views and publication grants from the same declarations. Self/mutual proof-step cycles
  refuse before publication. P4 producers and serving explanations remain open.
- Attachment indexes and retained ambiguous answers own shared-budget reservations, with preallocation
  admission and failure cleanup. A flat sorted index retains scalar-oracle equivalence. This qualifies
  attachment buffers only, not parser/validator/COPY totals or process RSS.
- Typed stage access refuses undeclared reads/writes and incomplete, failed or cancelled execution.
  `cpg-core::model_runtime` constructs fresh catalogs over a shared DataFusion runtime, rejects
  foreign stage/attempt read permits and SQL mutations. A neutral reservation interface shares the
  compute pool; accounting every producer/index/validator/codec buffer remains open.
- The pure verdict policy treats scope boundaries and approximations as Unknown, preserves explicit
  NotRequested, requires complete exact coverage for refutation, and ignores display truncation.
- PostgreSQL generation lowering, streamed COPY/read validation, writer-draining sealing, exact
  relation/validator receipts and pool-bound reader leases. Subset creation is explicitly
  `create_conformance`; it cannot select a production facts generation. Cleanup removes schema,
  registry, receipts and generation events atomically. Retry returns AlreadyAbsent only for complete
  absence; orphaned schema/registry state requires explicit repair. Production facts admission is
  not implemented.

Focused evidence (2026-09-29; Cargo commands prefixed by `python3 scripts/build_environment.py --`):

| Command | Outcome and boundary |
|---|---|
| `cargo test --release -p lctx-model --test domain --test domain_conditions --test domain_assertions --test domain_stages --test domain_resources --test domain_verdicts` | passed: 29 focused model controls |
| `cargo test --release -p lctx-model --test domain_transfer --test domain_assertions --test domain_paths` | passed: five transfer, three assertion and two path controls; independent reviewer rerun |
| `cargo test --release -p lctx-model --test domain --test domain_resources` | passed: 17 domain and three resource controls; independent reviewer rerun |
| `cargo test --release -p lctx-model --test domain_derivation`; `cargo test --release -p lctx-postgres --test domain_derivation` | passed: three model controls and one real PG18 four-case test; independent reviewer rerun after proof-source correction |
| `cargo test --release -p lctx-postgres --test domain_transfer` | passed: real PG18 transfer/control/selection/support readback, generated view targets and cross-scope call-site refusal |
| `just build-features` | passed: CLI union regenerated; lower model macro crate excluded; no dependency pin upgraded |
| `cargo test --release -p lctx-model --test domain_calls` | passed: seven call/signature/binding controls, independently rerun after review corrections |
| `cargo test --release -p lctx-postgres --test domain_calls` | passed: real PG18 signature/call/support readback and wrong-provider refusal; contract fixtures, not producer qualification |
| `cargo test --release -p cpg-extract --test capture`; `cargo test --release -p cpg-extract --lib capture::tests` | passed: three capture controls plus changed-during-capture unit; 65 MiB input under a 3 MiB capture reservation budget, not RSS |
| `cargo test --release -p lctx-model --doc` | passed: seven negative/positive declaration pairs including nominal proof targets; one pre-existing ignored legacy example |
| `cargo test --release -p lctx-postgres --test generations` | passed: two real disposable PG18 tests, including 65 MiB evidence, stored corruption and contract refusal, write drain, leases, conformance selection refusal, cleanup retry and orphan repair |
| `cargo test --release -p cpg-extract --test typed_conformance` | passed: real pinned AST to typed records, sealed PG18 validation/readback and conformance-only selection refusal |
| `cargo test --release -p cpg-core --test model_runtime` | passed: two fresh-catalog/capability/shared-pool controls; no total-memory/RSS claim |
| `just docs-check`; `uv run python scripts/adr.py lint` | passed: documentation publication and 44 current ADR records |
| `just fmt`, `just test-all`, facts pilots | not_run: functional scope incomplete |

Independent bounded reviewers accepted artifact/capture/acquisition corrections and the
[assertion/condition foundation](../design_review/reviews/design_review_semantic-assertion-foundation_2026-09-29.md)
after its three corrections. The [native syntax subset review](../design_review/reviews/design_review_semantic-native-syntax_2026-09-29.md)
accepted the bounded producer/store seam without qualifying P0. The
[call contract review](../design_review/reviews/design_review_semantic-call-contract_2026-09-29.md) accepted
three corrected findings, and the [input capture review](../design_review/reviews/design_review_semantic-input-capture_2026-09-29.md)
accepted the checked-copy and bounded reservation contract. The [earlier input review](../design_review/reviews/design_review_semantic-input-validation_2026-09-29.md)
still has open coordinated-resource qualification. The
[transfer/path review](../design_review/reviews/design_review_semantic-transfer-contract_2026-09-29.md)
accepted its scope after call-site ownership correction; the
[attachment resource review](../design_review/reviews/design_review_semantic-attachment-resources_2026-09-29.md)
accepted buffer accounting; the [nominal derivation review](../design_review/reviews/design_review_semantic-nominal-derivation_2026-09-29.md)
accepted typed targets, cycle validation and PG projections after proof-source identity correction.
Stage/session/lifecycle changes await their
assembled review; none of these bounded receipts establishes enclosing architecture acceptance.

**Next / still open:** remaining representative lexical/type/flow/document/deployment domain families
and whole-call binding/root composition and opaque caller rebasing; complete call producer/normalization integration; occurrence ownership; coordinated allocation accounting; exact coverage
and full production frontier admission; production capture/assembler integration and assembled P0-E review.
Then finish generated installation verification/reset, provider lease/stream integration, service
classification, store/CLI cutover and Delta deletion in P1; migrate every raw producer/field in §4.1.2
and reconstruct owned state in P2. Earlier Phase 0 receipts do not qualify this target.

## 5. Deletion and preservation obligations

Delete legacy code by ownership boundary after preserving raw-field mappings and independent controls.
No legacy adapter inventory is maintained. Preserve fixtures, oracles, protected benchmarks, evaluation
isolation and unrelated operational services. Git retains removed implementation. Old runtime readers
must be quiesced before replacing or retiring project state.

## 6. Deferred capability obligations

Phase 4 retains finite summaries/discharge, completion/evaluation, frame exits and call execution,
context protocols/values, modeled identities/actions, behavioral reachability, communities/PageRank,
FCA/RCA and optional kNN wherever their existing consumer remains. Retirement needs consumer evidence
and an explicit decision; deleting old code during reconstruction does not retire these obligations.

## 7. Tooling, pins, skills and documents

Phase 0 adds the bounded derive and explicit Arrow codec dependencies. Phase 1 removes Delta family
and skill selection and replaces store/CLI/PG test commands. Phase 2 updates fact owners and tests.
Model, PG and producer digests, generated declarations and SQLx metadata must be regenerated from their
owners. Phases 3–5 update their owners when implemented. No documentation labels target as Tested
without corresponding evidence.

## 8. Findings disposition

The bounded [input-validation review](../design_review/reviews/design_review_semantic-input-validation_2026-09-29.md)
has the following separate finding namespace (2026-09-29):

| Source finding | Current disposition | Owner and closure evidence |
|---|---|---|
| input-validation F01/F03/F04/F05 | addressed within reviewed slice | `lctx-model` input/source invariants; real PG manifest/span/cross-input ownership refusals and multi-distribution positive; reviewer source reinspection accepted |
| input-validation F02 | open, P0.3/P1 | model/store/producer coordinated memory accounting including conversion copies and invariant state; large-input measured envelope and refusal controls required; server row admission and per-row COPY alone are partial |


This table owns the current disposition of the review's findings. Each closes by construction in the
phase named, and only on its closure evidence.

| Review finding | Disposition | Responsible WP | Closure evidence |
|---|---|---|---|
| [F01](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F01) unknown default served as absent | open → phase 4 | 4.5 | tri-state evaluation; provider-constructor and factory fixtures `Unresolved` |
| [F02](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F02) call relation has no owner; origin hidden | open → phases 3–4 | 3.1, 3.5, 4.5 | five policy views; S1 as one policy edit; decorator-registration fixture shows its origin |
| [F03](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F03) association basis dropped | open → phase 4 | 4.5 | basis and origin required columns; candidate-only fixtures `Unresolved` |
| [F04](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F04) no transfer algebra; identity served as computed | open → phase 4 | 0.4, 4.2 | composition table; P0 `facade` renders `unchanged` |
| [F05](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F05) two condition representations | open → phases 2, 4, 5 | 2.2, 4.1, 5.2 | occurrence-keyed atoms; no parallel DNF; served `condition_id`; ty-upgrade identity control |
| [F06](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F06) no place/transfer vocabulary | open → phases 3–4 | 0.4, 3.1, 4.3, 4.5 | P0 rerun: plain-class option→field→reader and per-branch supply |
| [F07](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F07) distributed refusal/obligation; silent cost cap | open → phases 0, 4 | 0.4, 0.6, 4.3, 4.5 | one obligation owner; cost-cap control; budget reason; memory pool |
| [F08](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F08) no derived-result contract; lineage gaps | open → phase 4 | 4.6, 4.7 | one emitter; input invocations; finding-ID rule; derivation views |
| [F09](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F09) nominal projection layer | open → phases 3–4 | 3.2, 4.6 | projection by declaration; ADR-0044 amendment (made 2026-09-29) |
| [F10](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F10) serving is a second hand authority | open → phases 0, 5 | 0.2, 5.1, 5.5 | generated DDL, views and inventories; no hand serving file |
| [F11](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F11) implicit stage composition; producer identity mislabelled | open → phases 0–1 | 0.7, 1.2 | stage-table refusal controls; producer identity covers every canonical producer; the pipeline schedules from the table |
| [F12](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F12) per-request rebuilds and round trips | open → phase 5 | 5.1 | generation-scoped prepared catalog; set-based hydration |
| [F13](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F13) pilot recognizer; lexical parameters outside the digest | open → phases 4–5 | 4.4, 5.4 | named authored model; policy digest |
| Review observations: browse unknown ownership, `EmptyUnderCoverage`, lexical-only reason | open → phase 5 | 5.1 | disclosed in responses |

**Operator decision (2026-09-29).** Do not repair these in the legacy code. The new contracts must
make them unrepresentable.


### Assertion foundation review findings

The [bounded assertion review](../design_review/reviews/design_review_semantic-assertion-foundation_2026-09-29.md)
reinspected these corrections on 2026-09-29. Closure is limited to the foundation; P0-E assembly stays open.

| Finding | Responsible component | Current disposition and evidence |
|---|---|---|
| [F01](../design_review/reviews/design_review_semantic-assertion-foundation_2026-09-29.md#F01) | `lctx-model::domain::assertion` | closed (bounded): condition evaluation input/scope validation; same-input/corpus positive and foreign/missing/out-of-scope negative controls |
| [F02](../design_review/reviews/design_review_semantic-assertion-foundation_2026-09-29.md#F02) | `lctx-model-macros` | closed (bounded): keyed qualification required by derive; paired declaration controls |
| [F03](../design_review/reviews/design_review_semantic-assertion-foundation_2026-09-29.md#F03) | `lctx-model::domain::model` | closed (bounded): generated companion dependency; incomplete membership rejected |


### Bounded native call contract findings

The [call review](../design_review/reviews/design_review_semantic-call-contract_2026-09-29.md) independently
reinspected corrections and reran seven controls on 2026-09-29. P2/P3 and C04 closure remain open.

| Finding | Responsible component | Current disposition and evidence |
|---|---|---|
| [F01](../design_review/reviews/design_review_semantic-call-contract_2026-09-29.md#F01) | `lctx-model::domain::calls` | closed (bounded): native provider/support ownership invariant; signature/call positive and crossed-provider controls plus real PG18 refusal |
| [F02](../design_review/reviews/design_review_semantic-call-contract_2026-09-29.md#F02) | `lctx-model::domain::calls` | closed (bounded): shape lookup identity checked before binding; forged optional-for-required map refuses |
| [F03](../design_review/reviews/design_review_semantic-call-contract_2026-09-29.md#F03) | `lctx-model::domain::calls` | closed (bounded): binding kind and aggregate projections; keyword keys and receiver/varargs/implicit positions retained |

### Transfer and derivation review findings

| Finding | Responsible component | Current disposition and evidence |
|---|---|---|
| [transfer-contract F01](../design_review/reviews/design_review_semantic-transfer-contract_2026-09-29.md#F01) | `lctx-model::domain::assertion` | closed (bounded): populated call site shares endpoint scope/acquisition checks; local/corpus positives and unrelated/artifact-scope refusals, real PG18 publication refusal, independent reinspection |
| [nominal-derivation F01](../design_review/reviews/design_review_semantic-nominal-derivation_2026-09-29.md#F01) | model derivation/derive/common invariant | closed (bounded): retain source step and conditional conclusion-to-source edge; self and mutually dependent explicit steps refuse; model and real PG18 independently rerun |

### Core review findings

All findings from the [core review](../design_review/reviews/design_review_cutover-core_2026-09-29.md)
remain open; acceptance requires implementation plus focused evidence and the P0.6 review.

| Finding | Owner and closure |
|---|---|
| C01 | P1.2/P1.3: schema-per-generation retirement, reader lease and select/retire concurrency controls |
| C02 | P1.3: reader rejects model/physical digest mismatch before decoding |
| C03 | P0.5: typed model membership and sole stage writer authority, including self-cycle refusal |
| C04 | P0.4/P2/P3: attributed direct/potential/higher-order alternatives; one normalization owner |
| C05 | P0.4: paths compose only through identity; map complete binding sets and boundary outputs |
| C06 | P0.4: preserve opaque local guards; refuse unsupported substitution rather than erase conditions |
| C07 | P0.4: typed ControlInfluence and Selection with separate value-transfer meaning |
| C08 | P0.4: transfer key excludes merged condition; stable derivation references |
| C09 | P0.4: scope-boundary unknown; approximation obligation; rendering budget separate |
| C10 | P0.4: receiver Unknown and one classification policy |
| C11 | P0.2/P2: acyclic source identities, typed syntax kinds and structural occurrence discriminators |
| C12 | P0.4/P2.3: indexed region join with scalar oracle and unresolved ambiguity |
| C13 | P0.3/P3–P5: derivation targets follow typed references and generated view grants |
| C14 | P0.1: DESIGN §15.1 assigns provider registration to cpg-core and PostgreSQL effects to lctx-postgres |

## 9. Risks

The principal risks are incomplete raw-family migration, parallel semantic definitions, invalid
publication races and accidental claims of restored downstream capability. The field mapping, single
root model, real concurrency tests and generation availability frontier address these directly.

## 10. Deferred, each with a trigger

Stage reuse requires the existing measured-workload trigger. New reasoning engines and application
Salsa require a concrete consumer. PostgreSQL performance tuning follows measured phase-exit costs.
Cross-release symbol matching waits for its consumer. Product PR6 waits for phase 5 qualification.
