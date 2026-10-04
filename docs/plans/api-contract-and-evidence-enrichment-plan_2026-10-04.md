# API contract and original-evidence enrichment

**Proposed, 2026-10-04.** Supporting plan for O01/O02/O04/O05/O06 in the
[coordinator](code-facts-analytical-enrichment-plan_2026-10-04.md), whose §7 owns all mutable
status and source-review dispositions. Baseline `dbabbe6904a3`; current contracts and pinned
Rust source are **Interface-checked, 2026-10-04**. Implementation and verification are **not_run**.
Read the coordinator for shared migration, qualification and operator-state boundaries.

## 1. Result and reusable foundations

These changes answer questions currently requiring agents to reconstruct meaning from several
packets: how two callable roles differ, which constructor input initializes a method's field,
which exact API use a diagnostic characterizes, how a public name reaches its declaration, and
which typing context an expression receives. Facts remain attributed; a source relationship,
contextual type or selected diagnostic is not runtime execution evidence.

Existing normalized callable variants, native/source slots, generic specialization, symbolic field
links, import/module assessments, C1 source characterizations and role-qualified TypeObservations
are usable inputs. Existing `GetOperation` exact-member/PublicPath selectors, opt-in sections,
`SectionPage`, `PageRequest`, original-evidence grants and generation leases supply the serving
envelope. New output fields change Rust-owned wire contracts even when request selectors stay the
same. Pure model operations decide meaning; PostgreSQL supplies declared rows and effects;
Python validates/transports the generated schema.

Relevant source entries:

| Owner / reader | Current entry |
|---|---|
| Callable roles and sole applicability binder | `domain/normalized/callables.rs`, `signature_applicability.rs`, `generic_specialization.rs` |
| Native signatures / traces | `domain/types/signatures.rs`, `cpg-extract/src/type_records.rs`, `source_usage_service.rs` |
| Source field relation | `domain/normalized/symbolic_fields.rs`, `catalog/evidence/symbolic.rs` |
| Selection hydration | `selection/inventory.rs`, `classification.rs`, `evaluate.rs`; PostgreSQL `generations/selection.rs` |
| Original diagnostics / uses | `catalog/evidence/characterization.rs`, `source_characterization_service.rs`, `source_usage_service.rs` |
| Imports and exposures | `normalized/links.rs`, `relation_normalization.rs`, `projection.rs`; public exposure/catalog identity |
| Expected location constraints | `domain/types/locations.rs`; `cpg-extract/src/type_records.rs` |
| Packet closure and delivery | `domain/serving/{requests,packets,mappings}.rs`; `generations/{packet_service,operation_sections,evidence_service}.rs` |

Paths under `domain` are relative to `crates/lctx-model/src/domain`; PostgreSQL services are under
`crates/lctx-postgres/src`. These are bounded entry routes, not a promise that every required input
is already hydrated. Implementing each package includes updating its exact declared inventories.

## 2. Common operation and packet rules

Inputs identify one captured input/context, member or exact source subject, selected roles and a
finite requested result domain. Native/member/parameter/slot/source IDs stay distinct. Results
retain input identities, support, condition/assumptions where applicable, explicit unresolved parts
and bounded canonical order. Unknown correspondence must not disappear from a page or become a
negative result. Charge pair/edge/type/member indexing, work, retained values and packet bytes;
reservations outlive the structures they account for.

Request-time operations borrow already admitted canonical rows from the pinned generation. Their
mappings name all source relations, supports, original occurrences and lookup paths, and
`PacketLease` enforces those reads. Do not hydrate every C1/facts family to avoid choosing inputs.
C4's shared diagnostic-use result and C2's trace association require canonical output declarations,
stage ownership, definition identity and shared production/replay. Query-local results require the
same pure semantic operation and deterministic controls but no new persistent comparison store.

Preserve existing invocation relationships when extending the bounded Relationships section.
Select a typed tagged relationship arm for source field links; its IDs cannot be represented by
merely placing a constructor in the current generic target-member packet. Add opt-in typed sections
for role comparison, contextual typing and access routes using the existing continuation contract.
No default operation selection expands into unbounded comparison or traversal. Original diagnostic
correlations use the existing evidence request and scoped output grants. Update native/Python/MCP
schema conformance in the same packages, not in a final UI-only cleanup.

## 3. C1 — Compare callable contracts by role

**Prerequisite:** current normalized variant/formal/default/type evidence. C2 is independent.
`lctx-model::domain::normalized` owns the comparison operation; serving consumes its typed result.
A request supplies member/callable, context/input and exact left/right variants, or an explicitly
bounded pair of roles. A role may have zero, one or many variants; never take the first “effective”
row. Refuse foreign member/context/source input before comparison.

The result identifies both variants/roles and reports per-dimension `Same`,
`DifferentRetainedStructure` or `Unresolved(reason, evidence)`: port presence/order/kind/name,
requiredness/default knowledge, receiver/descriptor adjustment, return/type structure and formal
correspondence. An exact shared source/native formal link establishes identity. Positional layout,
keyword-name/kind and collector alignment can compare interfaces, but must be labelled **layout**,
retain both port IDs and never invent a source formal. Receiver adjustment stays explicit; do not
blindly drop a leading parameter.

Compare structural type evidence only within its closure/support limits. Different terms are
retained differences, not incompatibility; opaque/truncated terms or unresolved binders remain
unresolved. Default Absent, Unavailable, Unknown, Expression, Factory and Literal remain distinct.
The sole existing binder still admits Source/Stub under its present contract; EffectiveTyped,
Synthesized or Specialized role comparison does not make those variants runtime-call authorities.
No cross-release substitutability or second assignability checker is introduced.

Hydrate variants, slots, exact source/native formal links, type supports/closure, adjustment and
origin evidence at the operation packet boundary. The optional comparison section and role-qualified
programmatic explanation use the same operation. Retrieval text may render the retained role facts;
if it uses comparison differences, it must call this operation with an explicit finite role policy
and include that policy/source in its text identity. Delete any renderer-local difference classifier.

Acceptance includes decorated ParamSpec source/effective differences, synthesized dataclass
constructors, bound/unbound receiver layouts, unknown default expressions, shadowed generic binder
identities, multiple variants per role and missing/partial terms. Compare before/after operation
answers against independently specified expected distinctions. Shuffled input and pages preserve
canonical pair ordering and request identity. Count/type/work/byte limits produce explicit partial
or refused sections. No compatibility or body claim is emitted.

## 4. C2 — Retain native overload origin before associating traces

**Prerequisite:** A1 guidance and C1's role/identity distinctions; the operation comparison can ship
without this package. Current `get_all_overload_trace` returns a closest index computed by Callable
equality even when selection failed. The chosen trace checks `is_closest_chosen`, but both public
paths erase the original target; `Type::Callable` lowering sets function identity unavailable.
Equal term/display/shape or a unique nearest index cannot recover the chosen member.

Exact Pyrefly source establishes a narrow observation opportunity: before trace erasure,
`lib/alt/overload.rs` carries `TargetWithTParams<Function>` and `FunctionKind::as_func_def_id`;
original targets can supply optional module/FuncDefIndex/class identity and an original call-local
input-vector ordinal. FuncDefIndex is source-definition identity, not a declaration overload ordinal.
`OverloadTrace` currently retains only Callable/tparams. Extend the minimally patched fork's
tracing-only seam to retain optional original member identity, call/site/run and original-vector
identity/ordinal, callable/TParams and actual selection state. The ordinal is query/run/vector-local;
original FuncDefId correspondence, not ordinal equality across calls, identifies a declaration.
It is not the old closest index. Selected-only `CalledOverload.table` can supply an observed
specialization table where available; merged matches deliberately have an empty/default table.
No per-candidate solved specialization or receiver basis is supplied by the current trace. Capture
such a basis only from explicitly identified original provider inputs when required for the selected
association; otherwise retain Unavailable rather than solving it again or guessing from TParams.

The union-expanded multi-match path retains a first-match representative. Capture exact matched
multiplicity/member membership, or explicitly mark representative/multiple selection unavailable
for unique association. Never call that representative the unique chosen member. Anonymous,
synthetic, missing-origin and erased-binder cases remain unavailable. This seam must not change
checker selection, diagnostics or inferred types. Follow pin-check/fork policy: record parent and
patch, update exact revision and pin rationale, rebuild affected crates and preserve embedded Ruff
isolation. No broad analyzer upgrade or inference patch.

Retain attributed typed origin observations at the exact call Arguments site with source/view/run/
context and support; keep candidates distinct even when structural terms coincide. Extend the
normalized association owner to reconstruct the admitted candidate set and compare exact native
origin, role, owner and specialization to normalized variants. Output `Unique`, `Ambiguous` or
`Unavailable` with all relevant IDs/reasons. Unique requires exactly one identity-established
member, not one matching shape. Missing required specialization/receiver correspondence remains
Unavailable even when an original source member is known; the packet can still show that source
origin separately. Chosen native typing remains separate from source-shape binding,
runtime target and body admission. Add canonical producer/replay inputs and output identity.

Source-use packets replace unconditional variant-unavailable wording only where the admitted
association establishes identity, preserving existing unavailable/ambiguous cases. Hydrate origin,
candidate, native declaration/variant and specialization premises under the same lease. Test
identical-shaped members with distinct origins, reordered families, failed/closest-only selection,
union multi-match representative, generic specialization, bound receiver, anonymous callable,
wrong Arguments site/view/context and tampered/dropped candidate origin. Compare native types and
diagnostics before/after the observation-only patch. A guessed unique association fails acceptance.

## 5. C3 — Consume source constructor-to-reader associations

**Prerequisite:** existing C1 `SourceFieldLink`, not another extractor. That result already links
field option, parameter option, normalized source association, exact reader and reader owner, with
source_association Known and runtime_value Unknown. Current Selection omits these rows from
`catalog_selection_inputs!`/`ClassificationData::Evidence`, and ConfigurationRelationship returns
unknown for Parameter/DeclaredParameter.

Implement the source relationship at the model Selection/catalog owner. Resolve the requested
constructor formal or native slot through existing exact correspondence to its CatalogOption,
then consume the C1 link for that exact class/member/context. Preserve separate formal and slot
targets if the existing target vocabulary cannot express them. Support the selected declared-source
parameter relationship; do not reinterpret current ExactStorage/ExactReader as runtime instance
value. Nonunique/unsupported correspondence stays unknown with reason. A false answer requires
sufficient scoped relationship coverage, never absence of a hydrated row alone.

Declare SourceFieldLink and the referenced SourceFieldAssociation, SourceFieldReaderLink/Reader,
option subjects, formals and relevant support in Selection inputs/classification/preparation.
Reuse the C1 table; do not emit another independent field link. Extend Relationships with a typed
source-field arm retaining parameter/field option IDs, association/reader/access/owner and original
premises. The operation's bounded section and option-relationship query agree because both consume
the same model interpretation. Retain the invocation relationship arm unchanged.

Use the review's metadata lead narrowly: inspect `has_base_any` and captured ancestor availability
for a constructor/member-universe uncertainty explanation. Existing missing-context evidence should
be used first. If native Any-base status distinguishes a real refusal, retain that attributed
metadata bit with the class evidence and a typed incomplete-base reason; test a fully captured
base versus Any/dynamic base. Keep known candidates and abstract_absence_known=false. Framework,
ordering and ORM flags without a changed selected answer stay deferred at coordinator §7.

Acceptance: separate timeout/title formals/readers, generated/inherited record, absent/ambiguous
formal, unrelated same-named class, mutation after initialization, alias store and separate instances.
Packet and predicate retain “source association, runtime value unknown” in every supported case.
Publication/source expansion rejects forged link premises; the query cannot bypass its declared
hydration. Test predicate unknown→supported only for the exact admitted link.

## 6. C4 — Correlate diagnostics with exact uses and scenarios

**Prerequisite:** selected diagnostics/run/settings/support plus existing SourceUsage and scenario
associations. Current `SourceCharacterizationScenario` means containment, while SourceUsage retains
an exact normalized event-source correspondence. Neither is the missing diagnostic-to-API relation.

C1 evidence owns a small reusable correlation assessment for each selected diagnostic/source
characterization in the requested captured domain and its admitted use associations. Define the
supported correspondence through original artifact/view/context and canonical occurrence roles:
resolve the diagnostic's supported primary range to an exact syntax/use subject and the existing
normalized event/source relation. Require a unique admitted subject or retain explicit candidate/
unassociated outcome. A span overlapping several calls, an enclosing scenario, matching displayed
symbol or secondary-only foreign source cannot establish member authority. Exact argument/child
correspondence may identify its containing event only through the canonical syntax role relation.
Keep member association separately resolved/candidate and preserve containment-only scenarios.

Produce typed assessment/link rows citing diagnostic premise/run/channel/settings, characterization,
use/event/source, alternative and API/scenario association. Retain unassociated reason/remainder;
no diagnostic with incomplete mapping vanishes. Bound occurrence/event lookup and fan-out with
charged indexes. Register C1 output/input inventories, definition fingerprint, shared operation and
exact-membership replay; reject missing/foreign associations and coupled output removal. This
result is reusable source evidence, not a new diagnostic verdict or example validity gate.

Hydrate the new result and its exact premises at `original_evidence`, including event source/
ownership rows where the current mapping is insufficient. Extend the typed source characterization
payload with bounded correlations, target resolution and remaining containment-only scenarios.
`GetEvidence` grants and `GetOperation` scenario evidence use this result; retrieval text includes
only the supported relevance note and exact channel/configuration. It never claims test execution.

Reuse current Ruff/Pyrefly emitted/suppressed/disabled/baseline distinctions and settings identity.
Explain selected suppressed diagnosis versus rule not selected versus unavailable analyzer coverage.
Add original directive span only when the selected suppression explanation needs it and exact
supplier semantics can attach it; range suppression/full-range and start-only cases differ. Empty
selected diagnostics proves neither validity nor a clean example. Noqa classification is not a
complete inventory of directives/fixes; keep unmatched baseline status explicit.

Acceptance: resolved use and candidate target, unrelated same-spelled call, diagnostic outside the
grant, missing/foreign primary/secondary range, ambiguous enclosing span, duplicate diagnoses,
matching/wrong-code noqa and partial range suppression, disabled rule, changed target/config,
parse failure and expected-exception/skip source intent with execution not_run. Exercise production,
replay and actual PostgreSQL evidence service; page/truncate correlations without losing remainder
status. Removing a link or changing target/channel/config must not retain a supported correlation.

## 7. C5 — Explain public import/re-export routes on demand

**Prerequisite:** captured module universe, exact public exposure/member and existing aliases,
ImportModuleAssessment/Candidate, module-resolution supports and typed ImportReference projection.
The current access packet retains exposure/candidate IDs but no ordered route. Direct joins establish
each admitted edge before traversal; serving SQL is not an export interpreter.

A model-owned request-time operation takes the exact selected public path/member, captured context
and finite route bounds. Build only relevant supported alias/reference edges, retaining direction,
original alias, target module, native resolution assessment/reason and exposure/candidate identity.
Keep parallel Import and Reference meanings separate. Enumerate bounded canonical routes to the
declaration, with explicit ambiguity, cycle, unresolved edge, omitted module and frontier limits.
Closed search applies only to the declared captured-module universe; it does not prove absence of
external consumers or installation requirements. Wildcard candidate membership is not definite
binding, and source/stub providers remain labelled.

Hydrate the exact inputs in access-route mappings and serve an opt-in typed route section using
existing GetOperation/PublicPath and continuation. Do not persist a general dependency graph or
precompute every possible path. Use existing petgraph/SCC traversal only where cycles need it;
charge graph/path/resolution storage and work before allocation. A page cap must not masquerade
as a complete route set. Original alias evidence remains independently expandable.

Inspect per-name ty StarImport placeholders only for an unresolved wildcard fixture where current
alias/module evidence cannot explain the uncertainty. If they add a useful reason, retain scoped
module/symbol/definition-state attribution in the existing flow/explanation lane; never let it
resolve exports or replace Pyrefly candidate membership. Catalog cannot start requesting Flow just
to enrich this optional reason. Otherwise record the lead's non-fit and current explanation.

Acceptance: explicit alias/re-export, same-named different targets, parallel edge roles, cycle,
partial/computed `__all__`, unresolved importer, omitted module, stub/source and supported direct
path. Invalid direction/support/view/context and fabricated wildcard certainty are rejected.
Explain setup/dependency metadata separately if a later installability journey is selected.

## 8. C6/C7 — Explain contextual types, then justify argument supply

### C6 — Existing supported locations

Current Expected extraction/validation supports selected attribute, assignment and return value
children. Add one model-owned contextual explanation at those locations using exact subject/view/
context/run and actual/Expected observation, structural term closure and supports. A result states
which role evidence exists, which part is unavailable and whether exact retained structure differs.
It does not decide assignability, infer a declared formal type from Expected or assert a runtime
violation. Equal displays do not establish equality; unequal structural terms are not incompatibility.

Expose the result as opt-in contextual typing on an existing source/evidence/operation journey.
Hydrate exact location/ownership and term closure under the packet mapping, not merely existing
signature-only type rows. Preserve trace channel, error-recovery status when observable and candidate
basis; unknown recovery status stays unknown. Missing trace is unavailable, never unconstrained.
Reuse C1 role/term presentation helpers without mixing their semantic questions.

### C7 — Conditional argument extension

After C6, compare a selected empty-container or overloaded-call explanation against existing native
slot/signature and actual argument packets. Query the exact pinned Pyrefly Expected trace at the
argument expression in a focused extraction fixture. Require a retained trace that improves the
answer and can be attached to the canonical argument/location/source view; otherwise record no
useful supply and defer argument expansion with a concrete consumer/trace trigger. C6 remains
required, and absence of argument Expected is not a defect.

When admitted, extend location validation and producer together with an explicit argument-context
role/basis. Append vocabulary codes; do not reorder TypeRole. Retain actual and Expected IDs plus
call/event/argument and native candidate trace association when known. C2 is required only for a
claim naming a uniquely selected original overload; a contextual trace may still be shown with
ambiguous/unavailable candidate origin. Erroneous-call/closest hints remain attributed context,
not successful typing choice. Add source attachment, support, stage/schema inputs and the C6
consumer in the same slice. Never emit an argument Expected row rejected by old location rules.

Acceptance: contextual empty container, exact assignment/return evidence, missing trace, erroneous
call, overload ambiguity, wrong parent, wrong argument role, same span in another view/context,
truncated/opaque term and optional unique native candidate. Native service/MCP packet preserves
availability and basis; no renderer performs textual compatibility. C7's branch decision belongs
at coordinator §7 and must identify the actual captured fixture and changed answer.

## 9. Integration and acceptance route

C1/C3/C4/C5/C6 can start from current facts. C2 requires the actual observation seam and all its
attribution consumers. C7 waits for C6's useful consumer, not for unrelated graph/kernel work.
All touch serving packets/mappings and several touch stage/type declarations; appoint one integrator
for those files while independent semantic modules/tests proceed. Avoid concurrent production
edits to the same tree/files. Shared ownership is an editing constraint, not a semantic dependency.

Each package includes focused model, extractor where changed, serving-contract and disposable
PostgreSQL service tests. Existing test routes include `domain_catalog_evidence`,
`domain_selection_catalog`, `domain_types`, `typed_types`, `native_diagnostics`, `native_exports`,
`serving_contracts`, `packet_layout` and PostgreSQL `services`. Use release-profile targeted tests
and compile checks while implementing, adapting the precise filters to actual test names.
The coordinator Q1 owns assembled fixture journeys and full `just test-all`/`just hygiene` once all
functional scope is integrated. This plan does not require a gate per section or real-library pilot.

Model/wire snapshots, codebooks, input inventories, analysis/mapping identities and stale adapter
rejection migrate together. Delete independent packet classifiers, shape-only overload matching,
redundant join outputs and speculative general graphs. Existing unavailable branches stay when
there is no supported replacement. Enduring guidance moves to semantic-model, acquisition/extraction,
product and serving owners with accurate evidence labels; accepted historical decisions stay intact.
