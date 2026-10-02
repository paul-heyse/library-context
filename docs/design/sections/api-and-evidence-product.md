# API and evidence product target

**Phase 4 Implemented; qualification in progress, 2026-10-01.** The accepted
[semantic model](semantic-model.md) owns the catalog, associations, selection, synthesis and
retrieval contracts. Their Phase 4 construction uses typed `lctx-model` owners, native computation
and `cpg-core` PostgreSQL generation adapters. Phase 5 serving remains unavailable. PR6 comparative
product work remains paused until that boundary is delivered.

<a id="section-14"></a>

## §14 Purpose, status and decision

**Accepted target (ADR-0071).** The product helps a coding agent find a built-in library capability,
select its exact public API and configuration, implement or deploy it, and inspect the evidence and
limitations behind that choice. Structured selection and connected, version-scoped evidence are
its proposed advantage. General behavioral completion is enrichment rather than the first-product
exit criterion.

PR1–PR5 define the retained product contract. Their earlier pipeline qualification does not qualify
the replacement typed pipeline or make its agent interface available. Phase 4 now constructs the
catalog and evidence needed by that interface; Phase 5 must connect them to serving and qualify the
result. Neither implementation establishes an advantage over Context7.

The [cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md) and
[Phase 4 plan](../../plans/semantic-model-phase4-detailed-plan_2026-09-30.md) own current delivery and
qualification. The [forward plan](../../plans/behavioral-model-forward-plan_2026-09-24.md) owns the
retained product sequence and product findings. New product behavior remains **Proposed** unless
its owning implementation and qualification are explicitly identified. Logical domain contracts
below lower from `lctx-model`; they are not independent SQL or Python schema authorities.

> Decision: ADR-0071, ADR-0085, ADR-0086, ADR-0087

<a id="section-14-1"></a>

## §14.1 What must be different from Context7

Version-aware documentation, source links, search and examples are comparison baselines, not
sufficient differentiation. The comparison must use Context7's strongest relevant route available
at the time of the frozen run (§14.12).

**Proposed advantage:** require a declared parameter, configuration-field domain, invocation form
and linked usage/deployment evidence, then return supported and unresolved results separately with
witnesses for each requirement. An implementation packet follows those same identities and pinned
original bytes. The agent need not reconcile unrelated snippets to establish what matched.
Whether this improves task success remains an evaluation question.

| Agent journey | Required product answer | Distinguishing contract |
|---|---|---|
| Find a registration feature with a timeout option | Public surface, option owner/default and matching evidence | Typed option joined to public identity and original scenario |
| Choose object configuration or per-call setting | Both scopes, declared defaults, field/reader links and unresolved precedence | No inferred global precedence |
| Use an asynchronous context-manager API | Factory signature, original use and setup/teardown qualification | Invocation form is distinct from full cleanup proof |
| Deploy a transport | Package/extras declarations, launch/config material, options and prerequisites | Release-scoped deployment evidence |
| Use a capability behind an unmodeled wrapper | Source declaration and observed/documented use, effective signature unresolved | Local uncertainty preserves usable evidence |
| Combine requirements | Per-requirement witnesses and joint-applicability status | Independent matches do not prove simultaneous support |

FastMCP 4.0.5 is the pinned pilot input. A usable pilot and a differentiated pilot are separate
milestones; current catalog implementation establishes neither. A second library is required
before a library-general claim.

<a id="section-14-2"></a>

## §14.2 Assessing the recommendations

The accepted R01–R08 dispositions remain product requirements. Their implementation now follows
the single typed semantic model rather than the retired catalog/projection pipeline.

| Recommendation | Retained contract | Current delivery boundary |
|---|---|---|
| R01 independent catalog readiness | Public universe independent of optional analysis/briefs; capability availability explicit | Phase 4 C0/catalog construction; Phase 5 readiness and loading |
| R02 API contracts and packets | Distinct public exposure, binding, declaration and signature variant | Phase 4 typed catalog; Phase 5 ordered packet hydration |
| R03 decorator surface | One ordered normalization owner; binding, protocol, registration and body admission separate | Normalized and Model owners; no general decorator evaluator |
| R04 options and relationships | Defaults/factories and domain basis explicit; field links scoped | Typed catalog and bounded Local/Summary/Structural evidence |
| R05 docs/scenarios/evidence roots | Original enclosing context, independent roots and expected-failure intent | C1 evidence/associations and S0 original-source grounding |
| R06 units, views and witnesses | Finite families, contextual units, family-normalized ranking and winning-unit witnesses | Phase 4 retrieval construction; Phase 5 query ranking |
| R07 strict/discovery/joint support | Explicit completeness domains, conflict and availability; one classifier | Phase 4 C2 selection; Phase 5 request/response adapters |
| R08 agent-task evaluation | Frozen development/confirmation tasks, deployment, usability distinct from differentiation | PR6 paused; §14.12 remains Proposed |

Runtime inspection is a conditional isolated observation for a task blocked by source/evidence
absence, never request-time import. PostgreSQL trigram suggestions, native text search and ANN
changes require a measured task gap and a replacement comparison. Coarse rebuilds reuse exact
admitted inputs; they do not introduce another canonical store or general incremental engine.
Deployment metadata, safe negative examples, version alignment and deterministic candidate
comparison serve the same product contract rather than expanding general analysis scope.

<a id="section-14-3"></a>

## §14.3 Responsibilities and representation flow

> Decision: ADR-0071, ADR-0073, ADR-0074, ADR-0085, ADR-0078

The flow is pinned sources and metadata → attributed facts → normalized relations → typed Phase 4
analysis/catalog owners → validated PostgreSQL generations. Phase 5 will consume those generations
for typed selection, bounded hydration, ranking and MCP presentation.

| Owner | Responsibility | Consumer boundary |
|---|---|---|
| Acquisition, `cpg-extract`, `cpg-flow` | Pinned inputs, declarations/types/docs/package metadata, attributed source and optional flow observations | No static-fact extraction by library import |
| `lctx-model::domain` and normalized owners | Declared relations, identities, codebooks, codecs and shared invariants | One semantic and wire authority; PostgreSQL physical contracts derive from it |
| Local, execution, transfer, Model and Summary | Pure bounded semantics, coverage, proof and obligations | Explicit qualification and uncertainty; no ambient store effects |
| Structural and Analytic; native `lctx-analytics` kernels | Typed traversal/patterns and ranking, community, concept and neighbour results | Complete input/universe/parameter identity; no inherited capability from navigation |
| C0 catalog, C1 evidence/association, C2 selection | Public contracts, original evidence, contextual witnesses and classification | Independent of brief seed selection |
| S0 synthesis, retrieval and embedding | Assertions/briefs/code boundaries; contextual units and explicit vector consumers | Grounded source/support closure; spec-qualified vectors |
| `cpg-core` | Stage read grants, decode/compute/validate/publication adapters | Real generation-store writes, no parallel semantic policy |
| `lctx-postgres` | Generation state, physical schema, typed storage and read/write grants | One relational store; readiness distinct from selection |
| Phase 5 Rust/native and `lctx_mcp` adapters | Typed requests, bounded hydration, optional semantic loading and presentation | Serving unavailable until its own implementation/qualification; Python does not restate semantics |
| Evaluation tooling | Pinned task environments, independent checks and paired runs | No task answers or skill-derived gold enter compiler inputs |

The catalog is a compiled projection of canonical evidence, not an authored knowledge store.
Pure owners consume explicit immutable inputs; effects occur through the stage adapters. Suitable
bulk computation uses DataFusion and finite closures use indexed worklists/native kernels. Indexes
preserve alternatives and one-to-many observations rather than choosing a convenient conflicting
record. Original evidence is content-addressed before serving; a citation must not silently replace
pinned bytes with a mutable current web page.

<a id="section-14-4"></a>

## §14.4 Public API and configuration contracts

> Decision: ADR-0071, ADR-0074, ADR-0078, ADR-0106, ADR-0108

**Public identity.** A public member names the release's exposed owner/name/access path independently
of binding interpretation. It references declarations without renaming them. Alias/inherited origins,
ordered MRO/shadowing and binding/access forms retain their own evidence. Shared implementations
may have multiple public exposures. Ambiguity returns choices, not an arbitrary declaration;
modules are browse groups, not fictional callable operations.

**Signature variants.** Preserve source, effective, synthesized, documented and observed roles,
ordered parameters, overload alternatives, types and evidence. Do not merge overloads into an
invented union signature. Parameter kind, requiredness, original default expression and exact
literal are distinct. No default, `None`, unknown default and factory expression are different;
rendering never evaluates them. Generated constructor metadata does not establish runtime storage.
Source and effective options have separate identities even when their displayed names coincide.

**Invocation and decorators.** Calls, bound/unbound methods, static/class methods, property accessors,
awaiting, iteration and modeled context use are separate forms with stated bases. Source `async def`
does not characterize an unresolved exported wrapper. Ordered decorator applications preserve
identity/candidates, source/application order, argument expressions and unresolved remainder.
Selected models establish binding, accessors, metadata, protocol or registration separately.
`wraps`/`__wrapped__` is a link, not behavioral equivalence. Catalog, synthesis and behavioral
admission consume the same normalized facts with their own declared claim boundaries.

**Options.** Options belong to a signature parameter, configuration field or explicitly documented
deployment control. Scope, type, requirement, aliases, defaults/factories and configuration-object
associations are attributed. Declared Literal/enum domains, documented choices and source-tested
values remain different evidence. A branch comparing `x == "http"` does not prove the accepted
transport set. Nested options retain bounded depth/cycle references rather than fictional dotted
keyword parameters.

**Field links.** Supported constructor-formal → exact field initialization and exact field read →
consumer links preserve argument binding, receiver/field identity and mutation boundaries.
`Config(timeout=t, title=n)` cannot attribute a read of `title` to `t`. Source reader-to-consumer
composition is separate from an exact initialization proof. Symbolic field reads remain
`Unknown`/proof-free where the Summary model lacks instance/temporal alias closure. Unknown aliases,
initializers or overrides do not become an all-fields taint join. General heap analysis is excluded.

<a id="section-14-5"></a>

## §14.5 Evidence, scenarios and useful relationships

> Decision: ADR-0076, ADR-0106, ADR-0108

Evidence retains original artifact digest, source kind/release alignment, span or stable anchor,
original bytes and extraction basis. Associations distinguish declaring, documenting, demonstrating,
invoking, expected failure and candidate suggestion. Exact resolution, explicit cross-reference,
ambiguous mention and similarity are different bases. Display preference cannot reconcile a
contradiction. Exact, evidence-mapped, other and unknown version alignment remain explicit.

C1 owns artifacts, original spans, scenarios, deployment records and associations. Release metadata
has a release subject rather than being copied into every member. Provider/site identity, modality,
invocation phase and original coordinates survive association. A singleton candidate is not a
resolved call. Synthetic Python coordinates and original MDX fences remain distinct.

Scenarios retain ordered original spans and enclosing example/function/class, APIs and explicit
bindings, import/fixture context, previous receiver mutations, registrations and unresolved inputs.
Do not synthesize missing setup or join unrelated fragments into a working recipe. Independent
status dimensions cover context completeness, parse/binding/environment/execution checks, intent
(demonstration/test/expected failure/skip/mixed/unknown), and any exact observed environment/result.
A parsed example is not executed; a passing expected-exception test is not a successful-use recipe.
Missing runnable examples are disclosed.

S0 documentary claims require an exact C1 member association, authenticated original spans and the
supported source scope. Native warning/parameter components retain literal attributes, nearest
field context and distinct source/effective option identities. Unknown, ambiguous, nested, inline
or unsupported fenced components cannot acquire member authority merely through their labels.
A refusal keeps the evidence and boundary rather than manufacturing an assertion.

API, option, scenario and relationship evidence roots survive independently of briefs or behavioral
claims. Same-generation closure is validated; dangling/foreign-generation references fail. P5
`get_evidence` must return bounded original bytes and context with generation/representation-bound
continuation. Omission cannot upgrade execution/context status or turn a partial source into a recipe.

| Relationship | Meaning | Limit |
|---|---|---|
| Facade delegates to helper | Resolved callsite route and direct bindings | Finite frontier; no inherited callee capability |
| Option initializes field / reader consumes field | Supported exact field identity or qualified symbolic route | Mutation/alias boundaries; no general instance equivalence |
| Producer handed to consumer | Original expression flow or established direct binding | Distinct from type-compatible suggestion |
| Callable declared/stored/registered/invoked | Separate extension-point evidence and phase | Callable type alone does not establish invocation |
| Related API | Explicit doc link, owner, co-use or typed handoff | Similarity is navigation only |

Expansion is finite and budgeted. Typed routes replace generic support edges; no arbitrary graph
traversal or composition planner is exposed.

<a id="section-14-6"></a>

## §14.6 Deployment evidence and reproducible use

> Decision: ADR-0076

Deployment evidence concerns the analyzed library, not our PostgreSQL runbook. Preserve exact
`Requires-Python`, conditional `Requires-Dist`, extras, entry points, environment/lock identity and
pinned configuration/docs. Verify acquired metadata bytes against distribution provenance before
parsing. Retain duplicate fields and interpretation failures. A declared extra is different from a
validated minimal installation; a full-extras pilot environment does not prove a sufficient subset.

Original CLI/shell/configuration blocks, documented environment-variable names, transport/setup
links and external prerequisites are observations. Environment reads do not establish deployment
necessity or acceptable production values. Never record operator secret values. Compilation and
requests do not execute those examples.

Explicit external task observations retain source/runner identities, exact lock/interpreter/runtime
inputs, command and observed outcome. Shared validation refuses stale or mismatched receipts and
retains failed outcomes with their scope. Their availability is separate from static extraction;
Phase 4 implementation makes no new live FastMCP deployment claim.

A future deployment packet returns release/Python range, package/extra declarations, original launch
material, API/options, prerequisites and check status. Minimal-install or requested-transport success
requires an isolated task check. A successful configuration does not establish all platforms or
transports. Pilot usability requires at least one programmatic setup and one CLI/config task in a
pinned disposable environment, without a generic shell semantics engine or mandatory runtime probing.

<a id="section-14-7"></a>

## §14.7 Query semantics and customizable requirements

> Decision: ADR-0071, ADR-0073, ADR-0076, ADR-0077, ADR-0081, ADR-0085; proposed refinement ADR-0114

**Implemented / focused-Tested foundation, 2026-10-02:** the
[foundation enhancement scope](../../plans/semantic-model-foundation-enhancements_2026-10-01.md)
separates the 45-relation classification inventory and six C2 output relations from broad
publication replay. Owned preparation retains charged indexes and uses the same finite predicate
and conjunction operations; standalone preparation still performs the authoritative broad replay.
The opaque canonical PostgreSQL consumer verifies content, captured source/epoch identity,
applicable selection availability and matching C2 validator receipts, retaining the original
serving-role generation lease. Actual Catalog admission, strict replay parity, corruption refusal
and lifetime controls passed in both profiles on 2026-10-02; scope-end qualification is in progress.
These foundations do not activate catalog routes or MCP serving.

**Proposed serving integration, 2026-10-01:** [ADR-0114](../../adr/0114-generation-serving-contracts.md)
and the [Phase 5 plan](../../plans/semantic-model-phase5-detailed-plan_2026-10-01.md) develop
process-wide preparation and the remaining request, lifecycle and representation contracts.
They establish no new predicate semantics.

Phase 4 C2 owns pure typed selection and contextual witnesses. Phase 5 must adapt those contracts
to PostgreSQL selection/hydration and MCP requests. Rust owns decoding/classification; Python
transports/presents it. Arbitrary SQL, an unconstrained predicate DSL and a second classifier are
outside the interface.

Each finite requirement declares domains/quantifiers, input relations/capabilities, witness shape
and completeness scope. Unsupported predicates are refused before effects. Nominal member,
binding, signature, evidence, type, generation and unit identities do not replace checking
same-generation membership and relation role. Generated wire schemas derive from Rust contracts;
serialization/deserialization, tags, closed objects, missing/null, defaults and bounded fields are
explicit. Schema-valid JSON still requires contextual and evidence-closure validation.

Initial requirements cover public identity/kind, invocation, declared parameter name/kind/type/default,
configuration domain/scope, explicit relationships, scenario intent/check status, source/version and
deployment metadata. A conjunction has at most 16 requirements. Structural type equality, nominal
identity and declared union membership do not imply runtime assignability; display similarity is a
hint. New predicates need a definition, basis and absence/completeness scope.

The default declaration query is existential over applicable variants/exposures. Witnesses retain
variant, binding and configuration context. A complete variant without a formal does not refute a
member whose other overload/effective surface is unresolved. Member absence requires closure over
the entire requested domain. Joint invocation requires compatible witnesses in one admissible
variant/binding/configuration context; unrelated instances or overloads cannot be combined.

Each requirement is `supported`, `contradicted`, `unresolved` or `conflicting`, with basis, context,
witnesses and coverage. These selection outcomes preserve any cited five-way behavioral verdict:

- Complete signature absence can contradict `declares_parameter("timeout")`.
- It cannot contradict keyword acceptance under unresolved wrappers, aliases or `**kwargs`.
- Missing branch literals or bounded searches do not prove rejection/absence.
- A test establishes its observed inputs/environment, not universal support.

Discovery is the default: supported, unresolved and conflicting candidates form separate groups.
Strict admits supported requirements under the requested basis. Any contradicted requirement excludes
a candidate from ordinary groups; explicit comparison may show it. Otherwise conflict takes its
own group, then unresolved discovery, then all-supported results. Joint applicability can downgrade
independent all-supported matches. Never choose the best verdict across incompatible requirements.

Joint status distinguishes independent records, compatible modeled context, demonstrated combination,
contradictory modeled context and not established. Existing facets are record evidence rather than
simultaneous behavior. Conditions compose only under shared evaluation/context identities, with
bounded uncertainty preserved; no general solver closes arbitrary combinations.

Request input closure includes actual selected context domains, missing lookups, coverage and
membership. Unrequested domains cannot establish absence. Budgets separately bound row conversion,
retained inputs, indexed work and returned evidence; a refusal is not an empty complete result or a
measured process-memory ceiling. P5 envelopes must distinguish exhaustive enumeration from ranked
retrieval, capability availability, unknown/conflict counts and continuation. Cursors bind generation,
normalized requirements, joint/group/ordering policy. Every retrieval leg filters eligible sets before
its final limit; bounded ranking never claims exhaustiveness.

<a id="section-14-8"></a>

## §14.8 Retrieval units, witnesses and ranking

> Decision: ADR-0077, ADR-0085; proposed refinement ADR-0114

**Proposed refinement, 2026-10-01:** [ADR-0114](../../adr/0114-generation-serving-contracts.md)
places fusion and winning-witness decisions in one model operation over library numerical scores.
Its narrow disposable exact-vector artifact derives from canonical embedding-use bytes/specs;
semantic units, contextual membership and evidence remain canonical. Physical admission, cleanup
and actual retrieval journeys remain Phase 5 implementation/qualification.

Phase 4 retrieval constructs addressable units with typed subjects, evidence origins, family,
rendering/input identity and source context. The four families are API/options,
documentation/deployment, scenario and source. Conceptual pages without an exact API association
remain evidence units; useful content is not forced into a guessed member ID. Brief retrieval is a
distinct consumer.

Unit, fragment, contextual occurrence and vector identities are separate. Identical family/text
can deduplicate corpus material while preserving original occurrences, member associations and
winning context. Bounded UTF-8 fragments do not discard original anchors. Embedding use/spec and
complete text keys are explicit; token refusal leaves original evidence addressable. Lexical-only
availability must be labeled. Query instruction/spec changes cannot mix incompatible vectors.

The retained P5 ranking contract uses exact-symbol priority, BM25 and exact vector ranks, with
one best contribution per family/member, deterministic ties and equal-weight family-normalized
RRF K60. Duplicate examples are not extra votes. Return winning unit/channel/rank metadata rather
than finding an approximate explanation afterward. A ranking witness is distinct from the evidence
that establishes a requirement. Eligibility precedes ranking.

Phase 4 unit/embedding construction does not establish live embedding service availability,
retrieval quality or MCP search. Phase 5 must admit its physical indexes and query policy against
canonical generation/spec identity; an ANN route needs separate qualification. Serving artifacts
are derived from typed generations rather than another canonical bundle/import store.

<a id="section-14-9"></a>

## §14.9 Agent interface and bounded implementation packet

> Decision: ADR-0071, ADR-0073, ADR-0076, ADR-0081; proposed refinement ADR-0114

**Proposed refinement, 2026-10-01:** [ADR-0114](../../adr/0114-generation-serving-contracts.md)
and [Phase 5 packages](../../plans/semantic-model-phase5-detailed-plan_2026-10-01.md#8-dependency-ordered-execution-packages)
specify a process generation guard distinct from request SQL occupancy, one CPU admission owner,
current model-derived wire mappings and bounded native exact-input reconstruction. The ten retained
routes and indivisible mandatory packet contract remain the target; implementation is not_run.

**Accepted Phase 5 contract; serving unavailable.** Ten retained routes are `search_operations`,
`find_operations`, `get_operation`, `browse_library`, `get_evidence`, `search_evidence`,
`compare_operations`, `search_capabilities`, `get_capability` and `inspect_value_paths`.
They are not newly qualified by Phase 4. Proposed concept/explanation tools remain separate scope.

| Route | Responsibility |
|---|---|
| `search_operations` | Ranked APIs with typed requirements, grouped selection and winning-unit witnesses |
| `find_operations` | Complete supported-domain selection, coverage/counts and continuation |
| `get_operation` | Unambiguous public identity and bounded implementation packet |
| `browse_library` | Deterministic module/class/member outline and scoped vocabulary/counts |
| `get_evidence` | Generation-qualified original span/scenario/deployment expansion |
| `search_evidence` | Independent documentation/scenario/deployment discovery |
| `compare_operations` | One to five named APIs against the same requirements/classifier |
| Capability and value-path routes | Optional grounded briefs/native semantics with explicit availability and qualification |

Rust owns generated request/output schemas, bounded decoding and complete nested packets. Thin
FastMCP adapters preserve root-object/`$defs`/tags/missing-null through actual listing/invocation,
structured `ToolResult`, lifespan/errors, total deadlines, cancellation and leases. Presentation-only
Python models remain useful; duplicate semantic declarations do not. Native work must respect a
bounded worker slot and generation lease through cancellation. No source/library execution occurs
in the query path.

Mandatory packets and independently hydrated optional sections share generation/member/section/
representation-bound cursors. Required signatures are indivisible; optional enrichment cannot block
core access. Include identity/release, public access/import provenance, invocation/signature variants,
options, at most two positive original scenarios, relevant deployment prerequisites, at most five
relationship links, evidence references and local conflicts/limits. A brief is optional. A singleton
retains its public identity and explicitly named class operation; class lookup cannot silently
choose an instance. No separate recipe knowledge store is needed.

Retained limits are 20 default/100 maximum browse/evidence rows, 16 requirements, one to five
comparison candidates, and 32 KiB default/256 KiB expanded final response bytes. Optional omission
and continuation are explicit; required signatures are never silently truncated. These are limits
to qualify, not latency measurements. Natural language nominates candidates but does not manufacture
structured matches.

<a id="section-14-10"></a>

## §14.10 Publication, optional analyses and rebuild boundaries

> Decision: ADR-0071, ADR-0073, ADR-0077, ADR-0078, ADR-0081, ADR-0085

Public catalog construction is independent of optional behavioral analysis and brief seeds. Catalog
and behavioral profiles have explicit identity. Unrequested analysis is `NotRequested`; selected
partial/unsupported coverage remains declared. Crashes or invalid required outputs fail publication.
A seed without an admissible Outcome produces no brief rather than a weakened documentary claim.

Phase 4 adapters read complete granted input closures, compute typed results and invoke shared
validation before publishing through the generation store. Coverage, readiness and availability are
different: unknown effective signature, absent scenario, unselected analysis and corrupt required
artifact cannot collapse to the same empty result. Advertised semantic closure must validate.
Phase 5 must check those capabilities before loading native execution; corrupt claimed artifacts
cannot fall back to a different backend.

Coarse dependency-aware reuse remains the accepted rebuild contract:

| Change | Required invalidation |
|---|---|
| Source/environment/provider meaning | Extraction and affected normalized/selected analyses, then catalog/retrieval |
| Newly captured facts | Current model/facts and all consuming stages; old missing values cannot become false |
| Association policy with unchanged facts | Catalog/evidence and dependent unit/support closure |
| Unit rendering or embedding spec | Retrieval/vector artifacts under a new admitted identity |
| Ranking/display policy | Serving policy/cursors; unchanged source facts and units need no reinterpretation |

Reuse includes scanned relations, group membership, missing lookups, roots/profile, coverage and
policy/provider definitions. Evidence-only or moved-span changes require current attribution/source
identity even when semantic shape is unchanged. Admission compares canonical outputs and provenance
against recomputation; valid encoding alone is insufficient. No measured reuse or performance claim
is made here, and no obsolete Delta or catalog-rebuild runtime is preserved as a second authority.

A serving generation must reference exact semantic content, retrieval/spec set, required capabilities
and artifact closure. Validate/publication precede ready/selection transitions; selection is not
readiness. Readers retain their pinned generation. Optional future caches are disposable and
version-qualified, never replacements for canonical validation.

<a id="section-14-11"></a>

## §14.11 Library fit, retained work and exclusions

> Decision: ADR-0071, ADR-0073, ADR-0074, ADR-0076, ADR-0081, ADR-0085

Reuse the pinned Ruff/Pyrefly/ty extraction, Arrow/DataFusion compute, PostgreSQL/SQLx/pgpq storage,
existing graph/condition kernels, PyO3 lifetime adapters, FastMCP transport and embedding codecs/cache.
`lctx-model` owns domain rules; native modules re-export those owners, and `cpg-core` performs effects.
No parallel analyzer, ORM, graph store, semantic Python engine or broad orchestration framework is
selected. [Pins](../../pins.md) owns versions, rather than stale versions in this product section.

Schemars derives wire schemas; independent schema tests use the selected offline validator policy.
Typed decoding/shared domain validators own production ingress. Schema and nominal-ID compile-fail
tests must exercise actual contract distinctions. SQLx adapter types do not replace smart-constructor
invariants. Dynamic identifiers come from executable declarations, values are bound, and Arrow
codecs/grants validate results. JSON belongs to genuinely variable payloads, not core selection keys.

Conditional alternatives retain consumer triggers in the [forward plan §7](../../plans/behavioral-model-forward-plan_2026-09-24.md#7-deferred-each-with-a-trigger):

| Candidate | Trigger and required boundary |
|---|---|
| Salsa | Demonstrated fine-grained derivation cost beyond coarse reuse; pure inputs, provider-family separation and disposable compatible cache |
| Ascent / Differential Dataflow | Difficult supported recursive closure / actual continuous insert-delete views; explicit evidence, negation completeness and deletion semantics |
| Moka | Measured repeated immutable-generation requests; generation/policy keys, cancellation/error behavior and hard request budgets |
| Dense index/interner/bitmap/persistent/small-vector crates | A measured representation need; canonical IDs and universes remain exact |
| SeaQuery | Substantive finite dynamic SQL expressions; it does not own predicate meaning |
| Validation/build helpers or schema/client generation | Repeated local constraints or a concrete external-schema/client consumer; one schema authority |
| Rewrite/unification/solver engines | Named pure reasoning gap with explicit laws and unknown-preserving semantics |
| New parsers, syntax/protocol/plugin/scientific systems | A selected library/task with a concrete unsupported format or consumer |

Salsa/Ascent comparisons must include cycles, deletion, missing lookups, source coordinates, policy,
complete outputs/provenance and cold/reused costs. Library availability does not prove speed or
product advantage. PostgreSQL trigram/text ranking and ANN remain conditional comparisons.

Optional runtime surface inspection is isolated, bounded and explicitly attributed. Import or
annotation inspection can execute code; neither is a pure serving operation. Observations cannot
overwrite static facts or establish wrapper equivalence. New formats get a focused pinned-library
check when a task requires them, not universal parser expansion.

Retain sound five-way verdicts, negative-claim criteria, attribution, receipts, regression fixtures
and sealed evaluations. Retention of domain contracts does not preserve duplicate legacy engines:
cutover removes them at the owner boundary. Withhold unsafe exposed claims. General decorator/heap
completion, all-channel summaries, lifecycle/exception closure, ontology expansion, engine bakeoffs,
logging models, ANN tuning and distributed deployment need a frozen product task or named defect.

<a id="section-14-12"></a>

## §14.12 Product qualification and stopping rule

**Proposed protocol; comparative qualification is not established by Phase 4 implementation.** Freeze a 24-task development
set before changing retrieval: eight feature-discovery, eight implementation (including at least
two deployment/setup tasks), four API/configuration-choice and four ambiguity/unsupported cases.
Then prepare a distinct 24-task confirmation set with the same strata, hidden from implementation
and tuning. Separate curator/reviewer responsibilities or seal the packet mechanically; if task
answers leak, disclose exposure and replace the confirmation set before scoring. Keep the existing
behavioral/brief gold and `eval/heldout` isolated and unchanged.

Run three conditions with the same agent/model/prompt, library environment, task checks and overall
tool/token/time budget: (A) current Context7, including its strongest available relevant API route;
(B) our pinned original docs/source/scenarios with unstructured retrieval; (C) B plus structured
contracts, options, requirements, relationships and packets. B and C use the same corpus and model;
ablate newly added evidence families and structure separately on development tasks. Record actual
Context7 library/version availability and requests. If exact versions cannot be matched, report
that stratum separately; a stale/missing-source win alone is not a structural-intelligence claim.

Before freezing confirmation, declare a 24-task comparison population with the same target release
and materially comparable original evidence availability for A/B/C. Confirm the Context7 version
and source coverage without running confirmation answers. All success/improvement thresholds below
apply to that declared comparable population; mismatched or stale-source wins cannot satisfy them.
If it cannot be assembled or availability changes during the run, comparative acceptance is
`blocked` with the missing coverage named. Keep the usability result and diagnostic unmatched strata,
but do not shrink the denominator, replace difficult tasks or claim a structural win after results.

Use at least three independently seeded agent runs per confirmation task/condition. Task success
requires the correct built-in API, valid invocation/options and task-specific checks of produced
code where executable; constrained explanation tasks use source-backed reviewer rubrics. API agents
are **task performers**, not the sole judges of their own answers. Check deployment in a disposable
pinned environment. Score unsupported claims and unsafe substitutions independently of success.
Report paired wins/losses/ties by task and stratum, not just aggregate retrieval recall. Include
tool calls, input/output tokens, p50/p95 latency, compile/index cost and live-service requirements.

Two distinct milestones prevent an attractive demo from being called differentiation:

1. **Usable pilot:** the entire selected core API set is discoverable; required contracts and
   evidence round-trip through production PG/MCP; unknowns remain local; end-to-end invocation and
   deployment tasks work with real embeddings; corruption, contradiction, stale-generation,
   cancellation and optional-capability controls pass. No prescribed reduction in semantic unknowns.
2. **Differentiated pilot:** on confirmation, C achieves task-majority success (at least two of
   three runs) on at least 20 of 24 tasks, including at least six of eight implementation tasks and
   all four ambiguity cases. Against A, require at least four more majority-success tasks, spanning
   at least two task strata, with no stratum losing more than one task. Against B, require at least
   two more majority-success tasks attributable to structured features. No disallowed claim may pass
   the negative-task checks; C must not increase supported-task false-claim counts. Equal outcomes
   with prettier packets or lower latency alone do not satisfy this first differentiation contract.

These are proposed **engineering acceptance thresholds**, not statistical proof from 24 tasks.
Freeze them with the protocol before results; report uncertainty and all failures. If parity or
regression occurs, the objective remains open: use development evidence to choose the smallest
missing association/analysis, then use a fresh confirmation set. Do not redefine success or tune
against the sealed answers. A repeat confirmation and a second differently shaped library are
required for a broad superiority claim. The earlier semantic suite retains its own truth criteria.

<a id="section-14-13"></a>

## §14.13 Migration and immediate implementation priority

**Phase 4 Implemented; qualification in progress, 2026-10-01.** The current delivery is typed Local,
execution/transfer, Model, Summary, Structural, Analytic, C0/C1/C2, S0, retrieval and embedding owners
with real PostgreSQL generation adapters. The Phase 4 plan owns review/Q0 outcomes and any remaining
qualification. This section does not promote that status to acceptance, a FastMCP pilot, live
embedding qualification or product differentiation.

The cutover is a hard ownership replacement. Remove retired `cpg-schema` semantic declarations,
Delta/projection/bundle authorities, duplicate catalog classifiers and legacy analytics/template
paths when their typed owner replaces them. Retain source/domain definitions, current regression
obligations and product contracts rather than compatibility readers or historical runtime records.
After a validated replacement is selected, quiesce affected readers and remove obsolete generations,
runtime copies and rollback assets under ADR-0078. Current reconstruction is required; historical
runtime recovery is not a product obligation.

Phase 5 is the next serving boundary: generation-pinned requests, typed hydration, optional native
loading, ranking, bounded packets and thin MCP transport must be implemented and qualified together.
Catalog/member readiness remains independent of brief availability. PR6 remains paused until that
boundary exits. The forward plan retains the product sequence—contracts/options, scenarios/deployment,
selection/witnesses, agent tools and comparative tasks—without treating old pipeline formats or
receipts as qualification of the replacement.

> Decision: ADR-0071, ADR-0073, ADR-0076, ADR-0078, ADR-0081, ADR-0085
