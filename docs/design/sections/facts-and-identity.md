# Facts and identity

This owner defines what a fact is in the store: which typed table is the authority for it, how
its identity is derived, which closed vocabularies describe it, how coverage and boundaries keep
"not analyzed" distinct from "absent", and how the `nodes`/`edges` catalogs make the family tables
a graph without a second authority. Producers are the extractor and the Stage C/D derivations
([acquisition and extraction](acquisition-and-extraction.md#section-4)); consumers are validation
([§8](validation-and-evaluation.md#section-8)), publication and projections
([storage and publication](storage-and-publication.md#section-5)), analytics and synthesis. The
dependency direction is one way: every crate depends on `cpg-schema`, which depends only on
`arrow-*` and `blake3` (§B2). The executable declarations are authoritative for columns and codes:
`crates/cpg-schema/src/` (`tables.rs`, `derived.rs`, `codebook.rs`, `id.rs`, `graph.rs`,
`rules.rs`, `findings.rs`, `flows.rs`), with snapshot tests in `crates/cpg-schema/tests/` and
graph, syntax and corpus tests in `crates/cpg-core/tests/`. Flow semantics are in the
[behavior model](behavior-model.md#section-3-9); the map is in the [architecture README](../README.md).

## §3 Fact model

Evidence labels are per section. The families, identity recipes, codebooks and catalogs described
below are **Implemented** and **Tested** as each section states; the full ontology of §3.1 and the
tables marked **Proposed** are accepted targets, not implementation claims.


### §3.1 Layers and node kinds

> Decision: ADR-0047

The full ontology below is the **target model** (**Proposed**). Node kinds are introduced only
with the consumer that reads them (§3.2).

| Layer | Node kinds |
|---|---|
| Source | `Artifact`, `SourceFile`, `Module`, `SyntaxNode`, `Token`, `Trivia` |
| Lexical semantics | `Scope`, `Symbol`, `Binding`, `Reference`, `Import`, `Export` |
| API / object model | `Function`, `Class`, `TypeAlias`, `Member`, `Signature`, `Parameter`, `TypeParameter` |
| Types | `Type` |
| Resolution | `CallSite`, `Argument`, `ResolutionSet`, `DispatchGroup`, `SyntheticCallable`, `ExternalSymbol` |
| Documentation | `Document`, `Passage`, `Example`, `Docstring` |
| Findings and briefs | `Finding`, `Assertion`, `Brief`, `UsagePattern` |
| Execution overlay (deferred) | `ControlGraph`, `ControlPoint`, `MemoryLocation`, `Access` |

A variable name is not a binding event. A binding event is not a reference. A type is not a
declaration, and an AST node is not an execution point.

**The implemented node kinds** (the append-only `node_kind` codebook; §3.8; **Implemented** and
**Tested**):

| Family | Node kinds |
|---|---|
| API, calls and dependency context | `module`, `class`, `function` (every `def`, overload stubs included), `parameter`, `call_site`, `argument`, `export` (a public access path), `external_module`, `external_symbol`, `synthetic_callable` |
| `syntax` | `syntax_node` |
| `lexical` | `scope`, `binding`, `reference` |
| `types` | `type`, `field` |
| `docs` | `document`, `passage`, `code_block` |

- `ExternalSymbol` and `SyntheticCallable` above are these kinds.
- A `ResolutionSet` is keyed by its call site (§3.6), so it is not a separate node.
- One syntactic occurrence has one node id and one kind. An argument or a reference is a role,
  identified from its carrier (§3.4.1).


### §3.2 Fact families and authority

**Implemented** and **Tested** unless a line or table row says otherwise.

- **Family tables are authoritative.** Each fact family is a set of typed Arrow tables. The
  family is also the Delta table group, the coverage unit, and the unit an extractor declares it
  produces. "Fact family" and "relation family" are the same term. A family is added with the
  consumer, pass and columns that read it.
- **The family → node/edge mapping.** `cpg-schema` declares how each family maps to node and edge
  kinds: kinds, endpoint kinds, role and ordinal columns. Endpoint-kind validation (§8) consumes
  this mapping.
- **The node and edge catalogs** (§3.8). `nodes` and `edges` are Stage-D derived tables of
  family `graph`, generated from one registry. They carry identity, kind, endpoints and evidence,
  never a payload, so the family tables stay the only authority, as the typed extension tables of
  the catalogs. Projections (§5) select from the catalogs by kind, derivation class and evidence,
  and join the extension tables for payload.
- **One producer per table.** A fact table is written by one producer: one extractor surface, or
  one Stage-C/D derivation (§4.1). `runs`, `contexts`, `producers` and `facts` are registries each
  producer appends its own rows to.
  - `releases`, `distributions` and `source_files` are written **once per extractor run**, which
    carries Stage A's output; later producers reference `release_id` and never append. An
    attempt holds several extractor runs only over distinct releases (the library and its
    corpus), because the family keys carry no run.
  - The `lctx-compiler` run (analytics and synthesis) and the `manual-review` run are over the
    library release too, but they declare no families and write no `releases`, `distributions`
    or `source_files`; the extractor-only rules are scoped to runs that declare a code family.
  - The corpus run has its own context (its search path puts the fetched tree ahead of
    site-packages) over the same environment, whose `distributions` it lists too.
    `distributions` is keyed by `context_id`: the environment belongs to the context. The corpus
    context's identity does not yet cover the whole tree root it searches
    ([§4.0](acquisition-and-extraction.md#section-4-0); [plan W8](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
  - What both runs assert about one thing (a dependency module or definition, a type term and
    its structure) is one node and one edge, from its first fact; the other run's fact is a
    counted duplicate in the lineage.
- **Merged tables are derivations.** Where two providers contribute to one logical record, each
  writes its own raw table. The merged table is a DataFusion derivation
  (`relational_derivation`, `cpg_schema::derived`) that carries keys, both `fact_id`s and what the
  join decides: a mapping, a status, a reason, or an aggregate of raw values (`signatures.form`
  and `function_key`, `resolutions.unresolved_reason`). It never copies a raw payload column
  unchanged, so the raw table stays the authority (**Tested** by the derived-table snapshots).
- **Every target is a typed node, and a reason appears only where a provider says why.**
  - A target outside the release is an `external_symbol` or `external_module`.
    `exports.target_node_id` is the declaration, the dependency definition, or the module (of the
    release or a dependency) the path names.
  - Where a null needs explaining, a `reason` column holds the provider's reason: Stage C's own
    (`provider_disagreement`, `missing_evidence`, `outside_provider_model`,
    `no_source_declaration` for a function Pysa describes without a `def` of its own,
    `unreachable_in_context` for a `def` the context never binds), or, for an export, Pyrefly's
    symbol kind of the origin, or `missing_evidence` when Pyrefly traces no origin or records no
    kind. `boundaries` stays the extractor's.
  - A variable-like origin in a release module (a variable, attribute, constant, parameter, type
    parameter or type alias) targets its module-scope `binding`: the last binding event of the
    name by ordinal, unbinding events excluded. `variable_origin` remains only for a variable-like
    origin in a dependency module, which has no binding node.
  - Any other unmapped target is our own failure to find what the provider referenced. It keeps a
    null reason, and a `typed:*` rule rejects the snapshot. There is no catch-all reason.

| Family | Tables | Status |
|---|---|---|
| `provenance` | `releases` (library, requirement, lock digest, or a source tree's label), `distributions` (every installed distribution: version, artifact sha256s, `RECORD` digest, in the release or not), `source_files` (with its release `distribution`; its `text`, the bytes every span indexes, null only when not UTF-8; its `role`: `release`, `example`, `test` or `doc_block`; ADR-0015), `contexts`, `producers`, `runs`, `facts`; `context_modules` (each dependency module a fact references: name, site-relative path or Pyrefly's bundled typeshed, distribution and version) and `context_definitions` (the Pysa definitions of those modules: the existence source of `external_symbol` nodes; §3.8) | Implemented, Tested |
| `exports` | raw: `declarations` (Ruff: qualified name, kind, parent, span, docstring text and span, `is_overload`), `export_syntax` (Ruff: import aliases, `__all__` statement span; syntax evidence only), `public_names` (Pyrefly: access path → origin and its file, `via_dunder_all`; §4.2.3). Derived: `exports` (public access path → the seed declaration in the origin's file: an implementation before an `@overload` stub, then the one Pysa describes, then the last in source order; one row per `public_names` row, so a `.py`/`.pyi` pair gives an access path two rows, one seeding each file, told apart by `source_files.is_stub`; Pass A seeds from the source row) | Implemented, Tested |
| `signatures` | raw: `parameter_syntax` (Ruff: ordinal, name, default text and span, annotation text); `parameter_docs` (Pyrefly's `parse_parameter_documentation` over each `def`'s docstring, Sphinx and Google styles; one row per documented parameter, its text as Pyrefly normalizes it, its span the description's verbatim bytes. The entry is anchored on its own header line (Sphinx `:param [type] name:`, Google `name:` or `name (type):`), never on a substring of the name, and runs over the lines indented deeper than the header; it is accepted only when its trimmed lines equal Pyrefly's text or begin with it, since Pyrefly's Google parser ends an entry at a continuation line holding a colon, and such a description is extended to its entry's end. A description that no header, or more than one, locates is a `signatures` boundary (`provider_disagreement`), never guessed; `docstring_tests` in `walk.rs`); `pysa_functions` (Pyrefly: function key → name span, Pysa's flags, signature count); `parameter_semantics` (Pyrefly Pysa undecorated signatures: kind, required, annotation); `class_ancestry` (Pyrefly: bases and reported MRO); `pysa_classes` (one row per class, so a class without bases is keyed). Derived: `provider_node_map` (Stage C, name-span join), `signatures` (per `def`: its callable, stubs rolled up to the implementation, and its Pysa signature index), `parameters` (Ruff ⋈ Pysa on the ordinal), `provider_class_map` (Stage C for classes), `synthetic_callables`, `ancestry_targets` and `override_targets` (bases, MRO entries and overridden methods resolved to nodes, a reason where an end does not resolve). **Known gap:** Pyrefly's resolved function flags (abstract method, body kind) are not persisted, so downstream abstract status is read from decorator text; persisting them is a schema migration ([plan W14](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition), RF/F10) | Implemented, Tested |
| `calls` | raw: `call_syntax` and `arguments` (Ruff: span, owner, ordinal, keyword, starred, expression span; `call_syntax` rows are the call sites; `arguments.node_id`), `pysa_calls` (Pyrefly Pysa call graphs: targets, receiver, phase, unresolved reasons; `payload_id`). Derived: `resolutions` (§3.6, one per call site), `call_targets` (joined on the full call-expression range, §4.2.3; typed: a declaration, a synthetic callable or a dependency definition, with the higher-order argument), `argument_resolutions` (higher-order arguments: status and unresolved remainder). Pysa rows at non-call sites (property accesses, identifiers, artificial and format-string sites) land on `syntax` and `lexical` nodes through `site_targets` and `identifier_targets` (§3.8's partition) | Implemented, Tested |
| `embedding_cache` | `embedding_cache` (spec_hash, input_hash, vector as `List<Float32>`, model identity). Global and append-only, not snapshot-qualified: its read mode is `global` (§6.2); written by an insert-only MERGE; the key is unique (§6.1) | Implemented, Tested |
| `coverage` | `coverage`, `boundaries` (§3.7) | Implemented, Tested |
| `graph` | derived: `nodes`, `edges` (§3.8). Not a coverage unit | Implemented, Tested |
| `syntax` | Raw `syntax_nodes` (Ruff): every statement, the clause nodes (`elif`/`else`, `except`, `case`, `with` items) and **every expression outside annotations**: placement depends on the source alone, never on a provider. Each row has its parent (the nearest placed ancestor), owner, field (`syntax_field`: body, test, orelse, handler, exc, cause, default, argument, …), ordinal in that field (a statement's block index), span, `kind` (Ruff's `NodeKind`, the `syntax_kind` codebook, an exhaustive match) and detail (a name, attribute, operator, literal as written, or a handler's name). A `def`, a `class` and a call are placed under their declaration and call-site ids; nothing inside an annotation is placed. Derived: `site_targets` (each Pysa attribute, artificial and format-string record → the deepest syntax node at its span; a chained comparison's pairwise site → its comparison → its typed target; a span with no node is our own failure, never a reason). Consumers: Pass B guards, raises, handlers and defaults; Pass C straight-line regions; FCA raised types | Implemented, Tested |
| `lexical` | Raw, from our recognizer (surface `lctx-lexical`, `recognizer`, inside the Ruff walk): `scopes` (module, class, function, lambda, comprehension; owner, and parent = the scope the scope's position evaluates in, so a lambda in a default or decorator belongs to the enclosing scope); `bindings` (every binding event per scope, ordinals in source order: kind, site, span, the assigned value's span, and the innermost branch Pyrefly decides statically that it sits in: the deciding test's kind (`type_checking`, `version_info`, `platform`, `constant`, `combined`) and whether Pyrefly analyzes or prunes that branch, clause by clause exactly as `SysInfo::pruned_if_branches` decides, recursively, so an `if` inside a pruned clause is never walked and its bindings take the pruning clause's mark; `static_marks_agree_with_pyrefly_pruning` and `static_polarity_is_pyrefly_recursive_pruning` check every fixture `if` and assignment binding against Pyrefly's own pruning; every event under `global`/`nonlocal` binds in the declared scope, a `nonlocal` target decided once every binding is known; a repeated name in one declaration is one event; the module's implicit globals and a method's `__class__` cell are `implicit` events); `references` (every name load outside annotations, and an augmented assignment's target, which reads before it binds; a role of its placed name, with its parent and field; nothing inside an annotation opens a scope or binds); `reference_resolutions` (Python's scoping rules as modelled: the scope's own bindings, else the nearest enclosing function scope with class scopes skipped, else the module, else the star imports whose wildcard set holds the name, else a builtin; comprehension first iterables and function defaults in the enclosing scope; walrus in the nearest non-comprehension scope; flow-insensitive candidates, except that a module or class body reading a name it binds only later also reads it from outside, as `LOAD_NAME` does; a builtin names itself, a builtin variable reads `variable_origin`, anything else `unresolved_target`). Every name set from outside the module's text is Pyrefly's: its `ImplicitGlobal` set, its `builtins` definitions that are real public names, each star import's `Transaction::get_wildcard` set (a star module Pyrefly cannot find stays a candidate for any otherwise unbound name). **Not modelled** (deferred until a consumer needs them): PEP 695 annotation scopes (class and alias type parameters) and the implicit unbinding at the end of an `except … as` handler. `export_syntax.resolved_module` is each import's absolute module by Pyrefly's own `ModuleName::new_maybe_relative`. Derived: `identifier_targets` (Pysa's identifier sites → the reference at their span → typed target), `import_targets` (each import → the release or dependency module it names; `unresolved_target` only where Pyrefly's finder says not found, a `context_modules` row of origin `not_found`, or where the import climbs past the top package). Consumers: Pass B binding order (§4.2.4), Pass C bindings and values, the import graph, `if_called` targets, variable exports | Implemented, Tested |
| `types` | Raw, from Pyrefly's native types (surface `pyrefly-types`, `native_structural`). `type_terms`: one row per distinct term, its id a Merkle hash over Pyrefly's own structure and identities (kind, detail, class pair, children with their roles; §3.4.1); the display is a label, and only a display-only kind (`other`, `truncated`) hashes it. Two structures that share an id but differ in kind, detail or display fail `unique:type_terms` (several runs may observe one term; `nodes` keeps it once). A class is a (module ref, class key) pair, an enum member keeps its class, and a recursive alias is a reference to its name, so a term is finite; a depth cap (32) makes that a guarantee. A type variable's id is Pyrefly's own identity (`QuantifiedIdentity`), so one variable is one term wherever it is observed and two unrelated `T`s are two; its bound, constraints and default are its children (`type_arg_role` `bound`, `constraint`, `default`). `type_term_kind` maps every `Type` variant by an exhaustive match; solver-internal and experimental variants are `other` (`display_only`). `type_term_args`: each child at its role and ordinal; a callable parameter carries its name, kind and requiredness. `type_observations` (§3.5.1): each parameter's type, each `def`'s return (an annotated one is the annotation), each call's result, each argument's value and each `raise`'s exception (Pyrefly's expression trace at the exact span; calls in annotations excluded). A subject Pyrefly records no type for (a `TypeVar(...)` declaration, a call in a lambda body, a branch Pyrefly skips for the platform) is a `types` boundary (`missing_evidence`) and the module's coverage is `partial`; a bare `raise` has no exception to type. `record_fields`: the fields a dataclass, attrs or pydantic class, `TypedDict` or `NamedTuple` declares itself (an inherited field a subclass assigns in a method stays its base's), with the flags as the field states them (default, `init`, alias and `kw_only` through `ClassField::dataclass_flags_of`; `TypedDict` required and read-only); the constructor they imply is Pyrefly's synthesized `__init__`, a `synthetic_callable` with its `parameter_semantics`. Derived: `type_class_targets` (each term's class → a release class or dependency definition; a miss is our failure, never a reason) and `type_binders` (each source-anchored variable → the innermost release declaration, type-alias or assignment statement holding Pyrefly's scope anchor; `scope_boundary` for an anchor outside the release). Consumers: Pass C type compatibility, FCA parameter, return and raised types. Controls from record fields are **not yet read**: Pass B follows parameters only, until a seed's controls are a record's fields | Implemented, Tested |
| `docs` | A corpus run over the library's upstream tree at its pinned commit (§4.0), in the library's environment; its search path is the tree, then site-packages, so a module both runs import is one file. Raw, parsed by markdown-rs (MDX constructs and frontmatter; byte offsets): `documents` (each selected file: path, digest, frontmatter title, whether it parsed; one that does not parse is `unavailable` with markdown-rs's message), `passages` (each root-level heading's section, whatever its depth, to the next, so a document's passages partition it, with level, heading and heading path; the text before the first heading is passage 0), `code_blocks` (fenced blocks at any depth, MDX components included: language, meta, code, digest; each in the passage its start falls in), `doc_links` (URL, title, text). `doc_components` holds each MDX JSX element, flow or text form (`component_form`), in pre-order with its parent ordinal and depth, its name (none for a fragment), its span, its inner span (first child to last; none when self-closing) and its lead (the first direct paragraph), in the passage its start falls in; `doc_component_attributes` holds its attributes in order, each by kind (`attribute_value_kind`: a literal's value as written; an expression's as source text, never evaluated; a bare name without a value; a spread without a name). Fenced and inline code never yield a component, and a heading inside one opens no passage. Components are span facts, not graph nodes; consumers are §10.3's documented warnings and `<ParamField>` parameter descriptions. `mentions` (our recognizer, `lctx-docs`) against the library run's public names and declarations, two classes never merged: `exact` for inline code (or a dotted prose token) that is a public access path, an origin path or a public class's member (`FastMCP.tool`); `lexical` for inline code that is a bare public name of a class, function, method or module, or such a name in prose when it is distinctive (an underscore, or two capitals and a lower-case letter), one `candidate` per origin (re-exports collapse to the shortest access path). Embedding-based linking is §9.7's. Derived: `mention_targets` (→ the `export` node, or the member's release declaration by the seed rank). Consumers: exact doc links to APIs and extractive brief text, §9.4 co-mention. **The usage code** is the same corpus run's code: the selected examples and tests, and every Python code block materialized as a module of its own (`_lctx_blocks/d_<document>/block_<n>.py`, named in `code_blocks.module_path`), with every code family but `exports`. The corpus names each installed file the release's distributions own by the library run's own site-relative `@path`, so a usage call's target is the release's own declaration or synthetic callable (the same Pysa key), a release class is one type term whichever run observes it, and an import of a library module targets the library's module node. A tree that holds its own copy of the package ahead of the installed one (a flat layout) would cut the usage code off the release, so it fails the compile, naming the module. Each usage module's text and role are in `source_files` (ADR-0015), so a snippet and whether it is an example, a test or a doc block are read from Delta alone. Consumers: Pass C examples and tests, §10.3–§10.5 usage patterns, §9.4 co-use | Implemented, Tested |
| `findings` | Analysis tables (contracts in `cpg_schema::findings`; provenance in-row, no `fact_id`; not a coverage unit; outside the `nodes`/`edges` catalogs): `analysis_invocations` (method, parameters as canonical JSON, projection digest, seed, diagnostics), `findings`, `finding_members`, `witnesses` (path steps keyed by node ids, `edge_id` as lineage), `evidence` (evidence_id → one of: fact, span, passage, example, fixture run, with resolved text), `assertions`, `assertion_support` (assertion → finding / evidence, role `support` or `scope`), `briefs` (with `review_state`, outside `brief_id`), `brief_assertions`, `brief_members`, `brief_documents`, `embedding_specs`, `assertion_policy`; and `public_paths` (every public path under the config's roots, own and inherited, with its export, kind, `own` and one `preferred` per node; §9's opening). A usage pattern is a `usage_pattern` assertion, not a table | Implemented, Tested |

**The behavior model's families.** The semantics are in [§3.9](behavior-model.md#section-3-9) and
[§9.9](behavioral-analysis.md#section-9-9); this table says what is stored.

| Family | Tables | Status |
|---|---|---|
| `behavior` | Analysis tables, written after `public_paths` in the analysis group; no coverage rows: `operations.behavior_status` is every public callable's verdict, and `operation_facet_status` says per operation and facet whether its rows are complete. The in-session relations `argument_flows`, `guards`, `parameter_reads` and `handoffs` (`cpg_schema::flows`); `delegations` (depth-1 call arcs with modality) and `behaviors` with the control fates Pass B finds (per public operation parameter: forwarded to which formal, a literal supplied to a callee's formal, raises when, or unfollowed with a reason). **No negative fate from absence:** a parameter with no fate on a channel the scan does not see is `not_analyzed` for that channel, never "unused". **One verdict per arc:** a behavior whose path crosses a candidate or potential arc is `unknown` (`override_dispatch`, `ambiguous_binding`), as the delegation over it is; `behavior_steps` keeps every behavior's path hop by hop, with per-hop conditions. Over the flow IR (`cpg_core::flow_model`): `value_flows` (per sink, the parameters or receiver fields whose value reaches it, identity, derived or only through a call, under a condition in the function's places; a rebound fallback is followed), `field_accesses` (every `x.f` read or written, by name), `ambient_reads` (reads of a module-global singleton's fields at the resolved key, with the read phase: `import`, `construction`, `snapshot`, `per_call`), `dynamic_accesses` (the getattr family and kin, and the class or modules they reach under the model), `raise_sites` (every `raise` with its region's condition, the parameters its tests read, and whether it `escapes` its function; only an escaping raise is a guard or a fate), `singletons`, and `negative_premises` (per parameter, field and setting). Behaviors include `derives`, `stores` (to a field or dict entry; a field stored unchanged composes with its reads in any relative's method at depth 2), `returns`, `reads_setting`, `is_read` (refuted under the model only where its premise holds: for a parameter, a concrete, runtime-reachable body no release subclass defines again; else `unknown` with `abstract_body`, `runtime_unreachable` or `override_dispatch`) and `tests` (the literals of the innermost test that reads a parameter), each with its `condition`. An operation the runtime view cannot reach is `unknown` (`runtime_unreachable`), with every claim about it; a claim reached only inside a call is `unknown` with `call_transfer` unless a transfer summary (§9.9) admits it. **Known gap:** qualified and aliased builtin `getattr` produce no read or dynamic boundary, so a field no-read premise can be wrong ([plan W4](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)). Handlers, callbacks and resources beyond the implemented summaries are **Proposed** (the forward plan's Stage 3) | Implemented and Tested for the relations named (2026-09-24); the rest Proposed |
| `flow` | Raw facts of the extractor run (surface `ty-flow`, `native_traversal`, `analyzer_assertion`, `normalized_structural`), from `cpg-flow` over every UTF-8 release module ([§4.2](acquisition-and-extraction.md#section-4-2)): `flow_uses` (every place load ty records, with its scope and whether it is inside an annotation), `flow_definitions` (every binding, normalized, with its value's span), `flow_reaching` (use → reaching definition or none, with its condition and `loop_carried`), `flow_values` (per sink: a definition's value, a call argument, a `return`, a `yield`, a `raise`; each use inside it, identity or derived, `through_call` when it sits inside a call, with the condition inside the expression, nested conditional expressions included), `flow_regions` (every statement's reachability, relative to its scope's entry), `flow_tests` (every test ty records as a predicate, with its span and condition: what a test reads comes from the uses inside it), `flow_attribute_loads` (every attribute load by name on any receiver, and `getattr`/`hasattr` with a literal name: what the field premise counts), `conditions` (the canonical encoding; `stated` false past the budget) and `condition_literals`. One coverage row per module: complete, `partial` (`syntax_error`) from a recovered tree, `failed` when the module already names the rename's sentinel. **Parity rules**, each with an injected case: `flow-use-is-a-reference` and `reference-is-a-flow-use`, `flow-definition-is-a-binding` and `binding-is-a-flow-definition` (declared residue: annotation and type-alias uses; `global`, `nonlocal`, `del`, implicit and star-import bindings; a class's or an alias's type parameters), `flow-reaching-within-candidates`. Exits and handlers are derived from regions and syntax, not a table of their own. **Known gap:** ty runs with empty program settings, so its Python version and platform are not the run context's ([plan W14](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition), RF/F11) | Implemented, Tested (2026-09-24) |

General alias analysis (points-to) stays out of scope ([§1.3](../DESIGN.md#section-1-3)); flow
over bounded places is the `flow` family, and type structure and `record_fields` are the `types`
family.

**Dependency context for model-named classes** (**Implemented and Tested in focused cases**,
2026-09-25; ADR-0045). The dependency context also retains a class definition named by a
committed exception model when its module is already described, so the model compiler can cite a
pinned class identity even if the analyzed source never mentions that class. Context capture does
not assert an exception occurrence. Each retained context class also has an attributed, ordered
Pyrefly MRO relation; resolved-empty and cyclic MROs have explicit marker rows, and publication
checks coverage, order and child identity. An ancestor is identified by Pyrefly's module and class
key and only becomes a handler class identity through its pinned `context_definitions` row.

> Decision: ADR-0047, ADR-0046, ADR-0015, ADR-0045


### §3.3 Physical profiles

**Tested** (`every_table_round_trips_through_delta_exactly`, 2026-09-23). Every data file is
written with **zstd level 3** (`delta::writer_properties`; `data_files_are_zstd`), with Parquet's
default dictionary encoding and page statistics.

| Logical value | Computation (Arrow) | Delta | Invariant |
|---|---|---|---|
| Node, fact, run, snapshot ID | `FixedSizeBinary(16)` | `Binary` | exactly 16 bytes |
| Content, schema or spec digest | `FixedSizeBinary(32)` | `Binary` | exactly 32 bytes |
| Closed category | `Int16` | `Int16` | code present in the versioned codebook |
| Offset, ordinal, count | `Int64` | `Int64` | range checks; no unsigned types anywhere |
| Projection-local dense index | `UInt32` | never persisted | temporary coordinate |
| Ordinary text | `Utf8` | `Utf8` (read back as `Utf8View`) | field-specific validation |
| Optional factual flag | nullable `Boolean` | nullable `Boolean` | null (unknown) is distinct from `false` |
| Score or weight | `Float64` | `Float64` | finite |
| Timestamp | `Timestamp(µs, "UTC")` | same | timezone string exactly `"UTC"` |
| Embedding vector | `FixedSizeList<Float32, 4096>` | `List<Float32>` in `embedding_cache` (child renamed `element`) | length 4096, finite, unit norm, checked on read |

**Conversion happens only at the Delta boundary**, checked in both directions.
- FixedSizeBinary is written as generic BINARY and read back through the DataFusion provider as
  `BinaryView`; a direct `BinaryView → FixedSizeBinary` cast is unsupported (**Tested**,
  2026-09-22).
- **Read path:** BinaryView → Binary → FixedSizeBinary(16), two `cast_with_options` steps with
  `CastOptions { safe: false }`. A wrong length is an error.
- **Timestamps:** µs normalization is lossy for ns, so only µs is ever written.

> Decision: ADR-0047


### §3.4 Identity rules

**Implemented** and **Tested** (the id-recipe snapshots and identity tests named in §3.4.1).

- **A qualified name is a label, not an identity.** Package versions, roots, stubs and
  redeclarations stay distinct; source and stub are linked by `STUB_FOR`.
- **A span alone is not a node ID.** Identity includes the syntax kind and the structural
  occurrence path.
- **Provider-local IDs** are mapped through `(run, module, provider kind, local key)`.
- **Codebooks are append-only.** Codes are never regenerated or reordered.
- **Source coordinates** are byte offsets into the exact UTF-8 parser input: the text Pyrefly
  loaded and parsed (BOM included), which is also the text the Ruff walk sees (§4.2.2). Other
  coordinates are converted, never assumed to match. Pysa locations (1-based line, 1-based UTF-8
  byte column) are converted with that module's own `LineIndex::offset(.., Utf8)`, the exact
  inverse of how Pyrefly produced them (**Tested**, 2026-09-22: all 29,279 locations of a
  FastMCP corpus round-trip exactly).
- **Type variables keep binder identity:** two unrelated parameters named `T` are distinct.
- **Deduplicate repeated ingestion of the *same* assertion only.** Assertions from independent
  providers are separate facts, even when they agree.

> Decision: ADR-0046, ADR-0047


### §3.4.1 ID derivation

> Decision: ADR-0047, ADR-0046, ADR-0045

**Implemented** and **Tested** unless a row says otherwise: the extractor's ids are pinned by an
id-recipe snapshot (`extractor_id_recipes_snapshot`, 2026-09-22), the Rust/SQL recipes by shared
known answers, and the analysis-result recipes by per-contract property tests.

**Encoding.** `BLAKE3("lctx-id/v1" ‖ len‖kind_tag ‖ len‖field …)`. Lengths are u64
little-endian. IDs are the first 16 bytes; digests are all 32. A version bump in the tag is a
migration (DP-24).

| ID | Derived from | Scope |
|---|---|---|
| `release_id` | the release distributions' names and versions and the sorted (path, sha256) of their `RECORD`-verified analyzer-readable files: what is analyzed, never the lock entry (a source tree: its label; a corpus: §4.0) | global |
| `node_id` | `release_id`, path within the release, then the structural occurrence path (syntax nodes: ruff `NodeKind` names and child ordinals) or the qualified name and occurrence (declarations) | stable across snapshots and runs. Syntax ids are producer-scoped: a ruff bump may rename a node kind |
| `context_id` | Python version, platform, ordered search and site-package paths (root-relative), config digest, environment digest (the installed distributions' dist-info names and every analyzer-readable site-packages file's site-relative path and content), lock digest. It does not yet cover the corpus tree root (§4.0, [plan W8](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)) | global |
| `producer_id` | tool, tool revision, adapter build digest (§4.0) | global |
| `run_id` | `release_id`, `context_id`, `producer_id`, sorted enabled families, the producer's own config digest | global |
| `fact_id` | `run_id`, record kind, subject id(s), canonical payload bytes. Provenance is outside the id: the same payload with different provenance fails the run (Tested) | per run |
| `finding_id`, `assertion_id`, `brief_id`, `evidence_id`, `invocation_id` | kind, subject `node_id`(s), canonical payload, per the analysis contracts' identity columns. **No config digest**, so an unchanged finding keeps its ID when parameters change; ablation diffs are joins. A finding's payload names its witness steps by call-site and callee node ids, never by `edge_id` (producer-scoped). An assertion's includes its sorted supports; a brief's, its seed, applicable case and sorted (section, ordinal, assertion); `review_state` is outside it. An invocation's is its method, parameters digest, projection digest, subject and seed. `capability_id` = `brief_id` | content |
| `behavior_id` | `H("behavior", operation, kind, parameter, callee, target, value, site)`: the claim. The **verdict, boundary reason, depth and `conditional` are outside it**: they grade the claim. A re-grade keeps the id, and `lctx diff` keys behaviors by id **and** verdict so it shows. Two rows under one id are an error of the scan, never merged | content |
| `summary_id` (**Implemented and Tested in focused cases**, 2026-09-25) | `H("summary-flow", callable, formal, input path, output path, transfer kind, condition id, return site/region facts, ordered (step kind, evidence id, step condition id))`. The verdict, boundary and approximation are outside the identity. Raw fact ids in its steps make the path producer-scoped; two parallel proofs with equal endpoints remain distinct | derived, producer-scoped |
| `condition_id` | `H("condition", encoding)`, the canonical DNF encoding ([§3.9](behavior-model.md#section-3-9)) | content |
| `edge_id` | `edge`, edge kind, source and target node ids, then the kind's discriminator: an ordinal, or for a provider row joined at one site its run-independent payload digest (`pysa_calls.payload_id` = `pysa-call` over the row's payload). Never a `fact_id`. **Tested** (`the_catalogs_hold_every_graph_shape`, `the_catalogs_are_the_same_across_runs_order_and_location`) and byte-identical on a pilot rerun and relocation (2026-09-23) | stable across snapshots and runs |
| Role and derived node ids | Argument: `argument`, call node, ordinal (Rust). Export: `export`, `release_id`, access path (SQL). Synthetic callable: `synthetic_callable`, module node, Pysa function key (SQL). External module: `external_module`, owner, owner version, module name (Rust), where the owner is the distribution whose `RECORD` lists the file and its version, else `pyrefly-bundled` and the fork revision, else `unowned` and the file's content digest. External symbol: `external_symbol`, the external module id, definition kind, Pysa key (Rust); its qualified name is a label, because conditional definitions can share one. Scope `H(scope, owner)`, binding `H(binding, site, name)`, reference `H(reference, name node)`. Type term: `type`, kind, detail, class pair and type-variable identity, then each child's role, ordinal, id, parameter name, kind and requiredness; a variable's is its identity alone, a display-only kind's includes its display (Rust; a Merkle id with no SQL form, so no `id:` rule). Field: `field`, class node, name (Rust; `id:record_fields`). Document: `document`, release, path. Passage and code block: `passage` or `code_block`, document node, ordinal (Rust; `id:documents`, `id:passages`, `id:code_blocks`) | stable across snapshots and runs for the same inputs. Type terms and external symbols are producer-scoped like syntax ids: they hash Pyrefly's detail text, Pysa keys and anchor byte offsets, so a Pyrefly bump or an edit earlier in a module renames them |
| `snapshot_id` | a fresh random 128-bit value per compile attempt | execution identity (DP-04) |
| `content_digest` | sorted `run_id`s (each carrying its `release_id`, and the lock and environment through its context; the `lctx-compiler` run carries the analytics-config digest), compiler digest, embedding spec hash, and a digest of the sorted `(spec_hash, input_hash)` keys the snapshot used. Never the shared `embedding_cache` version, which another library's compile can move. Equal across a pilot rerun and relocation (**Measured**, 2026-09-23) | compares reruns |
| `compiler_digest` | the locked engines (DataFusion, Arrow, Parquet, object_store, delta-rs and its kernel, read from `Cargo.lock` by `cpg-core`'s build script) and analysis libraries (`lctx_analytics::LIBRARIES`), a hand-bumped compiler output version, the UDF version, the synthesis template version (`synth::TEMPLATE_VERSION`, which stands for Stage F's queries and templates; the analysis ledger test fails any output change made without bumping it), every derivation query, declared projection digest and Pass B relation digest, the public-path relation, every table contract and every validation rule; and a digest of every `.rs` file of `cpg-core`, `lctx-analytics` and `cpg-schema`, computed by the same build script, so a code change no version names still moves run and producer ids (`every_compiler_source_is_hashed`). `TEMPLATE_VERSION` stays the published lineage and the ledger the alarm. Stored on every `snapshots` row (a unit test on each input) | per build |

- **Ids in SQL** (`cpg_core::udf`; **Tested** by its known-answer and plan-time refusal tests).
  Stage D computes its ids with one scalar UDF, `lctx_id(kind, …)`, registered in every session.
  - It implements `IdHasher` exactly: the kind through `IdHasher::new`, and every other argument
    in the `opt_*` encoding (a presence byte, then the length-prefixed value).
  - It accepts Utf8/Utf8View/LargeUtf8, Int16 and Int64 (hashed as i64), Binary, BinaryView and
    FixedSizeBinary, and Boolean. UInt64 (`row_number()`), Int32 and floats are rejected at plan
    time, never cast. The kind must be a non-null text literal, and the result field is
    non-nullable; both are checked when the query is planned (`return_field_from_args`). Each
    argument's type is resolved once per batch, and each row continues a hasher already seeded
    with the tag and kind.
  - Known-answer vectors are shared with the Rust tests, so an id computed in Rust (the
    extractor, a later pass) equals the one computed in SQL. The UDF's recipe is part of
    `compiler_digest`.
- **Keys are snapshot-qualified.** Uniqueness is checked on `(snapshot_id, key)`, so an identical
  rerun re-emits the same `node_id` and `fact_id` in a new snapshot without conflict (**Tested**
  at pilot scale, 2026-09-23: a rerun's `nodes`, with `existence_fact_id`, are byte-identical).
- **Collisions are validator failures** (§8), never silently merged. `nodes` keeps one row per id
  only for the kinds several existence rows legitimately assert (an export read from a `.py` and
  its `.pyi`, a dependency module or symbol, or a type term two runs reference); any other
  repeated id fails `key:nodes`, and a type term's id with two kinds, details or displays fails
  `unique:type_terms`.
- **One recipe, two places.** The ids the extractor computes in Rust whose inputs are also
  columns (argument, external module, external symbol, scope, binding, reference) are
  `cpg_schema::id::recipe` functions in the UDF's `opt_*` encoding. A generated `id:*` rule
  recomputes each in SQL on every compile, and known-answer values are pinned.
- **Analysis-result recipes are Rust-only** (`cpg_schema::findings::recipe`; `lctx_id` takes
  scalars, not lists). Each contract declares its identity and lineage columns; a property test
  per contract checks that perturbing an identity column changes the id and a lineage column does
  not.
- **Producer scope.** `call_target` and `higher_order_target` edge ids take Pysa's payload,
  function keys included, so a Pyrefly bump may rename them, like syntax and external-symbol ids.
- **The compiler has its own run.** Analytics and synthesis are a run of producer
  `lctx-compiler` over the library release and its context, whose config digest is the analytics
  config's and whose tool revision is the `compiler_digest`. Its `runs` and `producers` rows are
  written with the raw tables (every input is known before Stage B), so `content_digest`
  includes it. It declares no families.
- **Overloads.** A public callable with `@overload` stubs is **one** declaration node (the
  implementation), with one `signatures` row per overload (`is_overload`) plus the implementation
  signature. Seeds and briefs attach to the declaration. A stub rolls up to the first later `def`
  of its name that is an implementation or that Pysa describes, else the group's last stub; Pysa
  gives one undecorated signature per stub, in source order, and none for an implementation.
- **Binding choice follows the analyzer.** Where one file binds a name more than once (a
  `sys.version_info` or `TYPE_CHECKING` branch, a redefinition), the `exports` seed and the
  overload roll-up prefer the `def` Pysa describes (a Stage-C key), because Pyrefly's binding pass
  drops the branches the context decides statically. The other reads `unreachable_in_context`.
  Under `TYPE_CHECKING` that means the typed facade wins over the runtime body, a real choice
  Pass A inherits. Calls inside an unbound `def` still read `missing_evidence` (Pysa has no record
  of them). **Tested** on `derive_cases` (2026-09-22).


### §3.5 Vocabularies and codebooks

**Implemented** and **Tested** (`registry_snapshot`, `codes_are_dense_from_zero_and_names_unique`,
the `codebook:boundaries.reason` case).

**Codebooks** are append-only `Int16`, versioned in `cpg-schema` (`codebook.rs`), whose
snapshot-tested registry is authoritative for every value. The table below explains the
codebooks whose meaning the code does not state.

| Codebook | Values |
|---|---|
| `origin` | input_context, source_observation, analyzer_assertion, derived_analysis, synthetic_model |
| `fidelity` | raw, native_structural, normalized_structural, report_projection, display_only |
| `extraction_mode` | native_traversal, report_decode, relational_derivation, graph_analysis, recognizer, statistical_analysis, template_synthesis, fixture_execution, manual_review |
| `modality` | definite (holds whenever the subject exists, under the model), candidate (one of a set), potential (holds if some condition occurs, e.g. "if called") |
| `coverage_status` | complete_under_stated_model, partial, not_requested, unavailable, failed |
| `resolution_status` | resolved, partial, unresolved, not_attempted |
| `resolution_domain` | call, attribute, import, name, type |
| `invocation_phase` | call, new, init, decorator, property_get, property_set |
| `boundary_reason` | native_unavailable, unresolved_target, unsupported_unpacking, ambiguous_binding, unsupported_control_flow, scope_boundary, budget_reached, missing_evidence, not_requested, provider_disagreement, outside_provider_model (a construct the provider's model does not cover, e.g. a call inside an annotation), syntax_error (facts from a recovered tree), undecodable_source (bytes are not UTF-8), no_source_declaration (a function the provider describes that has no `def` of its own: a synthesized member such as a dataclass `__init__`, or a callable class field), unreachable_in_context (a `def` the analyzer's context never binds), variable_origin (an export whose origin Pyrefly calls a variable-like symbol in a dependency module; a release-module origin targets its `binding`), and the values the behavior model appends |
| `pysa_unresolved_reason` | the variants of Pysa's unresolved-call reason at the pinned Pyrefly, spelled as Pysa spells them; appended when the pin moves |
| `evidence_status` | structurally_observed, documented, statistically_derived, fixture_checked, unresolved |
| `source_role` | release, example, test, doc_block (ADR-0015) |
| `mention_class`, `mention_source` | exact, lexical; inline code, prose |
| `unfollowed_reason` | rebound, computed, unmapped: Pass B's reasons, stored as text in `finding_members.label` and read back by name, never by a default |

- **Other codebooks** (`node_kind`, `edge_kind`, `derivation_class`, `module_origin`,
  `definition_kind`, `symbol_kind`, the extraction codebooks, `syntax_kind`, `syntax_field`,
  `lexical_scope_kind`, `binding_kind`, `static_branch`, `type_term_kind`, `type_arg_role`,
  `type_role`, `record_kind`, `component_form`, `attribute_value_kind`, `finding_kind`,
  `assertion_kind`, `analytic_method` and the behavior model's) are declared in `cpg-schema` with
  their family. Points of meaning: `lexical_scope_kind` is separate from the coverage
  `scope_kind` (whose `document` value is the `docs` family's coverage unit); `binding_kind` is
  the recognizer's binding events: Ruff's kinds it emits plus `del`, the `global`/`nonlocal`
  declarations and `implicit` (a module's implicit globals, a method's `__class__` cell);
  `module_origin` `not_found` is an import Pyrefly's finder cannot find, a row with no node;
  `edge_kind` code 35 (`usage_link`) is retired and never reused.
- **Missing output is not negative evidence.** "Unavailable", "not requested", "failed" and
  "unresolved" are recorded separately.
- **`fidelity` values.** Each value says what structure a fact guarantees:
  - `raw`: text or bytes exactly as found in the source (spans, slices);
  - `native_structural`: the provider's own structure, field for field (a Ruff AST node, a native
    `pyrefly_types::Type`);
  - `normalized_structural`: our structure-preserving derivation;
  - `report_projection`: a provider's documented projection of richer internal state (Pysa's
    structs). What the projection carries is all there is;
  - `display_only`: a display string with no structure.

  A row's fidelity is that of its weakest semantic field.
- **`model_id` is not a codebook.** It is `<producer_id>/<surface>` (e.g. `ruff-ast`,
  `pyrefly-pysa`, `pyrefly-public`, `lctx-compiler/pass-a`), validated against `producers`.
- **Extractor tables use `extraction_mode = native_traversal`.** `report_decode` stays in the
  append-only codebook, unused. The exceptions: boundary facts compare two surfaces, so they are
  surface `compare` with `relational_derivation`; the lexical recognizer's rows and docs mentions
  are `recognizer`. The mode is a parameter of the fact sink, never hard-coded.
- **Pyrefly-sourced values map through exhaustive matches.** Every Pyrefly enum → codebook mapping
  is a `match` with no wildcard arm, so a new upstream variant (a new unresolved reason, a new
  `OriginKind`) fails the build instead of degrading silently. `OriginKind` becomes
  `site_detail` text through our own exhaustive match, never upstream's `Display`.
- **`type_role`** says what an observation types, and follows from the subject: `parameter`,
  `return`, `call_result`, `argument`, `raised`. Whether the type is an annotation's or computed
  is the observation's `declared` flag, not a role. Contextual roles (`expected`, `narrowed`,
  `unnarrowed`, `contextual`, …) append when a consumer needs them; Pyrefly's
  `get_expected_type_trace` already reaches the first (**Proposed**).

> Decision: ADR-0047, ADR-0046, ADR-0015


### §3.5.1 Type observations and class order

**Implemented** and **Tested** (`cpg-core/tests/syntax.rs`, `type_shapes`).

- `has_type` is a derived edge over `type_observations` that keeps the role on its evidence row.
  It is never an editable copy.
- **Declared types** come from native annotation data, never from TSP `getDeclaredType` (which
  returns the computed type). An observation is `declared` when the subject has an annotation (a
  parameter's `annotation_text`, a `def`'s return annotation): its type is Pyrefly's reading of
  that annotation, read directly through `Answers::get_annotation`. A subject without one gets
  Pyrefly's computed type with `declared` false. `Key::ReturnType` is the computed return, which
  for an annotated `async def` that is not a generator wraps the annotation as
  `Coroutine[Any, Any, <annotation>]`; the declared return is the annotation itself.
- **Pyrefly's MRO** is kept exactly as reported. It excludes the class itself and `object`, so it
  is labelled as ancestors, not as a complete runtime MRO. It is never re-derived by
  topologically sorting base edges.


### §3.6 Resolution is a set

**Implemented** and **Tested** (the derived-table snapshots; `modality_follows_the_variant_table`).

- **A call's targets form a `ResolutionSet`,** carrying:
  - `status`;
  - `domain`;
  - `has_unresolved_remainder`;
  - `candidate_set_complete_under_model`;
  - Pysa's unresolved reasons (Interface-checked against the pinned source).
- **Targets record their invocation phase.** Candidate targets are `modality = candidate`.
- **Callable values that may be invoked** (Pysa `ifCalled`) become `potential` targets on a
  `Reference`, not `CallSite`s.
- **Synthetic sites** (Pysa `artificial-call`) carry `origin = synthetic_model`.
- **Override dispatch is never a single callee.** A Pysa `Target::Overrides(f)` is a `candidate`
  target to `f`, and its resolution has `candidate_set_complete_under_model = false`: any override
  of `f` can be reached, including subclasses outside the release. Pass A never reports it as
  `direct_delegation` without further evidence (§4.2.3).

> Decision: ADR-0046


### §3.7 Coverage and boundaries

**Implemented** and **Tested** (the `coverage:*` rule cases; `cpg-extract/tests/coverage.rs`).

- **`coverage(snapshot_id, run_id, scope_kind, scope_node_id, fact_family, status, reason)`**
  - `scope_kind` is release, module or callable (or document, for `docs`).
  - Every family an extractor declares gets a row for every module in scope.
  - A module an extractor never reached is `failed` or `unavailable`, never absent.
- **`boundaries(snapshot_id, fact_id, subject_node_id, fact_family, boundary_reason, detail)`**
  - Resolution issues and analysis stops, with one row per stop.
  - Analytics and briefs read these rows instead of treating "the analysis stopped" as "the
    library has nothing more".
- **Two channels, one fact each.** `coverage` and `boundaries` describe the producer's run. Gaps
  that Stage C/D finds are `reason` columns on the derived rows (§3.2). Where both describe one
  fact (a call with no Pysa record), a rule requires them to agree per call site, both ways (§8).
- **What a projection can cite as completeness.** These state what the graph covers and where it
  stops:
  - `coverage` per family and module;
  - `boundaries`;
  - the unresolved remainders on `resolutions` and `argument_resolutions`;
  - the `external_module` and `external_symbol` nodes: the edge of the analyzed universe, with no
    outgoing call edges because their bodies are not analyzed;
  - `synthetic_callable` nodes: functions with no body in source (a dataclass `__init__`), so a
    call path ends there;
  - `graph_gaps`: the raw rows the graph does not represent. It is empty today and stays a
    published table, so a future family with unrepresented rows must declare them.

  A projection's spec (§5) states which of them it accepts. A non-finding outside that coverage
  is never read as absence (CI-04, CI-08).

> Decision: ADR-0047


### §3.8 Graph catalog and edge registry

**Implemented** in `cpg_schema::graph` (registry, catalogs, rules) and `cpg_core::udf`, and
**Tested**:
- `cpg-core/tests/graph.rs` on the `graph_shapes` fixture: the catalogs as an insta snapshot; an
  isolate; two parallel call sites giving two edge ids; a self-loop; a cross-file SCC and a
  diamond found by a petgraph projection built from the catalogs, with each arc's lineage; an
  unresolved call with no edge; a higher-order `potential` edge; `f(g(x))`'s argument distinct
  from the inner call; a variable export with its reason; identical catalogs across two locations
  and the reversed module order;
- `cpg-core/tests/syntax.rs`: the placed tree of `syntax_shapes`; every name of `lexical_shapes`
  with its resolution and bindings; every observation, type variable, class and record field of
  `type_shapes`, two unrelated `T`s as two terms with two binders, the recursive alias as one
  finite term;
- `a_corpus_documents_its_library`: the selection, passages, a code block inside an MDX
  component, links, mentions by class and an other-library member refused, an unparsable
  document's coverage, and the usage code (a test module and a materialized block reaching the
  release's own declarations, the release class one term); the same corpus in two places is one
  context and run, and another library release is another corpus; a tree shadowing the release
  fails; an empty glob is refused;
- every registry rule rejects an injected violation (below).

This is how the typed family tables become a graph without a second authority, following the
code-intelligence profile
(`docs/design_review/design_principles/profiles/code-intelligence/principles.md`, CI-01–CI-05,
CI-09).

**The registry** (`cpg_schema::graph`) is the one declaration the catalogs, references and graph
rules are generated from (DP-16).
- **Per node kind:** its **existence source**, a relation independent of every column that
  references the node; its `node_id` column, `module_node_id` and existence `fact_id` column.
- **Per edge kind:**
  - its source relation (a family table or a derivation);
  - its source and target columns, with the allowed endpoint kinds;
  - its **direction meaning**;
  - whether parallel edges are allowed;
  - its **derivation class**: `extracted` (one provider row), `analyzer` (a provider's
    resolution), `joined` (a Stage-C/D join), `recognizer` (our analysis);
  - its evidence and support `fact_id` columns;
  - its ordinal;
  - its `edge_id` discriminator.

**The catalogs** (family `graph`, Stage D, after every other derivation):
- `nodes(snapshot_id, node_id, node_kind, module_node_id?, existence_fact_id)`: one row per node
  of every kind, from the existence sources. So an isolate (a public function nobody calls) is
  present.
- `edges(snapshot_id, edge_id, edge_kind, src_node_id, src_kind, dst_node_id, dst_kind, ordinal?,
  evidence_fact_id, support_fact_id?)`: one row per relationship. Payload, including phase,
  receiver, keyword and role, stays on the source row that the evidence cites.
- Ids come from `lctx_id` (§3.4.1). Edges get no `facts` rows (§B6).

**Edge kinds by family.** Direction reads "source → target".

| Family | Edge kind: source → target (derivation) |
|---|---|
| API, calls and dependency context | `declares`: module/class/function → class/function (extracted). `overload_of`: stub → callable (joined). `stub_for`: `.pyi` declaration → the `.py` declaration the `exports` seed rank picks, one shared rank (joined). `has_parameter`: function → parameter, ordinal (extracted). `exports`: export → declaration, external symbol, module or external module, one edge per access file, the file as discriminator (analyzer; parallel). `encloses_call`: owner → call site (extracted). `has_argument`: call site → argument, ordinal (extracted). `call_target`: call site → function, synthetic callable or external symbol (analyzer; discriminator `payload_id`; phase, receiver and modality on the evidence row). `higher_order_target`: argument → callable (analyzer; `potential`). `base_class`, `mro_entry`: class → class or external symbol, ordinal, MRO as reported (analyzer). `overrides`: function → function or external symbol (analyzer). `declared_in`: external symbol → external module (joined) |
| `syntax` | `ast_child`: module, declaration, call site or syntax node → the placed child, ordinal (extracted; one parent per node). `argument_value`: argument → its value's placed node (joined; one per argument of a call outside an annotation). `site_target`: the syntax node or call site at a Pysa attribute, artificial or format-string site → function, synthetic callable or external symbol, **not** `potential` records (analyzer; parallel, discriminator `payload_id`; `synthetic_model` for artificial sites) |
| `lexical` | `owns_scope`: module, declaration, lambda or comprehension → scope. `lexical_parent`: scope → enclosing scope. `binds`: scope → binding, ordinal. `introduces`: binding → the declaration, parameter or placed statement that makes it (recognizer). `reads_binding`: reference → binding in its own or the module scope (recognizer; `candidate` when several). `captures`: reference → a binding of an enclosing function scope (recognizer). `reads_builtin`: reference → the builtin function or class (recognizer). `shadows`: binding → the previous event of its name in its scope (joined). `potential_target`: reference (Pysa identifier sites) or syntax node (Pysa `if_called` at attribute sites) → callable (analyzer; `potential`). `imports_module`: module → the module an import names (joined; one per alias). **Deferred:** `imports_symbol` (Pyrefly's `find_definition` at an alias; reopen when a consumer needs symbol-level import targets, since `exports` already trace re-export origins) and separate `global`/`nonlocal` edges (their effect is the declared scope of the binding and the resolution) |
| `types` | `has_type`: parameter, function, call site, argument or `raise` syntax node → type term (analyzer; one per subject; role and `declared` on the evidence row). `type_arg`: term → term, ordinal (analyzer; parallel, discriminator the `type_arg_role`, since a callable returning its first parameter's type joins one pair twice at ordinal 0). `type_class`: term → class or external symbol (analyzer, through `type_class_targets`). `has_field`: class → field, ordinal (analyzer). `field_type`: field → term (analyzer) |
| `docs` | `contains_passage`: document → passage, ordinal; `contains_block`: passage → code block, ordinal (extracted). `mentions`: passage → export, class or function, at the byte offset (recognizer; parallel; `exact` or `lexical` and the modality on the evidence row). `block_module`: Python code block → its materialized module (joined). A usage call's `call_target` reaches the release's own node directly, because the corpus names the release's files by `@path` |

**Rules** generated from the registry, or hand-written beside it (§8):
- **Endpoint kinds:** both ends exist in `nodes` with an allowed kind, and `src_kind`/`dst_kind`
  equal it.
- **Node-valued references**, from the registry's `node_columns`: every node-valued column of every
  family table names a node of an allowed kind in `nodes`. The hand-kept `REFERENCES` holds only
  fact, provenance and composite references. A reference whose target is built from its own
  source column is never generated, because it cannot fail.
- **Evidence and support** exist, in the kind's evidence table.
- `key:nodes`, where one id with two kinds is a collision, and `key:edges`.
- **Lineage from raw rows:** each source row yields exactly its declared edges, or its derived row
  carries a provider's reason. A lineage rule that re-reads its edge kind's own unfiltered source
  guards an edit of that edge's SQL rather than a data condition; such rules are declared **edit
  guards** (`cpg_schema::rules::EDIT_GUARDS`, counted apart in §8).
- **Partition of `pysa_calls`:** every row is a call-site row (lineage), an unresolved remainder
  counted on its `resolutions` or `argument_resolutions` row, a syntax site row (lineage through
  `site_targets`), or an identifier site (lineage through `identifier_targets`). No row is a gap;
  the gaps rule is an edit guard over an empty table. The remainders rule rejects a resolution
  that stops counting its remainder.
- **Placement:** every declaration, and every call outside an annotation, has its `syntax_nodes`
  row; a node is placed once; a placed child lies within its placed parent, in the same module; a
  Pysa site with no node, or a name with no reference, is a `typed:*` violation (never a reason).
- **Lexical:** `id:scopes`, `id:bindings`, `id:references`; `typed:reference_resolutions` (no
  binding, builtin or reason); `typed:identifier_targets`; `typed:import_targets` (a reason only
  from Pyrefly's finder, so an import we misname fails it); every reference's name is a placed
  syntax node.
- **Types:** `id:record_fields`; `typed:type_class_targets`; `typed:type_binders`; lineage for every
  observation, term argument, class-bearing term and record field.
- **Source text and role (ADR-0015):** `semantic:source-text` (the text is present exactly when
  the bytes are UTF-8, with `byte_len` bytes); `semantic:source-role-by-run` (the `release` role
  exactly in a run that declares `exports`).
- **Docs:** `id:documents`, `id:passages`, `id:code_blocks`; `typed:mention_targets`; a run's
  release has modules or documents, and a document's release has a run; a run declaring a family
  has what it covers (`coverage:family-has-scope`: a document for `docs`, a module for a code
  family); `coverage:complete` expects a `docs` row per document; `unique:release-paths` (an
  attempt's releases share no path, so a `@path` module reference names one file);
  `unique:type_terms` (one term id, one kind, detail and display, however many runs emit it).
- **Docs components:** the CHECKs `span_order`, `parent_before`, `inner_within` and
  `lead_within`; `semantic:doc-component-parent` (a parent exists, precedes its child, contains it
  and is one level up; a top-level component is at depth 0); `semantic:doc-component-in-passage`;
  `semantic:doc-attribute-component` (an attribute's component exists, and it has no name exactly
  when it is a spread and no value exactly when it is bare); and `semantic:docs-span-in-document`
  (every passage, code block, component, link and mention lies within its document's bytes).
- **Typed targets:** a null target carries a reason.
- **Ids:** the Rust recipes equal their SQL form (`id:*`).
- **Every rule is falsifiable or declared.** `every_rule_is_exercised_or_declared_an_edit_guard`
  requires every hand-written rule to reject an injected violation (in `compile.rs`, `graph.rs`,
  `each_graph_rule_rejects_a_doctored_catalog` or `the_corpus_rules_reject_their_violations`),
  every generated template to have a case, and every other rule to be a declared edit guard.

**The registry as data.**
- `edge_kinds` publishes each kind's derivation class (`derivation_class`: extracted, analyzer,
  joined, recognizer), direction meaning, parallel policy, evidence table and endpoint kinds with
  every snapshot. A projection selects by them from the store, not from its own build.
- `graph_gaps` publishes each raw row the graph does not represent, with its reason
  (`not_requested`) and the family that will represent it. It is empty today. Nothing is left out
  silently.

**Scale** (**Measured**, FastMCP 4.0.5 with its corpus, `just pilot`, 2026-09-23; the last
whole-CPG measurement, before the behavior-model families): 905,648 nodes and 1,449,162 edges,
every rule passing; `edges` derives in 1.0 s and `nodes` in 0.5 s of a 29.9 s compile. This
bounds ADR-0047's revisit trigger on catalog derivation cost; the current tree has not been
re-measured ([STATUS](../../../STATUS.md)).

**What the catalogs never hold:**
- transitive closures, paths or all-pairs results (CI-08);
- a merged "best" target in place of a candidate set;
- graph-local indices (§5).

> Decision: ADR-0047
