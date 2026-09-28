# Library Context: targeted product recommendations after PostgreSQL rollout

**Review baseline:** `739f9dfbfa2fb3acb52574e8bf894599b5d34af7`, September 28, 2026. **Status:** proposed recommendations for agent assessment, not implemented changes. **Method:** read-only examination of current repository source, contracts, status, and selected reports. No compiler/test execution, connection to the operator database, or head-to-head Context7 evaluation was performed. Deployment results below are repository-reported receipts, not independently repeated measurements. [S01] [S02]

## Decision

Keep the completed infrastructure and make the next increment an **API-and-evidence product**, not another expansion of the general Python analysis engine. The differentiating deliverable should be: find the appropriate public API, show its actual invocation/configuration surface, supply contextualized examples, and expose the evidence and uncertainty behind the guidance.

The PostgreSQL rollout materially changes the implementation route, but not the product priorities. The repository now records generation-pinned native serving, COPY publication, recovery, exact vector retrieval, and 1,024-dimensional embeddings. ANN was not admitted for the measured deployment; exact retrieval remains selected. These are completed capabilities to reuse, not tasks to repeat. Semantic Stage 3 remains separately incomplete. [S02]

The recommendations intentionally address the earlier bounded-product proposal. They do not ask the agent to complete general decorator interpretation, all-channel transfer proofs, or a comprehensive ontology before releasing a useful tool.

## Assumptions and retained boundaries

The immediate target is a supported set of pinned Python libraries, initially the existing pilot, used by capable programming agents. Such agents can interpret original source and documentation when the system provides accurate identities, context, and provenance. The product need not convert every useful observation into a fully established runtime claim.

Retain canonical Delta fact authority, PostgreSQL's derived serving projection and operational roles, the existing Rust semantic owner, and Python/native transport separation. Do not introduce a second canonical graph or a second implementation of condition semantics in SQL or Python. The runbook and projection contracts already support these boundaries. [S04] [S11]

Distinguish **valid evidence**, **successful retrieval**, and **established behavior**. None is a substitute for the others. In particular, a behavioral unknown must not become a positive assertion, but it also need not erase a valid signature, source excerpt, documented use, or observed example.

## Recommendation index

| ID | Recommended change | Main kind of work |
|---|---|---|
| R01 | Make evidence-catalog usability independent of optional briefs and deep-analysis completion | Release/publication policy |
| R02 | Expose complete API contracts and assemble bounded implementation packets | Product projections and hydration |
| R03 | Represent decorator effects on the public surface separately from behavioral admission | Structured normalization and selective models |
| R04 | Promote options, configuration fields, and a few useful relationships into queryable records | Derived facts and bounded analysis |
| R05 | Make documentation and contextualized usage scenarios available across the public catalog | Evidence association and extraction |
| R06 | Retrieve evidence units and retain why each result matched | Retrieval contracts and views |
| R07 | Separate strict matches from exploratory candidates and joint applicability | Query semantics |
| R08 | Qualify the product on discovery and implementation tasks, separately from semantic research | Evaluation and scope control |

## R01 — Make evidence-catalog usability independent of optional briefs and deep-analysis completion

**Observed basis.** The rollout is qualified for local exact serving, yet the current status still records an analysis-free, undocumented seed failing `semantic:documentation-only-has-outcome`. It also explicitly separates PostgreSQL completion from incomplete semantic Stage 3. This leaves an unnecessary coupling between catalog usefulness and the availability of a valid brief or stronger analysis. [S02]

**Recommendation and rationale.** Make publication of the supported API/evidence catalog require valid identities, internally consistent product relations, correctly scoped evidence, and supported query behavior. Do not require every public operation to receive a brief or every behavioral channel to be complete. An API with no supported outcome sentence should publish as an API record without a brief, not as a malformed documentation-only brief.

This does not mean bypassing validators. Corrupt references, cross-generation evidence, or unsupported positive claims must still prevent the affected content from being published. The distinction is between an absent optional artifact and invalid mandatory evidence. Existing defects that can produce false claims in an exposed query path remain release blockers. Similarly, an analyzer that explicitly reports unsupported semantics is different from an unhandled compiler failure; do not turn arbitrary errors into success.

The useful release can state “the interface and usage evidence are available; this behavioral question is unresolved.” That is the desired boundary for an implementable product. Preserve the deeper suite and its unmet criteria rather than redefining those tests as passed.

**Likely ownership.** Existing synthesis/brief selection and publication policy, the current execution plan, and operation/evidence readiness reporting. This should not become a parallel publication framework.

**Auditable outcome.** A fixture with a valid public function, no docstring, and unresolved decoration can be published, searched, and inspected without a brief. The underlying body's claims are not promoted to exported-callable behavior. A separate fixture with a broken evidence reference still fails validation. No lowered behavioral verdict threshold is needed.

## R02 — Expose complete API contracts and assemble bounded implementation packets

**Observed basis.** Native `get_operation` now owns PostgreSQL hydration, but still derives parameter entries from facets and attaches fates to them. The response is not a complete structured call contract. Class handling searches for a public `__init__` record. Meanwhile, the canonical facts already preserve ordered parameter syntax, defaults, requiredness, annotations, parameter documentation, overload associations, type structure, and synthesized constructor information. [S05] [S06] [S07]

**Recommendation and rationale.** Add a coherent public API contract projection. It should preserve ordered signature variants, parameter kinds, requiredness, literal/default-expression distinction, parameter documentation, return information, public spellings, defining/inherited origin, and invocation form. Do not reconstruct these fields from strings in facets or re-extract facts already available.

Keep the exposed member distinct from the implementation declaration it references. A property is a public member with getter/setter/deleter associations, not whichever accessor happened to own the path. A class may have a provider-described synthesized constructor even when no public source `__init__` exists. Preserve whether a signature is source-declared, analyzer-inferred, documented, or runtime-observed; a custom decorator can leave the effective exported signature unresolved while the source declaration remains known.

Assemble an implementation packet from these relations: preferred import/access path, call or access form, signature, relevant options, one or two contextualized examples, selected source/helper links, and scoped limitations. It is a deterministic response projection, not a new authoritative prose summary. Support a bounded default packet and expandable sections rather than returning the entire behavior graph. Add module/class browsing and facet counts over the same catalog for exploration.

**Likely ownership.** Canonical signature/type/public relations; `cpg-schema` serving schemas and projection contracts; existing compiler-side projection builders; `lctx-postgres::hydration`; existing MCP response contracts. Keep semantic decisions in Rust and use PostgreSQL to select and assemble established records.

**Auditable outcome.** Round-trip fixtures through canonical publication, PostgreSQL import, and MCP with positional-only/keyword-only parameters, `*args`/`**kwargs`, a default expression, overloads, inheritance, a property with multiple accessors, and a synthesized constructor. Parameter order and provenance survive. The packet supplies valid invocation guidance where supported and explicit uncertainty otherwise. Merely formatting a default never executes it.

## R03 — Separate decorator-aware surface descriptions from behavioral admission

**Observed basis.** The current `decorated_functions` classifier has a deliberate narrow exemption: one bare-name built-in `classmethod`, `staticmethod`, or `property`, supported by lexical resolution and matching provider flags. Multiple decorators and other forms remain withheld. This protects behavioral claims but does not provide a complete public-surface description. [S08]

**Recommendation and rationale.** Normalize decorator applications into an ordered representation: target declaration, resolved decorator identity or candidates, source/application order, arguments, transformation category, evidence, and unresolved remainder. Consume existing syntax and resolution facts rather than building another parser or general decorator interpreter.

Treat surface transformations as specific claims, not one “transparent” Boolean. A model can establish receiver binding or property access without certifying unchanged effects, exception behavior, or lifecycle. Extend the current supported built-in forms to selected imported spellings and valid stacks, then cover the pilot's frequent protocol-changing and registration patterns. Model only the supported composition and retain the ordering constraints.

Record wrapper associations such as `__wrapped__`, but do not use `functools.wraps` as a general exemption from behavioral withholding. Python documents metadata copying and the wrapper link; it does not make a wrapper equivalent to the wrapped body. For unknown decorators, retain the original declaration, decorator expression, documentation, and usage examples while marking the exported-callable transformation unresolved. [S19]

The existing shared decorator classifier should remain the owner of its behavioral admission decision. Surface-description results may supply more granular evidence to it, but a new PostgreSQL-side or presentation-side whitelist would recreate divergent semantics.

**Likely ownership.** Existing decorator syntax/provider mapping, a small typed surface-transformation relation/model set, the shared `flow_model` admission contract, and R02's API projection.

**Auditable outcome.** A fixture matrix covers a supported imported descriptor, a supported stack, a property accessor chain, a shadowed decorator, a replacement decorator, and a metadata-preserving wrapper that changes behavior. Supported surface information becomes visible without admitting unsupported body-level claims. General arbitrary decorator execution is not an exit criterion.

## R04 — Promote options and configuration records; restrict new analysis to high-value relationships

**Observed basis.** The fact model already includes structural types and record fields for dataclasses, attrs/Pydantic classes, TypedDicts, and named tuples, including available constructor flags. It explicitly notes that record-field controls are not yet consumed as controls. The current plan also retains an older diagnostic in which dependencies were attributed across unrelated fields after constructing `ToolMeta`. That diagnostic is a regression target, not a fresh measurement of this HEAD. [S07] [S17]

**Recommendation and rationale.** Materialize an option/configuration projection tying each parameter or field to its owning API/configuration type, type term, requiredness, default/default factory expression, documentation, and known aliases. Preserve separate records for declared domains, documented choices, and values tested by source branches. A tested literal is useful discovery evidence, but not proof of an exhaustive accepted-value set.

Add only the immediate analyses that materially help API selection: a known constructor argument assigned to a particular field; that field read by a particular method; a parameter forwarded to a known formal; an observed producer/consumer handoff; and an extension parameter declared as callable versus directly observed being stored, forwarded, registered, or invoked. These must remain separately typed relationships with evidence and qualifications. They should not become one generic `supports` edge.

Field-specific construction/projection is especially valuable. For supported record models, `Config(timeout=t, title=n)` must not make every field read depend on both inputs. Keep identities at the field level. Unknown custom initialization, mutation, or aliasing should leave a local boundary rather than trigger a general heap-analysis project or silently distribute dependencies to all fields.

Associate configuration objects and examples with the public entry point that uses them. Keep return-flow summaries and abstract capabilities optional; many useful questions can be answered from explicit field mappings and documented controls. Do not infer lifecycle or invocation guarantees from a `Callable` annotation alone.

**Likely ownership.** Existing type/field/parameter facts and flow relations, a small product-oriented derived relation set, and R02/R06 projections. This is the main justified increment of semantic analysis in the proposed product scope.

**Auditable outcome.** A configuration search finds an option exposed through a typed record, not only a direct keyword argument. A two-field construction fixture preserves the correct source-to-field mapping without cross-field leakage. Unsupported custom behavior remains explicit. Declared, documented, and source-tested values stay distinguishable in the response.

## R05 — Make source-backed documentation and usage scenarios first-class across the catalog

**Observed basis.** `usage.rs` still selects patterns for seeds, refuses several enclosing constructs, and chooses a small self-contained statement subset. Canonical documentation already has passages, code blocks, mentions, and source mappings. Separately, the serving bundle's finding-support closure is explicitly rooted in findings cited by served assertions. Consequently, simply adding a new lookup may not expose evidence that was never selected into that projection. [S09] [S07] [S10]

**Recommendation and rationale.** Build operation/option-to-document and operation-to-scenario associations over the whole eligible public surface, independent of brief selection. Distinguish exact symbol links, resolved usage occurrences, lexical ambiguity, and similarity-only candidates. Preserve version/source-context alignment; a passage from another release can be retained as such but must not become an exact-release contract silently.

Represent a usage scenario as original source spans plus its enclosing context, relevant operations, explicit option values, setup dependencies, and validation state. Keep the current compact extractor when it succeeds, but use an enclosing block/function/example fallback when it cannot preserve context. An `async with` example should retain its context; a pytest-dependent example should disclose its fixture dependencies rather than vanish.

Do not equate bound names with semantically complete setup: prior receiver mutations, registration calls, and context entry may matter even when they introduce no new names. Prefer a larger faithful excerpt over an aggressively sliced “standalone” example whose behavior changed. Retrieval of context-dependent code is useful when labeled accurately.

Extend evidence-root selection to API contracts, options, scenarios, and bounded relationship records, reusing existing source IDs and closure validation. Do not create fabricated assertions merely to make their evidence eligible for export. Essential source/evidence associations remain available independently of a rendered brief. Add NumPy-style documentation parsing only when an intended library needs it; broader documentation syntax coverage is not a universal prerequisite.

**Likely ownership.** Existing documentation extraction and mentions; `cpg-core::usage`; evidence/support projections in `cpg-schema::bundle` and `serving_projection`; generation-qualified PostgreSQL hydration. The new roots should participate in existing manifest, reference, backup, and recovery contracts.

**Auditable outcome.** An operation outside brief seeds has retrievable usage evidence. Context-manager, exception-handling, and fixture-dependent examples retain context/dependency labels and exact source provenance. Syntax-checked, binding-checked, and executed examples remain distinct. Dangling or foreign-generation evidence is rejected.

## R06 — Retrieve evidence units, retain match witnesses, and keep the current embedding baseline

**Observed basis.** Native ranking enumerates `signature_doc` and `source_body` for operations. Exact ranking collapses chunk distances to an entity score, and the rank payload contains entity, view, score, and rank rather than the winning source unit. The current 1,024-dimensional specification still instructs queries to retrieve “capability briefs.” Token admission and the PostgreSQL deployment are already qualified in the recorded rollout. [S12] [S13] [S16] [S02]

**Recommendation and rationale.** Make retrieval units addressable independently of their parent operation. A unit needs a stable identity, parent/association identities, view kind, source spans or structured-fact references, rendering specification, input digest, and embedding specification. It must be possible to explain what matched without performing an unrelated second retrieval.

Add an API/options view and an original usage-scenario view to the current signature and source views. Keep documentation passages independently searchable, including conceptual pages that do not resolve uniquely to an operation. Structured views should serialize supported roles, values, and qualifiers, not generate narrative capability briefs. An unresolved dependency should not be serialized as an established supported capability merely to improve recall.

Replace the duplicated fixed view list with one small declared registry consumed by materialization, import validation, ranking, and hydration. This is a schema-owned catalog, not a dynamic plugin framework. Retain winning unit IDs and stable tie-breaking when aggregating units into operation scores. A query hit should expose the option, passage, or example responsible for the match.

Reassess fusion as evidence channels are added. Ten near-duplicate examples should not create ten independent votes. Normalize/group within each evidence family, preserve lexical/exact-symbol behavior, and evaluate whether the new channels contribute. Revise the stale query instruction under the existing specification/migration rules, without mixing incompatible generations. Retain Qwen3 and 1,024 dimensions as the baseline; keep exact retrieval selected. Any eventual ANN activation needs the existing admission process for the changed corpus.

**Likely ownership.** Compiler-side retrieval-unit construction; `cpg-schema` view/rank/projection contracts; `lctx-postgres::retrieval`; current lexical/fusion consumer; result hydration. New product relations extend the current PostgreSQL rollout instead of replacing its query owner.

**Auditable outcome.** A task expressed through an option value or usage idiom retrieves the right public API and the precise matched evidence. Winning-unit identity survives the PostgreSQL/native round trip. Duplicating an example does not give the operation uncontrolled rank inflation. Existing exact-symbol and lexical-only controls continue to work. Embedding and ranking gains remain measurements to obtain, not assumed outcomes.

## R07 — Separate strict matches, exploratory candidates, and joint applicability

**Observed basis.** `find_operations` tracks matches and unresolved operations, but `search_scope` puts only matched operations into the eligible set used by ranked retrieval. Facet classification accepts independently established/conditional facet rows; it does not establish that their conditions are jointly satisfiable in one behavior context. [S14] [S15]

**Recommendation and rationale.** Preserve strict search, and add an explicitly selected exploratory policy that can return relevant unresolved candidates. Expose per-requirement match/open/contradicted status and the evidence basis. Confirmed contradictions should not be hidden by semantic similarity; unsupported candidates should not be advertised as satisfying the filter. Candidate nomination does not write new facts or change membership/verdicts.

Put the selection policy in the existing typed Rust request/classification owner, then use the same resulting eligibility policy for lexical and vector legs. Do not add a second classifier in the MCP layer. Carry policy identity through cursors and response metadata where appropriate so pagination and hydration preserve the request's meaning.

Keep two questions distinct: “which operations have records for these features?” and “which operations support these features together under one configuration?” The first can use current facets. The second requires compatible contexts or supporting joint usage evidence. For the first release, return `joint_applicability_not_established` when the system has only independent conditional facts. A context-preserving example can provide evidence of a particular combination without becoming a universal proof.

This prevents both undesirable extremes: silently deleting promising APIs because static analysis is incomplete, and claiming arbitrary combinations because separate feature tags are present.

**Likely ownership.** `lctx-postgres::repository::{Where, classify, search_scope}` and their request/response contracts, with retrieval consuming the same eligible population. Existing condition reasoning remains owned by the Rust semantic layer.

**Auditable outcome.** A fixture universe contains a confirmed match, an unresolved candidate, and a contradicted requirement. Strict retrieval returns only supported matches; exploration can return the unresolved candidate with its open requirement. Neither treats the contradiction as a match. A function with mutually exclusive feature modes is not reported as supporting both simultaneously merely because two conditional facet records exist.

## R08 — Qualify discovery and implementation usefulness separately from deep semantic coverage

**Observed basis.** The existing evaluation includes detailed questions about multi-stage transfers, constructor defaults, decorator consumption, registration failures, and injected parameters. Those are valuable semantic tests, but are more demanding than simply helping an agent identify and correctly invoke the appropriate built-in API. PostgreSQL workload/ANN qualification does not answer this product question. [S18] [S02]

**Recommendation and rationale.** Add a bounded product evaluation alongside, not in place of, the current semantic suite. A proposed initial set is 24 tasks: eight feature-discovery tasks, eight implementation tasks, four configuration/API-choice tasks, and four ambiguity or unsupported-behavior tasks. Freeze intended outcomes and assessment criteria before using them to choose retrieval/model changes. Keep a genuinely held-out subset or a separate confirmation set.

Compare three conditions: current Context7; this project's documentation/source evidence baseline; and that baseline plus the proposed structured contracts, options, relationships, and scenarios. Use the same agent, target environment, task, and tool/token allowance. Capture the Context7/library version actually available; report a version mismatch rather than manufacturing a matched baseline. Context7's official description already includes version-specific documentation and source examples, so simply indexing those materials is not evidence of superiority. [S21]

Assess correct API selection, valid invocation and options, task-specific executable results where appropriate, unsupported claims, and unnecessary exploration. Retrieval recall and latency remain useful diagnostic metrics, but do not substitute for whether the agent completes the task. The additional evidence baseline isolates whether structured intelligence helps, rather than attributing gains from improved documentation ingestion to deep analysis.

Keep the frozen behavioral suite's outcomes intact, retain false-claim and evidence-integrity regression tests, and do not gate the first useful product on every open summary channel. Before claiming library-general benefits, add one library with a meaningfully different API or documentation style. This is a generalization check, not a prerequisite for putting the pilot into use.

**Likely ownership.** Current evaluation tooling, a separate product-task corpus, and the existing plan's release criteria. Integrate the recommendation into the current authoritative plan instead of maintaining competing execution plans.

**Auditable outcome.** A reproducible comparison records both successes and unresolved cases. The team can explain which additions improved agent outcomes and which did not. No Context7 superiority claim is made before measurement, and no incomplete semantic milestone is relabeled complete to obtain a product release.

## Implementation sequence and migration discipline

Start with R01/R02 and the baseline portion of R08. This provides a useful contract and a way to judge the work before adding new analyses. R03/R04/R05 can then proceed in bounded slices around the pilot's central APIs. Add R06/R07 over those real evidence records and run the held-out comparison. R02's packet becomes progressively richer as its evidence inputs land.

Carry new records through the existing chain: canonical facts or explicitly derived relations; serving schemas; projection identities/reference checks; PostgreSQL migrations/import; hydration/native transport; and serving/recovery tests. A table existing in PostgreSQL is not completion. Completion is a product query returning the intended information with correct identity, evidence, uncertainty, and context. The current projection manifest already declares relation/schema/content and artifact identities; extend it rather than making a side database. [S11]

Reuse published facts for product projection changes where possible. Ranking and presentation adjustments should not gratuitously rerun Pyrefly/ty. However, changing a canonical fact's meaning is an analysis/schema migration, not a presentation-only rebuild. Do not turn incremental recomputation into a separate infrastructure project before the product path works.

## Libraries and work deliberately not added

The workspace already contains the relevant infrastructure: Pyrefly/Ruff/ty, Arrow/DataFusion/Delta, petgraph, a BDD library, PostgreSQL drivers and COPY support, pgvector integration, native async bindings, and PostgreSQL integration-test support. None of R01–R08 requires a new major analysis or storage engine. [S03]

Do not make another analyzer, general solver, graph database, large ontology, full RCA expansion, generalized heap model, all-channel interprocedural completion, another ORM, or ANN tuning a first-release dependency. Preserve working components; change their release role rather than deleting them reflexively.

If a central API remains inaccessible because it is generated, decorated, or native, permit a small opt-in runtime-observation adapter. Python's `inspect` supports signature and wrapper/member inspection, with limitations; execute such inspection in a pinned, isolated worker, retain observations separately, and never interpret successful introspection as a proof of behavior. This is a trigger-based escape hatch, not a mandatory pipeline. [S20]

**Recommended next milestone:** on the existing pinned pilot, an agent can discover a relevant built-in API, inspect its complete supported contract, locate a correctly contextualized example, and follow its evidence, even when deeper behavior remains unknown. The decisive improvement is organizing and exposing the knowledge already extracted, supplemented by a small number of high-value surface and field analyses.


## Audit source register

All repository references are pinned to the review SHA. Source ranges identify the examined implementation; historical measurements are labeled in the narrative.

- **S01 — Reviewed commit.** Repository HEAD returned during the review; commit dated 2026-09-28. [S01]
- **S02 — STATUS.md.** Current rollout scope, deployment receipts, exact-versus-ANN decision, remaining semantic scope, and analysis-free brief failure. [S02]
- **S03 — Cargo.toml.** Current analysis, storage, native-binding, embedding adapter, and integration-test dependencies. [S03]
- **S04 — docs/postgresql.md.** Canonical Delta authority, PostgreSQL serving dependency, offline rebuilding, and reconciliation. [S04]
- **S05 — crates/lctx-postgres/src/hydration.rs.** Operation responses derive parameter entries from facets and attach fates; constructor assembly remains separate. [S05]
- **S06 — crates/lctx-postgres/src/hydration.rs.** Selected-operation loading, public-path resolution, public __init__ lookup, facets, statuses, and behavior loading. [S06]
- **S07 — docs/design/sections/facts-and-identity.md.** Provider provenance; existing signatures, parameter documentation, type structure, record fields, documentation, and usage inputs. Some paragraphs retain older implementation-status labels. [S07]
- **S08 — crates/cpg-core/src/flow_model.rs.** Existing narrow decorator classifier and shared decorated-set entry point. [S08]
- **S09 — crates/cpg-core/src/usage.rs.** Seed-oriented usage extraction, exclusions for enclosing constructs, candidate limits, source spans, and parsed/bound-name checks. [S09]
- **S10 — crates/cpg-schema/src/bundle.rs.** Finding support closure is rooted in findings cited by served assertions. [S10]
- **S11 — crates/cpg-schema/src/serving_projection.rs.** Projection/bundle formats, artifact membership, relation/schema digests, resource bounds, and manifest identities. [S11]
- **S12 — crates/lctx-postgres/src/retrieval.rs.** Rank payload and hard-coded signature_doc/source_body operation views; qualified route selection. [S12]
- **S13 — crates/lctx-postgres/src/retrieval.rs.** Exact ranking aggregates chunk distances to entity scores; IPC exports entity, view, rank, and score. [S13]
- **S14 — crates/lctx-postgres/src/repository.rs.** search_scope includes only matched operations, not unresolved candidates. [S14]
- **S15 — crates/lctx-postgres/src/repository.rs.** Facet classification accepts established/conditional rows independently and distinguishes open records. [S15]
- **S16 — specs/embedding/qwen3-embedding-8b.json.** Current 1,024-dimensional MRL specification and remaining capability-brief query wording. [S16]
- **S17 — docs/plans/behavioral-model-forward-plan_2026-09-24.md.** Historical checkpoint diagnostic, including cross-field dependency over-attribution; not a fresh measurement of the reviewed HEAD. [S17]
- **S18 — eval/behavior/fastmcp-4.0.5.toml.** Existing semantic evaluation and detailed multi-stage option-transfer question. [S18]
- **S19 — Python functools documentation.** Wrapper metadata and __wrapped__ association; not a general semantic-equivalence contract. [S19]
- **S20 — Python inspect documentation.** Optional runtime inspection APIs and signature/wrapper limitations. [S20]
- **S21 — Context7 official overview.** Context7 describes version-specific documentation and source code examples; no comparative performance claim inferred. [S21]

[S01]: https://github.com/paul-heyse/library-context/commit/739f9dfbfa2fb3acb52574e8bf894599b5d34af7
[S02]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/STATUS.md
[S03]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/Cargo.toml#L1-L122
[S04]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/docs/postgresql.md#L1-L220
[S05]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/crates/lctx-postgres/src/hydration.rs#L240-L470
[S06]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/crates/lctx-postgres/src/hydration.rs#L1-L210
[S07]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/docs/design/sections/facts-and-identity.md#L94-L142
[S08]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/crates/cpg-core/src/flow_model.rs#L305-L402
[S09]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/crates/cpg-core/src/usage.rs#L1-L132
[S10]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/crates/cpg-schema/src/bundle.rs#L1-L190
[S11]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/crates/cpg-schema/src/serving_projection.rs#L1-L230
[S12]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/crates/lctx-postgres/src/retrieval.rs#L1-L230
[S13]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/crates/lctx-postgres/src/retrieval.rs#L270-L430
[S14]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/crates/lctx-postgres/src/repository.rs#L255-L420
[S15]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/crates/lctx-postgres/src/repository.rs#L255-L420
[S16]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/specs/embedding/qwen3-embedding-8b.json#L1
[S17]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/docs/plans/behavioral-model-forward-plan_2026-09-24.md#L98-L130
[S18]: https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/eval/behavior/fastmcp-4.0.5.toml#L1-L80
[S19]: https://docs.python.org/3/library/functools.html#functools.update_wrapper
[S20]: https://docs.python.org/3/library/inspect.html
[S21]: https://context7.com/docs/overview
