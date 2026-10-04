# Pinned provider capabilities and additional analytical uses

**Interface-checked, completed 2026-10-04; review started 2026-10-03.** Supporting supplier audit for the comprehensive design/target
review. The principal reviewer owns independent architectural judgment. This document identifies
observed supply, current consumption, opportunities and material integration limits; an unused
capability is not automatically a defect. Proposed uses below are not accepted architecture.

Baseline: `main` `efe0c24aa72114914849bb29528ce0436ab67665` with the coordinator's 231 tracked
dirty changes, patch SHA256 `54270067f1e57b40ad3a4aa7e42565e37b38a9643be371ec1b859c97b0bbe61c`.
No production, shared-skill, pin or configuration changes. No probes/builds/network requests.
Applicable evidence vocabulary is core 3.2 §D and code-intelligence profile 1.3. Existing receipts
retain their original dates/scopes and do not establish Q0 for this tree.

## 1. Supply identity and inspection boundaries

[Pins](../../../pins.md#linked-rust-libraries), root `Cargo.toml:55–81`, the checked-out primary
sources and the two patches establish the relevant nominal families:

| Supply | Exact identity inspected | Access boundary |
|---|---|---|
| Pyrefly | 1.4.0-dev.3; fork `72bb34d6d67c2bc14720c77e2ad7eff6b89d360f`; parent/tag `80cec3f57364bc11d4a39a419f6894a8eabcaa00` | Linked session/Pysa/type APIs plus observational fork seams; embedded registry Ruff 0.0.14 stays local to its adapter |
| Independent Ruff/ty | Ruff 0.16.10, mostly crates 0.0.16; fork `f7bdff69e1fb94ab0ed5b340e977aac0d26e9301`; parent `3265ed1f944c98bb4c04d632fbefb1257cdb583d` | Latest canonical syntax, populated Checker facts, explicit-source lint settings; ty core symbolic index and precision observation |
| salsa | salsa, salsa-macros and salsa-macro-rules 0.28.5 | One latest-family graph; embedded Ruff's salsa feature stays disabled |

Local source roots inspected:

- `/home/paul/.cache/lctx-code-facts/pyrefly-fork/source` (`git rev-parse HEAD` matched the fork above).
- `/home/paul/.cache/lctx-code-facts/ruff-fork` (same check matched the fork above).
- Repository `third_party/pyrefly-1.4.0-dev.3.patch` and `third_party/ruff-0.16.10.patch`.

Source paths below starting `pyrefly/lib/` or `crates/ty_*` name those exact trees. External links
name the immutable upstream parent when the interface is unchanged by the patch. New seam
semantics come from the repository patch and fork source, not an upstream-only page.
`ruff_*_latest` and `ruff_*_ty` aliases use the same git source; equal crate versions from another
source are not interchangeable AST types. `scripts/check_family.py:30–43,97ff` expresses the
source/salsa/dependent-family policy; its current run is **not_run** in this audit.

This is a current-code review, using the approved exact-source route. Context7/current-main
documentation is not used to certify pinned Rust embedding contracts. Offline skill briefs are
navigation and recorded evidence; the consequential interfaces below were re-read in source.

## 2. Finite breadth reconciliation

The prior [inventory](../2026-10-03_code-facts-expanded-target/coverage-inventory.json) has
26 areas, 137 concepts and 340 provider facts (132 Pyrefly, 93 Ruff, 115 ty). Its 41 recommended
payloads, 211 competing/existing payloads and 88 deferred payloads are **prior selection
dispositions**, not current extraction counts. The [consumer ledger](../2026-10-03_code-facts-expanded-target/consumer-ledger.md)
already refined some supplier dispositions: notably C12 selected diagnostics despite the
inventory's generic offline-only classification. The five-plan series is therefore not literally
an implementation of every old `recommend_payload` entry and nothing else.

All 137 concept IDs and all 340 fact dispositions were enumerated for reconciliation. Areas are
grouped below where the same consumer rationale applies; no new independent semantic guarantee
is asserted for every fact. Deep source inspection covers §3–§5, rather than every parser,
formatter, IDE, framework or runtime implementation. Counts below are facts' primary concepts,
so facts cross-listed under other concepts are counted once.

| Area | Concepts / facts | Current audit route and limit |
|---|---:|---|
| source | 4 / 13 | C1 immutable bytes, source/view correspondence; notebooks and PEP723 remain consumer-triggered, not a new corpus claim |
| lexis | 3 / 10 | Retained parser tokens/positions support extraction; trivia/style/editor transformations not independently re-audited |
| syntax | 8 / 24 | C1 latest Ruff parse, errors and exact role/kind attachment; no generalized literal/regex/codegen product selected |
| directives | 3 / 12 | C12 selected diagnostic channels use suppression context; no blanket TODO/pragma/fix system |
| environment | 5 / 26 | Explicit captured version/platform/root/config; builtin/stdlib tables are adapter alternatives, not new library capabilities |
| imports | 5 / 13 | C5 per-alias native module identity; file import graphs/dependency metadata remain distinct from runtime dependency |
| exports | 5 / 12 | C5 computed/partial exports, re-export/public paths; other provider export maps are corroboration alternatives |
| scopes | 9 / 26 | C1 lexical binding/context and C7 capture timing; lexical resolution is not ty may-reach nor heap identity |
| references | 4 / 13 | Final Ruff local references/Pyrefly symbols; ty semantic reference kinds and alias policy deepened in §4 |
| control | 6 / 13 | C8 reachability and C9 terminal use; no analyzer CFG/completion guarantee inferred from a survey row |
| conditions | 5 / 10 | C8 narrowing/precision/typed premises; ty semantic truth/type queries are competing observations (§4) |
| dataflow | 7 / 5 | Literal/type evidence is bounded; points-to, effects and taint need independent analyses, not relabelled type facts |
| types | 9 / 21 | C3/C6 structural type terms and contextual traces; ty second opinion and missing selected locations in §4–§5 |
| classes | 6 / 17 | C4 metadata/MRO/members; public ty access versus private enumeration in §4 |
| callables | 5 / 13 | C2 roles/overloads and C3 generic slots; returned hint is not applicability (§4) |
| calls | 8 / 16 | Existing Pysa target/origin and model binder; ty call-site mapping is an oracle/characterization alternative |
| exceptions | 3 / 5 | C10 exact raised-type/handler operations use existing input; no whole-program propagation supply added |
| diagnostics | 8 / 28 | C12 selected Ruff/Pyrefly original evidence is current; baselines/config/fix/metric breadth is not full diagnostic coverage |
| docs | 4 / 9 | Existing docstrings plus deprecation; IDE-formatted parameter documentation is an alternative input, not a fact authority |
| project | 4 / 7 | Configured file-set and fixture/test characterization; no unreferenced-definition absence claim |
| editor | 9 / 19 | Native semantic routes examined where useful; no editor transport, rename, server or completion product qualification |
| frameworks | 3 / 7 | C4 typed record metadata and C12 local pytest answers; ORM runtime and tensor shape semantics not certified |
| runtime | 5 / 6 | Dynamic attributes/imports/exec/monkeypatch/execution remain explicit support limits |
| concurrency | 3 / 4 | C9 exit/protocol observations; no scheduling/thread/resource lifetime analysis from async flags |
| identity | 5 / 11 | Model IDs/source identity versus invocation-local analyzer IDs; incremental mechanism is not a product relation |
| native | 1 / 0 | No modeled extension supply established by this finite index; not proof of upstream impossibility |

This preserves complete finite survey breadth without certifying 340 semantics. The index is
not an exhaustive list of source functions; unindexed helpers can still matter. Current usage
labels are revalidated below, rather than imported from the skill's historical project overlay.

## 3. Selected 41 payload routes against current code

The groups below account for all 41 old selected fact IDs. They identify current concrete
producer/first-operation paths where inspected. C10/C11/C12 also have existing/new subpayloads
outside that shortlist. Implementation evidence does not establish runtime acceptance.

| Prior selected IDs (prefixes retained) | Current producer → model first interpretation → output |
|---|---|
| `ruff.bindings`, `ruff.qualified-names`, `ruff.semantic-context` | `ruff_context.rs` + `ruff_lexical.rs` retain active node and final binding/reference roles → `normalized/native_lexical.rs:95ff`, `decorator_identity.rs` → reference/declaration characterization, typed decorator attributes, identity-aware recognition (C1) |
| `ruff.argument-lookup`, `ruff.builtins-tables`, `ruff.stdlib-modules`, `ruff.typing-constructs`, `ruff.side-effects`, `ruff.truthiness`, `ruff.static-branch-classification` | These are supplier/helper alternatives, not seven new extracted relations. Scoped search of `cpg-extract/src` and `cpg-flow/src` found no direct use of those helper names/stdlib crate; static branch facts are explicitly obtained from Pyrefly `SysInfo` through `native_branches.rs`. Current syntax/call/typing lowering has its own owned operations. Any replacement needs exact equivalent semantics and a consumer, particularly side-effect/truthiness helpers which do not prove runtime behavior. No defect follows solely from this difference (C1/C8) |
| `pyrefly.function-signatures`, `pyrefly.overloads`, `pyrefly.decorators`, `pyrefly.generics` | `type_records.rs:1475–1760` reads native function/class-field candidates, source parameter/return answers, effective/synthesized/overload variants and structural generics → `normalized/callable_normalization.rs`, `generic_specialization.rs`, `signature_applicability.rs` → role-explicit catalog/selection and qualified source-use applicability (C2/C3). Canonical declaration decorators also come from latest syntax |
| `pyrefly.class-metadata`, `pyrefly.abstract-members`, `pyrefly.class-fields`, `pyrefly.class-members`, `pyrefly.deprecation` | `type_records.rs:1815–2256` reads metadata, abstract/protocol members, MRO owner contexts, inherited/synthesized fields, record options and deprecation → model class/member/record identities and normalized field/callable interpretation → analytic class traits, constructor/field facets, source characterization (C4/C6). `abstract_absence_known=false` explicitly avoids unsupported negative |
| `pyrefly.import-resolution`, `pyrefly.export-map`, `pyrefly.dunder-all` | `public_records.rs` and `pyrefly_stage.rs` use native import handles/export locations plus canonical aliases → `normalized/relation_normalization.rs:264ff` and entity/public-path operations → C5 catalog visibility and target correspondence; dynamic/partial export enumeration is retained |
| `pyrefly.expression-types`, `pyrefly.expected-types`, `pyrefly.literal-values` | `type_records.rs:1590–1810` supplies declaration/argument/result/test and selected contextual traces, structural literals/type terms → location validation, operand attachment and question-specific typing admission → C6 facets/C8 conditional operations. Expected locations are intentionally finite, not all contextual traces (§5) |
| `pyrefly.captured-variables`, `pyrefly.mutable-captures`, `ty.closure-snapshots` | `capture_records.rs`, `cpg-flow/lib.rs`, `ty_flow.rs:1234ff` retain captured identity, mutation/timing, enclosing candidates and precision → `domain/captures.rs`, flow-capture and execution capture operations → dependence explanation and bounded qualified transfer (C7) |
| `ty.live-bindings`, `ty.narrowing-constraints`, `ty.places`, `ty.predicates`, `ty.reachability-constraints`, `ty.scopes`, `ty.use-def` | `cpg-flow/lib.rs`, `narrowing.rs`, `predicate.rs` now lower scope/place/use/binding, reachability and narrowing independently → `ty_flow.rs:1408ff`, atom decisions and basis-labelled condition operations → C8 conditional summaries. The old claim that narrowing evaluator is uncalled is obsolete; precision loss survives separately from a terminal true formula |
| `pyrefly.call-targets`, `pyrefly.receiver-dispatch`, `pyrefly.higher-order-parameters`, `pyrefly.implicit-calls`, `pyrefly.context-managers` | `call_records.rs:206ff,366ff,418ff` retains Pysa alternatives, higher-order routes and artificial origins; `protocol_records.rs:191,269` calls the new native context-exit/terminal seams → model dispatch/protocol/completion operations with basis → C9 source-use and terminal/exit explanations. Exit observations are not inferred merely from Pysa origin or Never result |
| `pyrefly.pytest-fixtures` | `parameter_definition_records.rs`/native definition answers → `catalog/evidence/characterization.rs:244ff` and `source_characterization_service.rs` → C12 located local fixture/parameter evidence, not fixture injection/execution |

Further current subpayloads: `diagnostic_records.rs:106–338` selects Ruff F821/F401/F841 with
emitted/suppressed channels and Pyrefly ordinary/directive/suppressed/disabled/baseline groups;
`catalog/evidence/characterization.rs:170–244` interprets these as original source evidence.
`source_usage_service.rs:427–486` serves supported chosen/candidate overload traces, explicitly
marks normalized variant identity unavailable, and does not equate a term with a variant ID.
C11 consumes normalized identities/roles through `analytics/native_attributes.rs`; it does not
need another raw provider for every attribute.

## 4. ty beyond flow: available interfaces, real limits and viable uses

Current manifests link `ty_python_core` for `cpg-flow`, not `ty_python_semantic`, `ty_ide` or
`ty_project`. `cpg-flow/src/db.rs` implements source/resolver/core `Db` over captured virtual files
and vendored typeshed. The following are therefore **current extraction gaps or independent
alternatives**, not stored-but-unused ty semantic rows.

### 4.1 Call arguments and signature alternatives

Primary interfaces: [call_signature_details](https://github.com/astral-sh/ruff/blob/3265ed1f944c98bb4c04d632fbefb1257cdb583d/crates/ty_python_semantic/src/types/ide_support.rs#L1112),
`CallSignatureDetails` at line 929, and `resolved_call_signature` at line 1558. Details expose
definition, argument-to-parameter mappings, displayed-parameter mappings and specialized
parameter types. The AST must be from that ty database's parse; foreign latest-Ruff syntax
is joined after observation, not passed as a substitute AST.

**Important semantic limit:** `call_signature_details` ignores type-checking errors to retain
IDE details. `resolved_call_signature` explicitly falls back to arity-based matching when
every overload fails type checking (lines 1593–1610). `find_active_signature_from_details`
chooses the most matched mappings if none matches fully (lines 1515–1550). Neither a nonempty
candidate list nor the returned single hint proves admissibility. The private
`resolve_single_overload` (1169ff) has a stricter uniqueness contract but is not an external API.
`call_argument_forms` is public but targets IDE classification of selected known type-form
arguments; its source TODO explicitly says ordinary declared annotations are insufficient.

**Proposed first use:** an independent, bounded differential oracle for the existing model
binder on concrete source calls, or an attributed call-site mapping explanation. Inputs are
exact source/call, configured environment, candidate signature role and receiver; model
normalization owns formal-slot/callable correspondence. Output is mapping/candidate disagreement
or an unavailable/candidate explanation in source-use evidence. It must not bypass
`normalized/signature_applicability.rs:134ff`, become a second binder authority or infer runtime
target closure.

Controls (proposed, **not_run**): valid annotated overload with unique match; same arity but
wrong type (a returned hint remains nonproof); two compatible overloads; missing required
parameter; `*args/**kwargs` unknown; bare ParamSpec synthesized display slot versus native slot;
bound receiver versus unbound source shape. Alternative: existing model binder plus Pyrefly
chosen/all trace evidence avoids a second inference stack; direct ty checks are an oracle
only when their environment and role differences are classified.

### 4.2 Expression types and contextual expectations

[SemanticModel/HasType](https://github.com/astral-sh/ruff/blob/3265ed1f944c98bb4c04d632fbefb1257cdb583d/crates/ty_python_semantic/src/semantic_model.rs#L1008)
are public; `HasType` documents possible panic for a node from another file. Inference uses
ty's node-index/scope identity (1032ff), not a range-keyed query accepting another parser's node.
Types retain distinctions such as Dynamic/Unknown, Never/Divergent, unions/intersections,
truthy/falsy and literal values. Many payload types and operations remain private despite
the public enum; display text is not a substitute structural type algebra.

**Proposed use:** selected offline cross-checks for inferred returns, literals, operand types
or transformed signatures, with one normalized disagreement classification. Inputs include
the original typing view and explicit environment; a runtime TYPE_CHECKING-renamed flow view
is a different question. First result is corroboration/disagreement with provider provenance,
not replacement of Pyrefly `TypeTerm`. Output can improve the explanation of uncertainty and
oracle coverage. If eventually admitted behaviorally, a distinct ty typing premise and
question-specific admission are necessary.

Controls: explicit Any versus missing annotation Unknown; annotated `int` versus inferred
Literal; narrowed use versus declaration; unreachable branch; string annotation with its
own scope; source role mismatch. `expected_string_literal_completions` is public, but semantic
`Db::is_open_file` controls collection of expected string-literal types (`db.rs:35–38`). An
editor-state-dependent completion query must not silently become deterministic catalog data.

### 4.3 Classes, members and type-level attribute lookup

[static_member_type_for_attribute](https://github.com/astral-sh/ruff/blob/3265ed1f944c98bb4c04d632fbefb1257cdb583d/crates/ty_python_semantic/src/types/ide_support.rs#L460)
returns the descriptor object type without invoking the descriptor, corresponding to
type-level `getattr_static`. `SemanticModel::attribute_completions`/`scoped_completions` return
public `Completion{name, ty: Option<Type>, builtin, is_type_check_only}`; these are candidate
members, not an exhaustive owned runtime field table. Full `all_members` is crate-private
(`types/list_members.rs:930`), and public member structs have crate-private payload fields
(704–727). The current fork does not expose those internals.

Public hierarchy functions return direct bases/subtypes with file/ranges (ide_support.rs:
1833–1914); subtypes are restricted to the caller-supplied module list, and an invalid target
also yields empty. This is not a complete MRO, whole-world subtype set or negative proof.
The skill's MRO fallback `[C, Unknown, object]` differs from Pyrefly's recovery-prefix model;
that recorded source contract should be reread if selected for an oracle.

**Proposed use:** compare descriptor identity and value type separately for a property/member
question; bounded subtype/member oracle for catalog constructor/invocation support. First
interpretation stays in model member/receiver/dispatch ownership. An output might explain
“static descriptor candidate” versus “typed access result,” or cross-check a captured direct
hierarchy. Existing Pyrefly structured members/MRO and source field origins are the lower-coupling
alternative. No reason to introduce a parallel class metadata authority merely for completions.

Controls: property descriptor versus property result; classmethod/staticmethod receiver;
inherited override versus defining class; inconsistent MRO; TYPE_CHECKING-only member; dynamic
`__getattr__`; missing module in subtype universe. Empty/unnameable outputs remain unavailable
or qualified candidates, never complete absence.

### 4.4 References and definition navigation

[find_references](https://github.com/astral-sh/ruff/blob/3265ed1f944c98bb4c04d632fbefb1257cdb583d/crates/ty_ide/src/find_references.rs#L14)
takes a project database, program file, byte offset and include-declaration flag; output retains
`ReferenceKind::{Read,Write,Other}`. `references.rs:45–88` explicitly resolves import aliases
for References modes but preserves them for rename/document highlights. Parameters may have
cross-file references via keyword labels, and pytest fixture exposure is part of the source
search. This is richer than an undifferentiated textual occurrence list.

**Proposed first use:** a bounded “where is this API/member/parameter referenced?” evidence
journey or independent local-read/write assurance. Inputs are captured project/file-set,
queried definition/offset and alias/declaration policy. Model-owned entity correspondence
interprets occurrence candidates into incoming-use evidence; this is distinct from normalized
call events/Pysa call graphs. Output could surface non-call attribute/decorator/import/keyword
uses with source citations. Current original source-use packets focus on normalized call
events; their success does not establish a complete semantic reference product.

Controls: same-spelled shadowed local; import alias preserved versus resolved; definition
included/excluded; keyword label; decorator/member read; unreachable/typing-only use; augmented
assignment; omitted captured module. No result proves absence of external/dynamic consumers.
Lower-coupling alternatives are final Ruff local lexical facts, Pyrefly in-process definition
answers/Pysa references, or bounded Glean/CLI oracle output. Public semantic helpers
`definitions_for_name`, `definitions_for_attribute`, `map_stub_definition` avoid `ty_project`
for definition queries but do not supply project reference search by themselves.

### 4.5 Integration and parity costs

Semantic `Db` extends current core `Db` with rule selection, lint registry, analysis settings,
program-file mapping, version provenance, dependency metadata, open-file state and cloning
(`ty_python_semantic/src/db.rs:10–40`). A custom explicit captured-input database is viable;
`ty_project::ProjectDatabase` is another option, but its discovery/environment defaults must
not become ambient input. IDE `Db` adds project/uv environment ownership. Several IDE/project
crates are unpublished and require matching git source; no new version/pin is justified by
these existing public APIs alone.

Scope-query work and project reference traversal must share the attempt's allocation/work
budget, cancellation and declared file universe. Persist model IDs and owned observations,
not salsa handles/lifetimes. Existing ty precision and Ruff/Pyrefly seam receipts do not prove
semantic Db/IDE inference, type conversion or project-wide cost/parity. Those questions need
selected exact controls before a **Tested** claim, not blanket full-stack qualification now.

### 4.6 Existing core supply: per-name star imports and definition-state queries

These routes do not require adding ty inference merely to expose their structural payload.
`ty_python_core/src/builder.rs:4150–4218` creates a separate StarImport definition and
placeholder for each exported name of a resolved source module. The public tracked predicate
contains importing file, symbol ID and referenced file (`predicate.rs:395–424`); its doc comment
explains why each name's actual boundness can differ. In the semantic layer, evaluation checks
explicit `__all__` and the imported symbol's boundness (`reachability.rs:2033ff`); membership
and definitely-bound/possibly-bound status are different questions.

Current `cpg-flow/lib.rs:752` retains StarImport as a definition kind, but `predicate.rs:515`
lowers its placeholder with other unsupported predicates to an opaque unavailable evaluation.
`ty_flow.rs:865–890,964–978` then emits NativeUnavailable and declines a qualification needing
that evaluation. This is an explicit support boundary, not a demonstrated false negative or
absence claim. Pyrefly already supplies each wildcard set in `pyrefly_stage.rs:2431–2483`;
`public_records.rs:111–155` uses actual native export data, partial-known `__all__`, computed
status and fallback candidates. The old skill narrative that computed exports are merely
dropped and that native unresolvable status is uncalled is stale.

**Proposed additional subpayload:** preserve the star placeholder's per-symbol/module identity
for an explanation of why a name's provider flow remains conditional. Model import/source
correspondence owns its interpretation; output is a located import-binding candidate with an
explicit boundness limit. Do not treat Pyrefly's wildcard membership as the truth of ty's
predicate or introduce a second authoritative export set. If actual predicate decisions are
needed, select their provider/view/premises explicitly. Controls: two differently conditional
names in one exporter, a preexisting importer binding, an explicit underscore name in `__all__`,
unresolved module, dynamic `__all__`, and same name imported from different modules.

`UseDefMap::definitions_with_usage` (use_def.rs:984) yields definition plus a used Boolean,
including standalone declarations but excluding combined definitions' early declaration part.
Current `cpg-flow/lib.rs:519` calls this API but discards that Boolean. Thus definition enumeration
is **used**, while the usage subpayload is **not retained there**. Public
`end_of_scope_bindings`/`reachable_bindings` (1117/1149),
`bindings_at_definition`/`declarations_at_binding` (1210/1221),
`imported_final_candidates_at_binding` (1235), and all-end-of-scope/all-reachable iterators
(1335ff) support different queries. Scoped search found no direct calls to these named APIs in
current cpg-flow beyond the separately used per-use and enclosing-snapshot lanes. This is not
a complete extraction deficit: per-use reaching sets, loop expansion and snapshots already
cover substantial current behavioral demand.

**Proposed use:** bounded explanation of an overwritten/unused declaration or end-of-scope
candidate, governed by lexical/source evidence or capture normalization. Inputs are scope,
place, definition, view and reachability/narrowing; first result is used/unused or possibly
bound characterization in that scope, not runtime variable liveness, dead-code proof or
absence of external references. Controls: annotation-only declaration, deletion, conditional
assignment, overwrite before use, use before later overwrite, loop-carried state and lazy
capture. Imported Final candidates need inference to determine Final: their name explicitly
does not establish declared type/finality. Existing Ruff selected unused diagnostics may be
enough for a source-evidence consumer; do not add the Boolean solely to duplicate F841.

## 5. Pyrefly and Ruff: additional subpayloads versus better first use

### 5.1 Contextual expected types and overload evidence

Primary source `pyrefly/lib/alt/answers.rs:1632–1687` exposes range-keyed actual/annotation/
expected traces, property-getter traces and chosen/all overload traces. `get_all_overload_trace`
returns the closest candidate index even when the candidate is not chosen; the adapter
correctly discards this index (`type_records.rs:1748ff`). `get_chosen_overload_trace` separately
checks `is_closest_chosen`. The native Arguments range is joined through its unique canonical
child rather than assuming the full call expression range.

**Current extraction gap:** expected types are queried only for selected `SyntaxField::Value`
children of attribute/assignment/annotated-assignment/return (type_records.rs:1773–1790).
Argument values receive actual type traces, not an expected-type query. `types/locations.rs:61ff`
also restricts Expected to those selected parent shapes. Generic TypeObservation storage is
not a semantic consumer for every possible contextual role. Scoped searches of normalization,
analytics, selection and execution found no explicit Expected first-operation dispatch beyond
location validation; this does not exclude generic projections/read APIs.

**Proposed extension/use:** for an invocation/example explanation, query native expected type
at a concrete argument expression (only when trace exists), then let the model compare selected
actual and expected roles as contextual evidence. Output is “this call expects this contextual
type under this typed candidate” with exact source and coverage, not a declared formal
parameter type or runtime violation. This needs an argument-context role/location contract and
overload/candidate basis; absence of a trace is unavailable. Current selected expected rows
could first acquire an explicit use before expanding extraction. Negative controls: contextual
empty container, overload ambiguity, missing trace, wrong parent, same-range different view,
expected type from an erroneous call. Alternative: declaration/native slot types already
support many correct-use questions without new contextual extraction.

**Better use of existing overload rows:** source-use packets retain terms but explicitly mark
variant identity unavailable (`source_usage_service.rs:466ff`). A model-owned unique association
could improve “which overload does this example illustrate?” only if native callable shape,
origin, specialization and normalized variant correspondence establish it. Equality of display
or term alone is insufficient; identical-shaped variants and closest-not-chosen are negatives.
No automatic type-trace-to-effective-invocation promotion is proposed.

### 5.2 Class metadata and descriptor observations

Current extraction consumes a substantial metadata slice, including defaults, transform
classifications, abstract/protocol members, inherited field origin, slots and deprecation.
The public metadata still offers `keywords`, `has_base_any`, framework classifications,
`is_total_ordering`/metadata, local disjointness/dataclass slot request and capture-init names
(`pyrefly/lib/alt/types/class_metadata.rs:327–491`). These are additional supply subpayloads,
not proof that current class/member semantics are defective.

**Proposed demand-led use:** `has_base_any` and unavailable ancestor/member contexts could make
a constructor/member-universe refusal more explanatory; inputs are native class evidence and
captured MRO contexts, model class/dispatch ownership produces a typed uncertainty reason,
and output retains “known candidates, incomplete base.” Dynamic/Any base and fully captured
exact base are negative/positive controls. Framework flags or total-ordering synthesis only
justify extension when a selected invocation or field query changes; no general ORM runtime
promise follows. Current `abstract_absence_known=false` and field-specifier structural identity
refusal are preservation constraints, not evidence to erase.

`Answers::try_get_getter_for_range` (1650) is a public actual getter trace. Scoped extraction
search found no call to it. **Proposed**: corroborate an attribute-read/property origin when
current Pysa implicit access is ambiguous, preserving getter identity separately from returned
value and descriptor. Existing Pysa artificial attribute origins/member evidence may already
satisfy that consumer; adoption requires a demonstrated output difference. Plain field,
overridden getter, dynamic descriptor and absent trace must remain distinct.

### 5.3 Ruff lexical detail, suppressions and configuration

Current fork `semantic_facts` emits populated active node, scope, definition, final binding,
reference and unresolved facts. `ruff_lexical.rs:398–465` retains load/typing/runtime/string/
TYPE_CHECKING flags, final binding location and unresolved annotation state. Normalization
requires native structural support and exact context before interpreting correspondence.
These facts provide a lower-coupling input for local reference/decorator/import evidence than
adding ty project traversal for every question. Static branch decisions currently come from
the explicit Pyrefly `SysInfo` adapter; runtime flow policy remains distinct.

Ruff diagnostics now select three rules and use the same captured parse/settings for
`Noqa::Enabled` and `Disabled`; multiset matching retains emitted versus suppressed rows
(`diagnostic_records.rs:117–243`). Exact source `ruff_linter/src/linter.rs:333–355` shows that
the Noqa flag governs removal; `Suppressions` contains range/line semantics, and range
suppression requires the full diagnostic range while `ruff: ignore` can match its start
(`suppression.rs:150ff,301ff`). The channel is the adapter's suppressed classification, not an
inventory of every directive or a separate unused-suppression analysis.

**Proposed better use:** original-evidence explanations may distinguish selected suppressed
diagnosis, rule not selected, or analyzer coverage unavailable. First interpretation belongs
to model diagnostic/evidence ownership; output must not equate “no selected diagnostic” with
valid example or absence of a defect. Existing channels/settings digest suffice for some of
this; add directive origin only when the output needs its specific source span. Controls:
matching/wrong-code noqa, range partial overlap, same diagnosis multiplicity, disabled rule,
parse failure, custom builtin, changed target Python and foreign primary/secondary span.

Pyrefly diagnostic channels already preserve suppressed/disabled/baseline states and the
unresolvable `NotComparedOrUnmatched` projection rather than guessing (`diagnostic_records.rs:
249–338`). Its constant validated ConfigFinder uses explicit captured roots/version/platform,
no fallback heuristics or interpreter query (`pyrefly_stage.rs:526–564`). Ruff settings use
`new_with_src`, avoiding ambient cwd. New rule facts, default severities or configuration schema
rows help only a named analyzer/explanation flow; they are not another feature catalog of the
analyzed library. ty `check_file` would supply a distinct provider result, not extend the
meaning of the existing Pyrefly/Ruff diagnostic relation.

## 6. Stale skill labels, independent controls and conditional libraries

The skill's project layer cites old baseline `761687bd`, old linked versions and the earlier
single-Pyrefly-parse/six-family narrative. These are stale for this tree. Likewise the old
unconsumed narrowing evaluator/closure-snapshot and deferred typed-predicate labels cannot
describe current C7/C8. The fork description as solely a visibility patch omits current
observational terminal/context-exit and Checker/precision seams. Conversely current manifests
confirm the ty semantic/IDE/project non-linkage; their unused labels remain applicable within
that inspected scope. Skill **supplier** pin versions agree; project wiring/status must be
read from repository owners. No skill was changed here.

Recommended review distinction: a missing first interpretation of already stored selected
evidence is different from additional provider subpayload, and both differ from a competing
inference authority. Each Proposed use above names independent positive/negative cases;
none is reported Tested. Historical skill CLI probes do not prove embedding the same API,
production environment equivalence or current compiled behavior. Configuration and source-family
policy checks alone do not prove semantic parity of a new consumer.

Only if a new intermediate analysis emerges:

- **Graph:** captured dependency/reference/class graphs can use `rust-graphs` task routes for
  SCC/reachability, DAG order or declared community/centrality projections. A reference graph's
  directed universe and edge meaning must be explicit before reusing an algorithm or ranking.
- **Reasoning:** use `rust-reasoning` for existing bounded BDD composition, theory relations or
  recursive fixpoints. Type-query disagreement is not itself a reason to add SMT; Datalog is
  relevant only to an actual recursive relation with a finite/charged termination contract.
- **DataFusion/Arrow:** relational provider/normalized joins, grouping, filtering and projected
  evidence reuse can use the `datafusion` task and representation routes. Inspect duplicate,
  null, ordering, schema and budget semantics before replacing a current keyed model operation.

These are conditional alternatives, not an exhaustive unrelated-library audit or predetermined
adoption. The source survey does not establish a complete CFG, heap aliasing, dynamic effect,
runtime purity or execution oracle; any newly requested such question needs its own model and
evidence rather than an analyzer display relabelled as proof.

## 7. Checks and unresolved evidence

- **passed:** read-only `git rev-parse HEAD` in both pinned fork roots; revisions match §1.
- **passed:** read-only Python enumeration of prior JSON confirms 26 areas / 137 concepts /
  340 primary provider records and lists all 41 old recommended payload IDs. This is finite
  accounting, not semantic conformance or fresh verification of all 488 old source links.
- **passed:** static `rg`/source reads at cited APIs and producer/consumer paths support the
  Interface-checked distinctions in §3–§5. Negative searches are limited to named manifests
  and current extraction/model/service paths; they do not establish upstream absence.
- **not_run:** new runtime/compile probes, CLI parity, performance, integrated gates, Q0,
  real-library compilation and operator activation. No new Tested/Measured claim is made.

Remaining consequential questions: whether selected existing Expected evidence already has a
sufficient useful first operation; which reference/descriptor/contextual query actually changes
a served answer; exact new ty database/opaque-type integration burden; actual mixed-provider
environment/source-role parity; and negative-control outcomes. The principal review should
judge these against the functional target and current model, independently of this opportunity
list. No production semantic defect is asserted solely from a missing indexed API call.
