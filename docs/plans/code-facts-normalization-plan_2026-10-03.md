# Code facts: correspondence and normalized characterization

**Proposed, 2026-10-03.** The [coordinator](code-facts-expansion-plan_2026-10-03.md) owns status,
finding disposition and combined acceptance. This plan develops C2–C6 and the identity foundation
for C11. It consumes owned provider observations from the [migration plan](python-analyzer-migration-plan_2026-10-03.md).
It does not decide runtime truth or analytic ranking. Enduring owners are facts/identity,
semantic-model Types/Signatures/Normalized, acquisition and catalog/product sections.

## 1. Existing foundations and target boundary

`lctx-model::domain::calls` already owns Signature, SignatureParameter shape/ordinal and call
application. `types` owns structural terms and occurrence-subject TypeObservation. Existing
normalized callable, public path and field operations should expand, not gain parallel authorities.
Current weaknesses are correspondence loss, source-only slots and repeated partial interpretation:

- Ordinary syntax parameters and their formal occurrences differ. Declarations/types attach to
  the formal; analytic code compares to the parent. Selection's existing formal use is correct.
- Source undecorated signatures cannot answer arbitrary decorated/generated callable questions.
- A dependency-only name index excludes analyzed root modules from import resolution.
- Class traits omit native metadata; record options are reconstructed for questions Pyrefly
  already answers. Located receiver/member/expected roles have selected consumers waiting.

The target operation sequence is observation attachment → normalized characterization → explicit
consumer policy. Every characterization retains origin, role, availability/coverage and support.
Resolved identity, inferred type, expected type and source text are separate facts, even when
their display strings match. New payload records are chosen by meaning; no requirement for one
relation or service per field. Executable declarations own column-level detail.

## 2. Source/formal/native-slot correspondence

N0 creates one model-owned correspondence operation using source snapshot/occurrence ownership,
syntax role, declaration symbol and existing SignatureParameter identity. It returns the source
parameter container, formal declaration occurrence, source callable/slot and descriptor receiver
role when established. Varargs may map to themselves; ordinary/default-bearing parameters do
not. Ambiguous/missing mappings return explicit unavailable correspondence, not a guessed parent.

Canonical relation support retains each source edge. Consumer lookup projects the formal/slot
required by its question. Migrate analytic parameter/receiver/type joins with P0 and keep declared
selection correct. Do not change TypeObservation.subject to the parent to satisfy broken joins.
After M2, native signature variants extend this operation with optional source-formal evidence;
the early repair is already the target, not a temporary shim.

Controls independently specify positional-only, keyword-only, ordinary with/without defaults,
varargs, kwargs and bound receivers. Assert expected formal IDs/names and receiver policy, not
just equality between two projections using the same resolver. Synthetic slots have no source
formal and still remain selectable and renderable.

## 3. Callable variants and typed slots

N1 normalizes callable identity plus role: source, effective-typed, synthesized, stub, documented
or observed evidence where those roles actually exist. Source/effective roles are not competing
runtime implementations. Variant identity includes context, role, origin and overload family;
shape retains positional/keyword/default/invocation information and descriptor adjustment.
Missing native effective type becomes Unknown with reason, never undecorated fallback or an
invented `(*args, **kwargs)` signature. Arbitrary decorator effects remain typing characterization.

Add a typed observation subject for signature parameters and return ports, using variant and
ordinal/return role. Keep source-located TypeObservation occurrence-owned. Slot types retain
declared/inferred/expected provenance as applicable; source correspondence is optional.
Defaults distinguish literal source value, generated/default-factory evidence and unknown runtime
value. Complete typed signatures cannot establish stable default object identity or factory purity.

Normalize overload alternatives individually and retain implementation body identity separately.
Candidate enumeration and chosen applicability are separate inputs. Pyrefly's closest index from
all-overloads is not chosen evidence; consume the explicit chosen trace. An unresolved overload
does not become an arbitrary first alternative, and a chosen typing alternative does not prove
which runtime implementation executes. Call binding/transfer admits a variant only through its
own policy and dispatch basis; the shared signature operation does not grant behavioral authority.

N1 includes the first catalog role rendering and P2b query contract integration; N2 and P1 expand
their consumers. Update §15.5/§14.4/§4.2.3 and route changed effective-callable decisions through
the ADR process. Delete source-only fallback/duplicated descriptor interpretation after consumers
move. Preserve declared ParameterType semantics; add explicit effective/synthesized queries.

## 4. Generic binders and substitution

N2 uses existing TypeTerm generic/ParamSpec vocabulary. Binder keys use declaring scope/native
origin, not the text `T`. Preserve TypeVar bounds/constraints/defaults, ParamSpec/TypeVarTuple,
Self, declared versus inferred variance and specialization witnesses. Normalized substitution
maps binder IDs to structural terms under a recorded provider/context basis.

The bounded substitution operation takes a term, binder environment and budget; it returns
resolved structure plus support, or a partial/recursive/unknown boundary. Recursive aliases,
unresolved ParamSpec and NamedInts stay explicitly qualified rather than string-parsed. Distinct
generic declarations called `T` never unify by spelling. Specialized member/signature types
retain the generic declaration and substitution origin; no loss of role when lowering a type.

`Box[T].get -> T` specialized to Box[int], a constrained/defaulted TypeVar, a ParamSpec wrapper
and a recursive alias are revealing controls. Catalog and explicit typed selection use the
specialized result; analytic policy may group declared structure separately. No general runtime
assignability engine is added. Structural equality/nominal identity retain their existing meaning.

## 5. Metadata, members and record options

N3 widens class/member characterization: MRO/metaclass, explicit/effective abstract members,
protocol/runtime-checkable, final declaration, enum members/values, slots, NewType, record and
transform options, inherited/synthesized/source members and deprecation message/origin.
Per-field basis distinguishes resolution, declaration and type-level computation. Missing fields
or incomplete class analysis do not certify absence of runtime attributes or abstract members.

One normalized record-option operation consumes native effective options for constructor/typing
questions and source options for source declaration questions. Preserve disagreement with
support. Retire `normalized/symbolic_fields.rs` reconstruction where its semantic question is
replaced; preserve distinct source/default facts and unsupported custom decorators. Generated
constructors use N1 slot identity and N2 specialization when generic. No second option parser
in catalog or analytics. Enum values aid option discovery; they do not close a heap model.

Metadata feeds catalog eligibility/characterization and declared analytic attributes. Runtime
dispatch closure belongs to B3: `typing.final`, protocol declarations and abstract flags cannot
independently close targets. A custom metaclass, inherited overridden field, transformed dataclass,
deprecated overload and incomplete abstract status exercise role/origin and uncertainty.
Deprecation text may explain a brief but cannot invent a replacement API.

## 6. Per-alias modules and public paths

N4a repairs current analyzed-root exclusion and uses existing provider resolution for each alias
where available. Module identity includes analyzed roots and dependencies under the importing
provider environment. This can land before M2 without a new parser. It must consume actual lookup
results: passing an assumed path to `import_handle` is not independent resolution evidence.

N4b expands namespace/bundled/unresolved candidates, relative aliases, re-export and wildcard
origin, explicit/partial/invalid computed `__all__` evidence. Normalize each public path with
shared entity identity and enumeration coverage. Known export entries can establish paths even
when the whole export set is partial; missing entries then cannot establish non-public absence.
Two aliases are two paths, not duplicate declarations or two analytic entity votes.

Module dependency projection, catalog aliases and original-example association consume these
records. Retire dependency-name-only resolution as the semantic owner, not merely add a second
index beside it. Fixtures include intra-root import, `from . import a, b`, namespace modules,
wildcard/re-export chains, circular imports and computed known-plus-dynamic exports. Cross-context
or unresolved lookup preserves candidate/unknown information, never path spelling guesswork.

## 7. Located/expected types and member access

N5 selects receiver/attribute base, assignment value, return and expectation locations used by
field/call/guard consumers. Do not persist every expression type without a named consumer.
Retain source expression, provider context, inferred/declared/expected role and coverage. An
expected type imposed by context is not an observed or inferred expression result.

Member normalization consumes located types and native declaration targets to return exact target,
typed candidates, descriptor/property access role, or unresolved status. Storage field identity,
property invocation and dynamic reads remain separate. `Any` produces no exact-field certificate;
a property and stored member with equal spelling do not share effect semantics. Behavioral
state/alias proof remains a later B3/local-fields admission responsibility.

Catalog field-access witnesses and explicit receiver/member queries are first consumers; B1
uses operand attachment, B3 uses candidate receiver identity, P1 consumes named type roles.
Controls cover property versus field, inherited override, method-defined field, Any receiver,
expected-type mismatch, wrong-source attachment and unavailable native tracing.

## 8. Packages, integration and evidence

| Package | Needed working input | Deliver and verify together |
|---|---|---|
| N0 | Current source/signature facts | Shared formal/slot mapping plus P0 migrations; parameter negative twins |
| N1 | M2 attachment/native callable payload, N0 | Role variants, slot/return types, first catalog/query integration; wrapper/opaque/overload/generated/bound-method cases |
| N2 | N1 variant/binder subjects | Bounded specialization plus catalog/typed query and analytic role inputs; distinct binders and recursive alias |
| N3 | M2 metadata, N1 constructor slots | Class/member/record interpretation plus catalog/P1 attributes; custom metaclass/source disagreement/default controls |
| N4a | Current module/import observations | Intra-root per-alias repair and path consumer; unresolved lookup control |
| N4b | M2 full exports/alias payload, N4a | Partial export/public path consumers and dependency projection; no absence from incompleteness |
| N5 | M2 located/member payload, N1 | Role-selected located/member normalization plus catalog/receiver/field consumers; property/Any controls |

All declarations/validators are model-owned and used by both publication and tests. Update
stage-input bindings, source dependency/context keys, coverage, preparation, store tables and
generated wire alongside each persisted change. Canonical deterministic keys cannot include
rendered text or provider-local integers alone. Test shuffled observation order and duplicate
support; retain disagreements rather than voting them away. Invalid role/cross-context reference
fails shared validation before publication.

Use targeted release tests and real disposable-PG round trips for persisted contracts. Schema
snapshots change only after reviewing diffs; state migrations in commits. No compatibility reader
or duplicate normalized mapper remains after consumer cutover. A source-only checkpoint is useful
without claiming richer variants implemented. Enduring documentation moves with actual adoption.

No local test is passed by authoring this plan. Q0 owns the assembled dual-profile catalog/query/
packet journeys and full gates. Better invocation coverage and discovery are Proposed benefits
from preserving variant/identity information; retrieval-quality improvement is unmeasured and
not required to close these contracts.
