# Facts and identity

<!-- owner-intro -->

## §3 Fact model

**Proposed** unless marked. Source: IP L427–L746 (ontology and identity), L975–L1143 (Arrow
contract), L1969–L2036 (v1 slice).


### §3.1 Layers and node kinds

> Decision: ADR-0014

The full ontology below is the **target model**. Increments introduce kinds only as their
consumers appear (§3.2).

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

**The v1 node kinds** (the `node_kind` codebook, grown by CPG slice; §3.8; ADR-0014):

| Slice | Node kinds |
|---|---|
| C1 | `module`, `class`, `function` (every `def`, overload stubs included), `parameter`, `call_site`, `argument`, `export` (a public access path), `external_module`, `external_symbol`, `synthetic_callable` |
| C2 | `syntax_node` |
| C3 | `scope`, `binding`, `reference` |
| C4 | `type`, `field` |
| C5 | `document`, `passage`, `code_block` |

- `ExternalSymbol` and `SyntheticCallable` above are these kinds.
- A `ResolutionSet` is keyed by its call site (§3.6), so it is not a separate node.
- One syntactic occurrence has one node id and one kind. An argument or a reference is a role,
  identified from its carrier (§3.4.1).


### §3.2 Fact families and authority

> Decision: ADR-0013

- **Family tables are authoritative.** Each fact family is a set of typed Arrow tables. The
  family is also:
  - the Delta table group;
  - the coverage unit;
  - the unit an extractor declares it produces.

  "Fact family" and "relation family" are the same term.
- **The family → node/edge mapping.** `cpg-schema` declares how each family maps to node and edge
  kinds: kinds, endpoint kinds, role and ordinal columns. Endpoint-kind validation (§8) consumes
  this mapping.
- **The node and edge catalogs** (ADR-0014; **Implemented** and **Tested** in C1, §3.8).
  - `nodes` and `edges` are Stage-D derived tables of family `graph`, generated from one registry
    (§3.8).
  - They carry identity, kind, endpoints and evidence, never a payload. So the family tables stay
    the only authority, as the typed extension tables of the catalogs.
  - Projections (§5) select from the catalogs by kind, derivation class and evidence, and join
    the extension tables for payload.
- **One producer per table.** A fact table is written by one producer: one extractor surface, or
  one Stage-C/D derivation (§4.1). `runs`, `contexts`, `producers` and `facts` are registries each
  producer appends its own rows to. `releases`, `distributions` and `source_files` are written
  **once per extractor run**, which carries Stage A's output; later producers reference
  `release_id` and never append (ADR-0013, amended for C5). An attempt holds several **extractor** runs only over
  distinct releases (the library and its corpus), since the family keys carry no run. The
  `lctx-compiler` run (analytics and synthesis, ADR-0019) and the `manual-review` run are over
  the library release too, but they declare no families and write no `releases`,
  `distributions` or `source_files`; the extractor-only rules are scoped to runs that declare a
  code family. The corpus run
  has its own context (its search path adds the tree ahead of site-packages) over the same
  environment, whose `distributions` it lists too. What both runs assert about one thing (a
  dependency module or definition, a type term and its structure) is one node and one edge,
  from its first fact; the other run's fact is a counted duplicate in the lineage. `distributions` is keyed by `context_id`:
  the environment belongs to the context.
- **Merged tables are derivations.** Where two providers contribute to one logical record, each
  writes its own raw table. The merged table is a DataFusion derivation
  (`relational_derivation`) that carries keys, both `fact_id`s and what the join decides: a
  mapping, a status, a reason, or an aggregate of raw values (`signatures.form` and
  `function_key`, `resolutions.unresolved_reason`). It never copies a raw payload column
  unchanged, so the raw table stays the authority (**Implemented**, `cpg_schema::derived`,
  slice 2; **Tested** by the derived-table snapshots).
- **Gaps are recorded in the derived row.** Where a null needs explaining, a `reason` column
  holds it: `provider_disagreement`, `missing_evidence`, `outside_provider_model`,
  `no_source_declaration` (a function Pysa describes without a `def` of its own) or
  `unreachable_in_context` (a `def` the context never binds). `boundaries` stays the extractor's.
  A null node with a null reason has one declared meaning per table: in `exports`, the origin is
  not a `def` or `class` of the release; in `call_targets`, the target is outside the release (a
  rule rejects a release target with neither).
  - **From C1 (ADR-0014; Implemented):** these null meanings are retired.
    - Every target is a typed node, with a target outside the release being an
      `external_symbol`. `exports.target_node_id` is the declaration, the dependency definition,
      or the module (of the release or a dependency) the path names.
    - A reason appears **only where a provider says why**: Stage C's own reason; for an export,
      Pyrefly's symbol kind of the origin, or `missing_evidence` when Pyrefly traces no origin or
      records no kind. Since C3 a variable-like origin in a release module (a variable, attribute,
      constant, parameter, type parameter or type alias) targets its module-scope `binding`: the
      last binding event of the name by ordinal, unbinding events excluded. `variable_origin`
      remains only for a variable-like origin in a dependency module, which has no binding node;
      a release-origin variable with no binding is our failure (C3 review F2).
    - Any other unmapped target is our own failure to find what the provider referenced. It keeps
      a null reason, and a `typed:*` rule rejects the snapshot. There is no catch-all reason
      (ADR-0014 review F1).

| Family | Tables | First increment |
|---|---|---|
| `provenance` | `releases` (library, requirement, lock digest, or a source tree's label), `distributions` (every installed distribution: version, artifact sha256s, `RECORD` digest, in the release or not), `source_files` (C1: with its release `distribution`; ADR-0015: its `text`, the bytes every span indexes, null only when not UTF-8, and its `role`: `release`, `example`, `test` or `doc_block`), `contexts`, `producers`, `runs`, `facts`. C1: `context_modules` (each dependency module a fact references: name, site-relative path or Pyrefly's bundled typeshed, distribution and version) and `context_definitions` (the Pysa definitions of those modules: the existence source of `external_symbol` nodes; §3.8) | 1 |
| `exports` | raw: `declarations` (Ruff: qualified name, kind, parent, span, docstring text and span, `is_overload`), `export_syntax` (Ruff: import aliases, `__all__` statement span; syntax evidence only), `public_names` (Pyrefly: access path → origin and its file, `via_dunder_all`; §4.2.3). Derived: `exports` (public access path → the seed declaration in the origin's file: an implementation before an `@overload` stub, then the one Pysa describes, then the last in source order; one row per `public_names` row, so a `.py`/`.pyi` pair gives an access path two rows, one seeding each file, told apart by `source_files.is_stub`; Pass A seeds from the source row) | 1 |
| `signatures` | raw: `parameter_syntax` (Ruff: ordinal, name, default text and span, annotation text), `parameter_docs` (slice 2.1, **Implemented** and **Tested**: Pyrefly's `parse_parameter_documentation` over each `def`'s docstring, Sphinx and Google styles; one row per documented parameter of the signature, its text as Pyrefly normalizes it, its span the description's verbatim bytes. The entry is anchored on its own header line (Sphinx `:param [type] name:`, Google `name:` or `name (type):`), never on a substring of the name, and runs over the lines indented deeper than the header; it is accepted only when its trimmed lines equal Pyrefly's text, or begin with it: Pyrefly's Google parser ends an entry at a continuation line holding a colon, and such a description is **extended** to its entry's end, its text then those lines (slice 2.1 review F2, F3). A description no header, or more than one, locates is a `signatures` boundary (`provider_disagreement`), never guessed; `docstring_tests` in `walk.rs`; on the pilot (Measured, 2026-09-23): 1,580 rows, 46 of them extended, and one description (`truncation_suffix`) a boundary), `pysa_functions` (Pyrefly: function key → name span, flags, signature count), `parameter_semantics` (Pyrefly Pysa undecorated signatures: kind, required, annotation), `class_ancestry` (Pyrefly: bases and reported MRO), C1 `pysa_classes` (one row per class, so a class without bases is keyed). Derived: `provider_node_map` (Stage C, name-span join), `signatures` (per `def`: its callable, stubs rolled up to the implementation, and its Pysa signature index), `parameters` (Ruff ⋈ Pysa on the ordinal); C1 `provider_class_map` (Stage C for classes), `synthetic_callables`, `ancestry_targets` and `override_targets` (bases, MRO entries and overridden methods resolved to nodes, a reason where an end does not resolve) | 1 |
| `calls` | raw: `call_syntax` and `arguments` (Ruff: span, owner, ordinal, keyword, starred, expression span; `call_syntax` rows are the call sites), `pysa_calls` (Pyrefly Pysa call graphs: targets, receiver, phase, unresolved reasons). Derived: `resolutions` (§3.6, one per call site), `call_targets` (joined on the full call-expression range, §4.2.3). C1: `arguments.node_id`, `pysa_calls.payload_id`, `argument_resolutions` (higher-order arguments: status and unresolved remainder); `call_targets` typed (a declaration, a synthetic callable or a dependency definition, with the higher-order argument). Pysa rows at non-call sites (property accesses, identifiers, artificial and format-string sites) stay raw, as a declared pending class of the lineage rule, until C2 and C3 give them nodes | 1 |
| `embedding_cache` | `embedding_cache` (spec_hash, input_hash, vector as `List<Float32>`, model identity). Global and append-only; not snapshot-qualified: its read mode is `global` (§6.2); written by an insert-only MERGE (ADR-0017 amendment); the key is unique | 1 |
| `coverage` | `coverage`, `boundaries` (§3.7) | 1 |
| `graph` | derived: `nodes`, `edges` (§3.8). Not a coverage unit | C1 |
| `syntax` | **Implemented and Tested (C2; revised by its compact review).** Raw `syntax_nodes` (Ruff): every statement, the clause nodes (`elif`/`else`, `except`, `case`, `with` items) and **every expression outside annotations** (the IP 2.1 exhaustive-exporter contract; placement depends on the source alone, never on a provider). Each row has its parent (the nearest placed ancestor), owner, field (`syntax_field`: body, test, orelse, handler, exc, cause, default, argument, …), ordinal in that field (a statement's block index), span, `kind` (Ruff's `NodeKind`, the `syntax_kind` codebook, an exhaustive match) and detail (a name, attribute, operator, literal as written, or a handler's name). A `def`, a `class` and a call are placed under their declaration and call-site ids; nothing inside an annotation is placed. Derived: `site_targets` (each Pysa attribute, artificial and format-string record → the deepest syntax node at its span, a chained comparison's pairwise site → its comparison → its typed target; a span with no node is our own failure, never a reason). Consumers: Pass B guards, raises, handlers and defaults; Pass C straight-line regions; FCA raised types (their type is C4's) | C2 |
| `lexical` | **Implemented and Tested (C3; revised by its compact review).** Raw, from our recognizer (surface `lctx-lexical`, `recognizer`, inside the Ruff walk): `scopes` (module, class, function, lambda, comprehension; owner, and parent = the scope the scope's position evaluates in, so a lambda in a default or decorator belongs to the enclosing scope), `bindings` (every binding event per scope, ordinals in source order: kind, site, span, the assigned value's span, and the innermost branch Pyrefly decides statically that it sits in: the deciding test's kind (`type_checking`, `version_info`, `platform`, `constant`, `combined`) and whether Pyrefly analyzes or prunes that branch, clause by clause exactly as `SysInfo::pruned_if_branches` decides, recursively: an `if` inside a pruned clause is never walked, so its bindings take the pruning clause's mark (H1 C1, H1 review F1: `SysInfo::evaluate_bool` per clause, the kind from the expression tree; `static_marks_agree_with_pyrefly_pruning` checks every fixture `if`, and `static_polarity_is_pyrefly_recursive_pruning` every assignment binding, against Pyrefly's own pruning, nested cases included); every event under `global`/`nonlocal` binds in the declared scope, a `nonlocal` target decided once every binding is known; a repeated name in one declaration is one event; the module's implicit globals and a method's `__class__` cell are `implicit` events), `references` (every name load outside annotations, and an augmented assignment's target, which reads before it binds; a role of its placed name, with its parent and field; nothing inside an annotation opens a scope or binds), `reference_resolutions` (Python's scoping rules as modelled: the scope's own bindings, else the nearest enclosing function scope with class scopes skipped, else the module, else the star imports whose wildcard set holds the name, else a builtin; comprehension first iterables and function defaults in the enclosing scope; walrus in the nearest non-comprehension scope; flow-insensitive candidates, except that a module or class body reading a name it binds only later also reads it from outside, as `LOAD_NAME` does; a builtin names itself, a builtin variable reads `variable_origin`, anything else `unresolved_target`). Every name set from outside the module's text is Pyrefly's: its `ImplicitGlobal` set, its `builtins` definitions that are real public names, each star import's `Transaction::get_wildcard` set (a star module Pyrefly cannot find stays a candidate for any otherwise unbound name). **Not modelled:** PEP 695 annotation scopes (class and alias type parameters), the implicit unbinding at the end of an `except … as` handler (C3 review O4, O5, deferred). `export_syntax.resolved_module` is each import's absolute module by Pyrefly's own `ModuleName::new_maybe_relative`. Derived: `identifier_targets` (Pysa's identifier sites → the reference at their span → typed target), `import_targets` (each import → the release or dependency module it names; `unresolved_target` only where Pyrefly's finder says not found, a `context_modules` row of origin `not_found`, or where the import climbs past the top package). Consumers: Pass B binding order (§4.2.4), Pass C bindings and values, the import graph, `if_called` targets, variable exports | C3 |
| `types` | **Implemented and Tested (C4; revised by its compact review).** Raw, from Pyrefly's native types (surface `pyrefly-types`, `native_structural`). `type_terms`: one row per distinct term, its id a Merkle hash over Pyrefly's own structure and identities (kind, detail, class pair, children with their roles; §3.4.1); the display is a label, and only a display-only kind (`other`, `truncated`) hashes it. Two structures that share an id but differ in kind, detail or display fail `unique:type_terms` (several runs may observe one term; `nodes` keeps it once). A class is a (module ref, class key) pair, an enum member keeps its class, and a recursive alias is a reference to its name, so a term is finite; a depth cap (32) makes that a guarantee. A type variable's id is Pyrefly's own identity (`QuantifiedIdentity`), so one variable is one term wherever it is observed and two unrelated `T`s are two; its bound, constraints and default are its children (`type_arg_role` `bound`, `constraint`, `default`). `type_term_kind` maps every `Type` variant by an exhaustive match; solver-internal and experimental variants are `other` (`display_only`). `type_term_args`: each child at its role and ordinal; a callable parameter carries its name, kind and requiredness. `type_observations` (§3.5.1): each parameter's type, each `def`'s return (`Key::ReturnType`; an annotated one is the annotation, §3.5.1), each call's result, each argument's value and each `raise`'s exception (Pyrefly's expression trace at the exact span; calls in annotations excluded). A subject Pyrefly records no type for (a `TypeVar(...)` declaration, a call in a lambda body, a branch Pyrefly skips for the platform) is a `types` boundary (`missing_evidence`) and the module's coverage is `partial`; a bare `raise` has no exception to type. `record_fields`: the fields a dataclass, attrs or pydantic class, `TypedDict` or `NamedTuple` declares itself (an inherited field a subclass assigns in a method stays its base's), with the flags as the field states them (default, `init`, alias and `kw_only` through `ClassField::dataclass_flags_of`; `TypedDict` required and read-only); the constructor they imply is Pyrefly's synthesized `__init__`, a `synthetic_callable` with its `parameter_semantics`. Derived: `type_class_targets` (each term's class → a release class or dependency definition; a miss is our failure, never a reason) and `type_binders` (each source-anchored variable → the innermost release declaration, type-alias or assignment statement holding Pyrefly's scope anchor, a joined fact; `scope_boundary` for an anchor outside the release). Consumers: Pass C type compatibility, FCA parameter, return and raised types. Controls from record fields are **not yet read**: Pass B follows parameters only (C4 O3, deferred until a seed's controls are a record's fields) | C4 |
| `docs` | **Implemented and Tested (C5a, C5b).** A corpus run over the library's upstream tree at its pinned commit (§4.0), in the library's environment; its search path is the tree, then site-packages (the library run's search path), so a module both runs import is one file. Raw, parsed by markdown-rs 1.0 (MDX constructs and frontmatter; byte offsets, probe P5): `documents` (each selected file: path, digest, frontmatter title, whether it parsed; one that does not parse is `unavailable` with markdown-rs's message), `passages` (each root-level heading's section, whatever its depth, to the next, so a document's passages partition it, with level, heading and heading path; the text before the first heading is passage 0), `code_blocks` (fenced blocks at any depth, MDX components included: language, meta, code, digest; each in the passage its start falls in) and `doc_links` (URL, title, text). **Components (the holistic assessment's A3; Implemented and Tested, 2026-09-24):** `doc_components` holds each MDX JSX element, flow or text form (`component_form`), in pre-order with its parent ordinal and depth, its name (none for a fragment), its span, its inner span (first child to last; none when self-closing) and its lead (the first direct paragraph), in the passage its start falls in; `doc_component_attributes` holds its attributes in order, each by kind (`attribute_value_kind`: a literal's value as written; an expression's as source text, never evaluated; a bare name without a value; a spread without a name). Fenced and inline code never yield a component, and a heading inside one opens no passage. Components are span facts, not graph nodes. On the pilot: 1,250 components and 1,605 attributes (Measured, 2026-09-24). Consumers: §10.3's documented warnings and `<ParamField>` parameter descriptions. `mentions` (our recognizer, `lctx-docs`) against the library run's public names and declarations, two classes never merged: `exact` for inline code (or a dotted prose token) that is a public access path, an origin path or a public class's member (`FastMCP.tool`); `lexical` for inline code that is a bare public name of a class, function, method or module, or such a name in prose when it is distinctive (an underscore, or two capitals and a lower-case letter), one `candidate` per origin (re-exports collapse to the shortest access path). Embedding-based linking is §9.7's. Derived: `mention_targets` (→ the `export` node, or the member's release declaration by the seed rank). Consumers: exact doc links to APIs and extractive brief text, §9.4 co-mention. **The usage run (C5b)** is the same corpus run's code: the selected examples and tests, and every Python code block materialized as a module of its own (`_lctx_blocks/d_<document>/block_<n>.py`, named in `code_blocks.module_path`), with every code family but `exports`. The corpus names each installed file the release's distributions own by the library run's own site-relative `@path` (C5 review F2), so a usage call's target is the release's own declaration or synthetic callable (the same Pysa key, probe P4), a release class is one type term whichever run observes it, and an import of a library module targets the library's module node. A tree that holds its own copy of the package ahead of the installed one (a flat layout) would cut the usage code off the release, so it fails the compile, naming the module. Consumers: Pass C examples and tests, §10.3–§10.5 usage patterns, §9.4 co-use. Each usage module's text and role are in `source_files` (ADR-0015, closing C6 review F2), so a snippet and whether it is an example, a test or a doc block are read from Delta alone | C5 |
| `findings` | ADR-0019 (contracts in `cpg_schema::findings`; provenance in-row, no `fact_id`; not a coverage unit; outside the `nodes`/`edges` catalogs): `analysis_invocations` (method, parameters as canonical JSON, projection digest, seed, diagnostics), `findings`, `finding_members`, `witnesses` (path steps keyed by node ids, `edge_id` as lineage), `evidence` (evidence_id → one of: fact, span, passage, example, fixture run, with resolved text), `assertions`, `assertion_support` (assertion → finding / evidence, role `support` or `scope`), `briefs` (with `review_state`, outside `brief_id`), `brief_assertions`, `brief_members`, `brief_documents`, `assertion_policy`; and `public_paths` (the holistic assessment's A1: every public path under the config's roots, own and inherited, with its export, kind, `own` and one `preferred` per node; §9's opening). A usage pattern is a `usage_pattern` assertion, not a table (D24) | 1 |

**The behavior model's families** (ADR-0022, 2026-09-24; landing by plan stage; Stage 1's tables
**Implemented** as analysis tables (deviation log B6) and **Tested**, the rest **Proposed**;
the semantics are in §3.9):

| Family | Tables | Stage |
|---|---|---|
| `behavior` | Analysis tables (ADR-0019; deviation B6), written after `public_paths`; no coverage rows: `operations.behavior_status` is every public callable's verdict, and `operation_facet_status` says per operation and facet whether its rows are complete. First today's in-session relations: `argument_flows`, `guards`, `parameter_reads` and `handoffs` (`cpg_schema::flows`); then `delegations` (depth-1 call arcs with modality) and `behaviors` with the control fates Pass B finds (per public operation parameter: forwarded to which formal, a literal supplied to a callee's formal, raises when, or unfollowed with a reason). **No negative fate:** Stage 1 sees parameter reads only at call arguments, so a parameter with no fate is `not_analyzed` for its other channels (stores, returns, tests), never "unused" (the ADR review's F2). **One verdict per arc** (increment 3's deep review, F1): a behavior whose path crosses a candidate or potential arc is `unknown` (`override_dispatch`, `ambiguous_binding`), as the delegation over it is; `behavior_steps` keeps every behavior's path hop by hop (F6). **Stage 2.6 (Implemented and Tested, 2026-09-24)**, over the flow IR (`cpg_core::flow_model`): `value_flows` (per sink, the parameters or receiver fields whose value reaches it, identity, derived or only through a call, under a condition in the function's places; a rebound fallback is followed), `field_accesses` (every `x.f` read or written, by name), `ambient_reads` (reads of a module-global singleton's fields at the resolved key, with the read phase: `import`, `construction`, `snapshot`, `per_call`), `dynamic_accesses` (the getattr family and kin, and the class or modules they reach under the model), `raise_sites` (every `raise` with its region's condition, the parameters its tests read through the flow IR, and whether it `escapes` its function; only an escaping raise is a guard or a fate), `singletons`, and `negative_premises` (per parameter, field and setting). Behaviors gain `derives`, `stores` (to a field or dict entry; a field stored unchanged composes with its reads in any relative's method at depth 2, deviation B17), `returns`, `reads_setting`, `is_read` (refuted under the model only where its premise holds: for a parameter, a concrete, runtime-reachable body no release subclass defines again; else `unknown` with `abstract_body`, `runtime_unreachable` or `override_dispatch`) and `tests` (the literals of the innermost test that reads a parameter, deviation B18), each with its `condition`; an operation the runtime view cannot reach is `unknown` (`runtime_unreachable`), with every claim about it; a claim reached only inside a call (a callee, receiver or argument) is `unknown` with `call_transfer` until Stage 3's summaries (ADR-0022 §Verdicts; deviation B15). Pass B follows the flow IR's identity sources, per-hop conditions in `behavior_steps`. Handlers, callbacks and resources are Stage 3's | 1, then 2 |
| `flow` | **Implemented and Tested (Stage 2.3, 2026-09-24).** Raw facts of the extractor run (surface `ty-flow`, `native_traversal`, `analyzer_assertion`, `normalized_structural`), from `cpg-flow` over every UTF-8 release module (ADR-0022 §The flow provider): `flow_uses` (every place load ty records, with its scope and whether it is inside an annotation), `flow_definitions` (every binding, normalized, with its value's span), `flow_reaching` (use → reaching definition or none, with its condition and `loop_carried`), `flow_values` (per sink: a definition's value, a call argument, a `return`, a `yield`, a `raise`; each use inside it, identity or derived, `through_call` when it sits inside a call, with the condition inside the expression, nested conditional expressions included), `flow_regions` (every statement's reachability, relative to its scope's entry), `flow_tests` (every test ty records as a predicate, with its span and condition: what a test reads comes from the uses inside it), `flow_attribute_loads` (every attribute load by name on any receiver, and `getattr`/`hasattr` with a literal name: what the field premise counts), `conditions` (the canonical encoding; `stated` false past the budget) and `condition_literals`. One coverage row per module: complete, `partial` (`syntax_error`) from a recovered tree, `failed` when the module already names the rename's sentinel. **Parity rules**, each with an injected case: `flow-use-is-a-reference` and `reference-is-a-flow-use`, `flow-definition-is-a-binding` and `binding-is-a-flow-definition` (declared residue: annotation and type-alias uses; `global`, `nonlocal`, `del`, implicit and star-import bindings; a class's or an alias's type parameters), `flow-reaching-within-candidates`. **Measured** on the pilot (FastMCP 4.0.5, 2026-09-24, after the Stage 2 end review's fixes): 66,954 uses, 23,898 definitions, 69,010 reaching rows, 53,466 value sources, 28,697 regions, 6,148 tests, 13,764 attribute loads, 11,184 conditions; parity 0 both ways for uses (13,715 annotation uses at Stage 2.3); 29,894 reaching pairs all within our candidates (Stage 2.3); 0.65 s. Exits and handlers are derived from regions and syntax (Stage 2.6), not a table of their own | 2 |

*Superseded:* "Deferred: CFG, dataflow and alias tables (§1.3, §13)". Flow and bounded places are
now in scope. General alias analysis stays out (§1.3). Type structure and `record_fields` are
built in C4 (ADR-0014).

**Implemented and Tested in focused cases (ADR-0029, 2026-09-25):** the dependency context also
retains a class definition named by a committed exception model when its module is already
described. This lets the model compiler cite a pinned class identity even if the analyzed source
does not itself mention that class. Context capture does not assert an exception occurrence.

**Implemented and Tested in focused cases (ADR-0030, 2026-09-25):** each retained context class
also has an attributed, ordered Pyrefly MRO relation. Resolved-empty and cyclic MROs have
explicit marker rows; publication checks coverage, order and child identity. An ancestor is
identified by Pyrefly's module and class key and only becomes a handler class identity through
its pinned `context_definitions` row. `EXTRACTOR_OUTPUT_VERSION` is 30 for this migration.

> Decision: ADR-0014, ADR-0012, ADR-0015, ADR-0019, ADR-0022, ADR-0029, ADR-0030


### §3.3 Physical profiles

**Tested** (`every_table_round_trips_through_delta_exactly`; C6 review, 2026-09-23). Every data
file is written with **zstd level 3** (H1 P6, `delta::writer_properties`; `data_files_are_zstd`),
with Parquet's default dictionary encoding and page statistics.

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
- **Tested** (spike S6, 2026-09-22): FixedSizeBinary is written as generic BINARY and read back
  through the DataFusion provider as `BinaryView`. A direct `BinaryView → FixedSizeBinary` cast is
  unsupported.
- **Read path:** BinaryView → Binary → FixedSizeBinary(16), two `cast_with_options` steps with
  `CastOptions { safe: false }`. A wrong length is an error.
- **Timestamps:** µs normalization is lossy for ns, so only µs is ever written.

> Decision: ADR-0012


### §3.4 Identity rules

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
  inverse of how Pyrefly produced them (**Tested**, spike S5: 29,279 of 29,279 exact).
- **Type variables keep binder identity:** two unrelated parameters named `T` are distinct.
- **Deduplicate repeated ingestion of the *same* assertion only.** Assertions from independent
  providers are separate facts, even when they agree.

> Decision: ADR-0012


### §3.4.1 ID derivation

> Decision: ADR-0014

**Proposed**, except the extractor's ids: **Implemented** in slice 1 and pinned by an id-recipe
snapshot (`extractor_id_recipes_snapshot`, **Tested** 2026-09-22).

**Encoding.** `BLAKE3("lctx-id/v1" ‖ len‖kind_tag ‖ len‖field …)`. Lengths are u64
little-endian. IDs are the first 16 bytes; digests are all 32. A version bump in the tag is a
migration (DP-24).

| ID | Derived from | Scope |
|---|---|---|
| `release_id` | the release distributions' names and versions and the sorted (path, sha256) of their `RECORD`-verified analyzer-readable files: what is analyzed, never the lock entry (a source tree: its label) | global |
| `node_id` | `release_id`, path within the release, then the structural occurrence path (syntax nodes: ruff `NodeKind` names and child ordinals) or the qualified name and occurrence (declarations) | stable across snapshots and runs. Syntax ids are producer-scoped: a ruff bump may rename a node kind |
| `context_id` | Python version, platform, ordered search and site-package paths (root-relative), config digest, environment digest (the installed distributions' dist-info names and every analyzer-readable file's site-relative path and content), lock digest | global |
| `producer_id` | tool, tool revision, adapter build digest | global |
| `run_id` | `release_id`, `context_id`, `producer_id`, sorted enabled families, the producer's own config digest | global |
| `fact_id` | `run_id`, record kind, subject id(s), canonical payload bytes. Provenance is outside the id: the same payload with different provenance fails the run (Tested) | per run |
| `finding_id`, `assertion_id`, `brief_id`, `evidence_id`, `invocation_id` (ADR-0019) | kind, subject `node_id`(s), canonical payload. **No config digest**, so an unchanged finding keeps its ID when parameters change; ablation diffs are joins. A finding's payload names its witness steps by call-site and callee node ids, never by `edge_id` (producer-scoped, ADR-0014 O6). An assertion's includes its sorted supports; a brief's, its seed, applicable case and sorted (section, ordinal, assertion); `review_state` is outside it. An invocation's is its method, parameters digest, projection digest, subject and seed. `capability_id` = `brief_id` | content |
| `behavior_id` (ADR-0021; increment 3's deep review, F8) | `H("behavior", operation, kind, parameter, callee, target, value, site)`: the claim. The **verdict, boundary reason, depth and `conditional` are outside it**: they grade the claim. A re-grade keeps the id, and `lctx diff` keys behaviors by id **and** verdict so it shows. Two rows under one id are an error of the scan, never merged | content |
| `summary_id` (ADR-0034; **Implemented and Tested in focused cases**, 2026-09-25) | `H("summary-flow", callable, formal, input path, output path, transfer kind, condition id, return site/region facts, ordered (step kind, evidence id, step condition id))`. The verdict, boundary and approximation are outside the identity. Raw fact ids in its steps make the path producer-scoped; two parallel proofs with equal endpoints remain distinct. | derived, producer-scoped |
| `condition_id` (ADR-0022 §Conditions) | `H("condition", encoding)`, the canonical DNF encoding | content |
| `edge_id` (C1, **Implemented** and **Tested**: `the_catalogs_hold_every_graph_shape`, `the_catalogs_are_the_same_across_runs_order_and_location`; byte-identical on a pilot rerun and relocation, C6 review 2026-09-23) | `edge`, edge kind, source and target node ids, then the kind's discriminator: an ordinal, or for a provider row joined at one site its run-independent payload digest (`pysa_calls.payload_id` = `pysa-call` over the row's payload). Never a `fact_id` | stable across snapshots and runs |
| Role and derived node ids (C1, C3, C4, C5 **Implemented**) | Argument: `argument`, call node, ordinal (Rust). Export: `export`, `release_id`, access path (SQL). Synthetic callable: `synthetic_callable`, module node, Pysa function key (SQL). External module: `external_module`, owner, owner version, module name (Rust), where the owner is the distribution whose `RECORD` lists the file and its version, else `pyrefly-bundled` and the fork revision, else `unowned` and the file's content digest. External symbol: `external_symbol`, the external module id, definition kind, Pysa key (Rust). Its qualified name is a label, because conditional definitions can share one. Reference (C3): `reference`, the name's syntax id. Type term (C4): `type`, kind, detail, class pair and type-variable identity, then each child's role, ordinal, id, parameter name, kind and requiredness; a variable's is its identity alone, a display-only kind's includes its display (Rust; a Merkle id with no SQL form, so no `id:` rule). It is producer-scoped like syntax and external-symbol ids: it hashes Pyrefly's detail text, Pysa class keys and anchor byte offsets, so a Pyrefly bump or an edit earlier in a module renames it. Field (C4): `field`, class node, name (Rust; `id:record_fields`). Document (C5): `document`, release, path. Passage and code block (C5): `passage` or `code_block`, document node, ordinal (Rust; `id:documents`, `id:passages`, `id:code_blocks`) | stable across snapshots and runs for the same inputs; Pysa keys make external symbols producer-scoped, like syntax ids |
| `snapshot_id` | a fresh random 128-bit value per compile attempt | execution identity (DP-04) |
| `content_digest` | sorted `run_id`s (each carrying its `release_id`, and the lock and environment through its context; the `lctx-compiler` run carries the analytics-config digest), compiler digest, embedding spec hash, and a digest of the sorted `(spec_hash, input_hash)` keys the snapshot used. Never the shared `embedding_cache` version, which another library's compile can move (ADR-0017 amendment). Slice 2 has the first two (**Implemented**; equal across a pilot rerun and relocation, C6 review 2026-09-23) | compares reruns |
| `compiler_digest` | the locked engines (DataFusion, Arrow, Parquet, object_store, delta-rs and its kernel, read from `Cargo.lock` by `cpg-core`'s build script) and analysis libraries (`lctx_analytics::LIBRARIES`), a hand-bumped compiler output version, the UDF version, the synthesis template version (`synth::TEMPLATE_VERSION`, which stands for Stage F's queries and templates; the analysis ledger test fails any output change made without bumping it; increment-1 deep review F1), every derivation query, declared projection digest and Pass B relation digest, the public-path relation, every table contract and every validation rule; and (the holistic assessment's A2(e), 2026-09-24) a digest of every `.rs` file of `cpg-core`, `lctx-analytics` and `cpg-schema`, computed by the same build script, so a code change no version names still moves run and producer ids (`every_compiler_source_is_hashed`). `TEMPLATE_VERSION` stays the published lineage and the ledger the alarm. Stored on every `snapshots` row (**Implemented**, **Tested** by a unit test on each input) | per build |

- **Ids in SQL** (C1, **Implemented** in `cpg_core::udf`, **Tested** by its known-answer and
  plan-time refusal tests). Stage D computes its ids with one scalar UDF, `lctx_id(kind, …)`,
  registered in every session.
  - It implements `IdHasher` exactly: the kind through `IdHasher::new`, and every other argument
    in the `opt_*` encoding (a presence byte, then the length-prefixed value).
  - It accepts Utf8/Utf8View/LargeUtf8, Int16 and Int64 (hashed as i64), Binary, BinaryView and
    FixedSizeBinary, and Boolean. UInt64 (`row_number()`), Int32 and floats are rejected at plan
    time, never cast. The kind must be a non-null text literal, and the result field is
    non-nullable; both are checked when the query is planned (`return_field_from_args`; H1 P7).
    Each argument's type is resolved once per batch, and each row continues a hasher already
    seeded with the tag and kind.
  - Known-answer vectors are shared with the Rust tests, so an id computed in Rust (the
    extractor, a later pass) equals the one computed in SQL.
  - The UDF's recipe is part of `compiler_digest`.
- **Keys are snapshot-qualified.** Uniqueness is checked on `(snapshot_id, key)`, so an identical
  rerun re-emits the same `node_id` and `fact_id` in a new snapshot without conflict (**Tested**
  at pilot scale: a rerun's `nodes`, with `existence_fact_id`, are byte-identical; C6 review R1,
  2026-09-23).
- **Collisions are validator failures** (§8), never silently merged. `nodes` keeps one row per id
  only for the kinds several existence rows legitimately assert (an export read from a `.py` and
  its `.pyi`, a dependency module or symbol, or a type term two runs reference); any other
  repeated id fails `key:nodes` (review O2), and a type term's id with two kinds, details or
  displays fails `unique:type_terms`.
- **One recipe, two places.** The ids the extractor computes in Rust whose inputs are also
  columns (argument, external module, external symbol; C3's scope `H(scope, owner)`, binding
  `H(binding, site, name)` and reference `H(reference, name node)`) are `cpg_schema::id::recipe`
  functions in the UDF's `opt_*` encoding. A generated `id:*` rule recomputes each in SQL on every compile, and
  known-answer values are pinned (review F3).
- **Producer scope.** `call_target` and `higher_order_target` edge ids take Pysa's payload,
  function keys included, so a Pyrefly bump may rename them, like syntax and external-symbol ids
  (review O6).
- **The compiler has its own run** (ADR-0019; built in increment 1 slice 1.4). Analytics and
  synthesis are a run of producer `lctx-compiler` over the library release and its context, whose
  config digest is the analytics config's and whose tool revision is the `compiler_digest`. Its
  `runs` and `producers` rows are written with the raw tables (every input is known before
  Stage B), so `content_digest` includes it. It declares no families.
- **Overloads.** A public callable with `@overload` stubs is **one** declaration node (the
  implementation), with one `signatures` row per overload (`is_overload`) plus the implementation
  signature. Seeds and briefs attach to the declaration. **Implemented** (slice 2): a stub rolls
  up to the first later `def` of its name that is an implementation or that Pysa describes, else
  the group's last stub; Pysa gives one undecorated signature per stub, in source order, and none
  for an implementation.
- **Binding choice follows the analyzer.** Where one file binds a name more than once (a
  `sys.version_info` or `TYPE_CHECKING` branch, a redefinition), the `exports` seed and the
  overload roll-up prefer the `def` Pysa describes (a Stage-C key), because Pyrefly's binding pass
  drops the branches the context decides statically. The other reads `unreachable_in_context`.
  Under `TYPE_CHECKING` that means the typed facade wins over the runtime body, a real choice
  Pass A inherits. Calls inside an unbound `def` still read `missing_evidence` (Pysa has no record
  of them). **Tested** on `derive_cases` (2026-09-22).

> Decision: ADR-0013 (superseding ADR-0007), ADR-0019, ADR-0034


### §3.5 Vocabularies and codebooks

**Implemented** and **Tested** (`registry_snapshot`, `codes_are_dense_from_zero_and_names_unique`,
the `codebook:boundaries.reason` case; C6 review, 2026-09-23).

**Codebooks** are append-only `Int16`, versioned in `cpg-schema`.

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
| `boundary_reason` | native_unavailable, unresolved_target, unsupported_unpacking, ambiguous_binding, unsupported_control_flow, scope_boundary, budget_reached, missing_evidence, not_requested, provider_disagreement, outside_provider_model (a construct the provider's model does not cover, e.g. a call inside an annotation), syntax_error (facts from a recovered tree), undecodable_source (bytes are not UTF-8), no_source_declaration (a function the provider describes that has no `def` of its own: a synthesized member such as a dataclass `__init__`, or a callable class field), unreachable_in_context (a `def` the analyzer's context never binds) |
| `pysa_unresolved_reason` | the 14 variants of Pysa's unresolved-call reason at the pinned pyrefly, spelled as Pysa spells them; appended when the pin moves |
| `evidence_status` | structurally_observed, documented, statistically_derived, fixture_checked, unresolved |
| `finding_kind`, `assertion_kind`, `analytic_method` | Defined in `cpg-schema` as their consumers land (§9, §10) |
| `node_kind`, `edge_kind` (C1+) | The v1 node kinds (§3.1) and the edge kinds of the registry (§3.8), appended by slice |
| `syntax_kind`, `syntax_field` (C2), `lexical_scope_kind`, `binding_kind`, `static_branch` (C3; `constant` and `combined` appended by H1), `type_term_kind`, `type_arg_role`, `type_role`, `record_kind` (C4) | Defined in `cpg-schema` with their slice. `lexical_scope_kind` is separate from the coverage `scope_kind`. `binding_kind` is the recognizer's binding events: Ruff's kinds it emits plus `del` and the `global`/`nonlocal` declarations |
| `fact_family` additions | `graph` (C1; not a coverage unit), `syntax`, `lexical`, `types`, `docs` as their slices land |
| C5 codebooks | `mention_class` (exact, lexical), `mention_source` (inline code, prose); `scope_kind` `document` (the `docs` family's coverage unit); `source_role` (release, example, test, doc_block; ADR-0015) |
| A3 codebooks | `component_form` (flow, text), `attribute_value_kind` (literal, expression, bare, spread) |
| A2(d) codebook | `unfollowed_reason` (rebound, computed, unmapped): Pass B's reasons, still stored as text in `finding_members.label`, read back by name, never by a default |
| `boundary_reason` addition (C1) | `variable_origin`: an export whose origin Pyrefly calls a variable-like symbol in a dependency module (a release-module origin targets its `binding` since C3) |
| C3 review appends | `binding_kind` `implicit` (a module's implicit globals, a method's `__class__` cell); `module_origin` `not_found` (an import Pyrefly's finder cannot find: a row, no node) |
| C1 codebooks | `module_origin`, `definition_kind`, `symbol_kind` (Pyrefly's export kinds, an exhaustive match), `derivation_class` |
| slice-1 extraction codebooks | `fact_family`, `scope_kind`, `declaration_kind`, `export_syntax_kind`, `parameter_kind`, `signature_form`, `argument_kind`, `pysa_site_kind`, `pysa_target_kind`, `pysa_callee_kind`, `implicit_receiver`, `ancestry_relation`; values in `cpg-schema`, whose snapshot-tested registry is authoritative |

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
  append-only codebook, unused in v1.
  - Exceptions (ADR-0014):
    - boundary facts compare two surfaces, so they are surface `compare` with
      `relational_derivation` (C1);
    - the lexical recognizer's rows are `recognizer` (C3);
    - docs mentions are `recognizer` (C5).
  - The mode is a parameter of the fact sink, never hard-coded.
- **Pyrefly-sourced values map through exhaustive matches.** Every Pyrefly enum → codebook mapping
  is a `match` with no wildcard arm, so a new upstream variant (a 15th unresolved reason, a new
  `OriginKind`) fails the build instead of degrading silently. `OriginKind` becomes
  `site_detail` text through our own exhaustive match, never upstream's `Display`.
- **`type_role` (C4)** says what an observation types, and follows from the subject: `parameter`,
  `return`, `call_result`, `argument`, `raised`. Whether the type is an annotation's or computed
  is the observation's `declared` flag, not a role. IP L576–623's contextual roles (`expected`,
  `narrowed`, `unnarrowed`, `contextual`, …) append when a consumer needs them; Pyrefly's
  `get_expected_type_trace` already reaches the first.

> Decision: ADR-0014, ADR-0012, ADR-0015


### §3.5.1 Type observations and class order

- `has_type` is a derived edge over `type_observations` that keeps the role on its evidence row.
  It is never an editable copy.
- **Declared types** come from native annotation data, never from TSP `getDeclaredType` (which
  returns the computed type). An observation is `declared` when the subject has an annotation (a
  parameter's `annotation_text`, a `def`'s return annotation): its type is Pyrefly's reading of
  that annotation. A subject without one gets Pyrefly's computed type with `declared` false
  (**Implemented**, C4). `Key::ReturnType` is the computed return, which for an annotated `async
  def` that is not a generator wraps the annotation as `Coroutine[Any, Any, <annotation>]`
  (`return_type_from_annotation`); that one rule is inverted exactly, so the declared return is
  the annotation (C4 review F2; **Tested**).
- **Pyrefly's MRO** is kept exactly as reported. It excludes the class itself and `object`, so it
  is labelled as ancestors, not as a complete runtime MRO. It is never re-derived by
  topologically sorting base edges.


### §3.6 Resolution is a set

**Implemented** and **Tested** (the derived-table snapshots; `modality_follows_the_variant_table`;
C6 review, 2026-09-23).

- **A call's targets form a `ResolutionSet`,** carrying:
  - `status`;
  - `domain`;
  - `has_unresolved_remainder`;
  - `candidate_set_complete_under_model`;
  - Pysa's unresolved reasons, which have 14 variants (Interface-checked).
- **Targets record their invocation phase.** Candidate targets are `modality = candidate`.
- **Callable values that may be invoked** (Pysa `ifCalled`) become `potential` targets on a
  `Reference`, not `CallSite`s.
- **Synthetic sites** (Pysa `artificial-call`) carry `origin = synthetic_model`.
- **Override dispatch is never a single callee.** A Pysa `Target::Overrides(f)` is a `candidate`
  target to `f`, and its resolution has `candidate_set_complete_under_model = false`: any override
  of `f` can be reached, including subclasses outside the release. Pass A never reports it as
  `direct_delegation` without further evidence (§4.2.3).

> Decision: ADR-0012


### §3.7 Coverage and boundaries

- **`coverage(snapshot_id, run_id, scope_kind, scope_node_id, fact_family, status, reason)`**
  - `scope_kind` is release, module or callable.
  - Every family an extractor declares gets a row for every module in scope.
  - A module an extractor never reached is `failed` or `unavailable`, never absent.
- **`boundaries(snapshot_id, fact_id, subject_node_id, fact_family, boundary_reason, detail)`**
  - Resolution issues and analysis stops, with one row per stop.
  - Analytics and briefs read these rows instead of treating "the analysis stopped" as "the
    library has nothing more".
- **Two channels, one fact each.** `coverage` and `boundaries` describe the producer's run. Gaps
  that Stage C/D finds are `reason` columns on the derived rows (§3.2). Where both describe one
  fact (a call with no Pysa record), a rule requires them to agree per call site, both ways (§8).
- **What a projection can cite as completeness** (C1, **Implemented**). These state what the graph
  covers and where it stops:
  - `coverage` per family and module;
  - `boundaries`;
  - the unresolved remainders on `resolutions` and `argument_resolutions`;
  - the `external_module` and `external_symbol` nodes: the edge of the analyzed universe, with no
    outgoing call edges because their bodies are not analyzed;
  - `synthetic_callable` nodes: functions with no body in source (a dataclass `__init__`), so a
    call path ends there (279 call edges on the pilot);
  - `graph_gaps`: the raw rows the graph does not represent yet, with the slice that will. Empty
    since C3, and kept as a published table for the next family with such rows (review O2).

  A projection's spec (§5) states which of them it accepts. A non-finding outside that coverage
  is never read as absence (CI-04, CI-08).

> Decision: ADR-0014


### §3.8 Graph catalog and edge registry

**Implemented** for C1 in `cpg_schema::graph` (registry, catalogs, rules) and `cpg_core::udf`, and
**Tested** (C1, 2026-09-22):
- `cpg-core/tests/graph.rs` on the `graph_shapes` fixture: the catalogs as an insta snapshot; an
  isolate; two parallel call sites giving two edge ids; a self-loop; a cross-file SCC and a
  diamond found by a petgraph projection built from the catalogs, with each arc's lineage; an
  unresolved call with no edge; a higher-order `potential` edge; `f(g(x))`'s argument distinct
  from the inner call; a variable export with its reason; identical catalogs across two locations
  and the reversed module order;
- every registry rule rejects an injected violation: lineage and endpoint from raw rows
  (`compile.rs`), and evidence, support, one-per-evidence, no-parallel and typed targets from a
  doctored catalog (`graph.rs`).

C2 and C3 are **Implemented** and **Tested** (`cpg-core/tests/syntax.rs`: the placed tree of
`syntax_shapes`, and every name of `lexical_shapes` with its resolution and bindings; each new rule
rejects an injected violation). C4 is **Implemented** and **Tested** (the same file: every
observation, type variable, class and record field of `type_shapes`, two unrelated `T`s as two
terms with two binders, the recursive alias as one finite term). C5 is **Implemented** and
**Tested** (`a_corpus_documents_its_library`: the selection, passages, a code block inside an MDX
component, links, mentions by class and an other-library member refused, an unparsable
document's coverage, and the usage run: a test module and a materialized block reaching the
release's own declarations, the release class one term. Also: the same corpus in two places is
one context and run, and another library release is another corpus; a tree shadowing the release
fails; an empty glob is refused; each C5 rule rejects an injected violation,
`the_corpus_rules_reject_their_violations`). This
is how
the typed family tables
become a graph without a second authority, following the code-intelligence profile
(`docs/design_review/design_principles/profiles/code-intelligence/principles.md` CI-01–CI-05,
CI-09; formerly the operator's graph guidelines §2–§4, §10–§12).

**The registry** (`cpg_schema::graph`) is the one declaration the catalogs, references and graph
rules are generated from (DP-16).
- **Per node kind:** its **existence source**, a relation independent of every column that
  references the node. Its `node_id` column, `module_node_id` and existence `fact_id` column.
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

**Edge kinds by slice.** Direction reads "source → target".

| Slice | Edge kind: source → target (derivation) |
|---|---|
| C1 | `declares`: module/class/function → class/function (extracted). `overload_of`: stub → callable (joined). `stub_for`: `.pyi` declaration → the `.py` declaration the `exports` seed rank picks, one shared rank (joined). `has_parameter`: function → parameter, ordinal (extracted). `exports`: export → declaration, external symbol, module or external module, one edge per access file, the file as discriminator (analyzer; parallel). `encloses_call`: owner → call site (extracted). `has_argument`: call site → argument, ordinal (extracted). `call_target`: call site → function, synthetic callable or external symbol (analyzer; discriminator `payload_id`; phase, receiver and modality on the evidence row). `higher_order_target`: argument → callable (analyzer; `potential`). `base_class`, `mro_entry`: class → class or external symbol, ordinal, MRO as reported (analyzer). `overrides`: function → function or external symbol (analyzer). `declared_in`: external symbol → external module (joined) |
| C2 (**Implemented**) | `ast_child`: module, declaration, call site or syntax node → the placed child, ordinal (extracted; one parent per node). `argument_value`: argument → its value's placed node (joined; one per argument of a call outside an annotation). `site_target`: the syntax node or call site at a Pysa attribute, artificial or format-string site → function, synthetic callable or external symbol, **not** `potential` records (analyzer; parallel, discriminator `payload_id`; `synthetic_model` for artificial sites) |
| C3 (**Implemented**) | `owns_scope`: module, declaration, lambda or comprehension → scope. `lexical_parent`: scope → enclosing scope. `binds`: scope → binding, ordinal. `introduces`: binding → the declaration, parameter or placed statement that makes it (recognizer). `reads_binding`: reference → binding in its own or the module scope (recognizer; `candidate` when several). `captures`: reference → a binding of an enclosing function scope (recognizer). `reads_builtin`: reference → the builtin function or class (recognizer). `shadows`: binding → the previous event of its name in its scope (joined). `potential_target`: reference (Pysa identifier sites) or syntax node (Pysa `if_called` at attribute sites) → callable (analyzer; `potential`). `imports_module`: module → the module an import names (joined; one per alias). **Deferred:** `imports_symbol` (Pyrefly's `find_definition` at an alias; reopen when a consumer needs symbol-level import targets, since `exports` already trace re-export origins) and separate `global`/`nonlocal` edges (their effect is the declared scope of the binding and the resolution) |
| C4 (**Implemented**) | `has_type`: parameter, function, call site, argument or `raise` syntax node → type term (analyzer; one per subject; role and `declared` on the evidence row). `type_arg`: term → term, ordinal (analyzer; parallel, discriminator the `type_arg_role`, since a callable returning its first parameter's type joins one pair twice at ordinal 0). `type_class`: term → class or external symbol (analyzer, through `type_class_targets`). `has_field`: class → field, ordinal (analyzer). `field_type`: field → term (analyzer) |
| C5 (**Implemented**) | `contains_passage`: document → passage, ordinal; `contains_block`: passage → code block, ordinal (extracted). `mentions`: passage → export, class or function, at the byte offset (recognizer; parallel; `exact` or `lexical` and the modality on the evidence row). `block_module`: Python code block → its materialized module (joined). A usage call's `call_target` reaches the release's own node directly (the corpus names the release's files by `@path`); `usage_link` (code 35) was retired by the C5 review before any snapshot relied on it |

**Rules** generated from the registry (§8):
- Endpoint kinds: both ends exist in `nodes` with an allowed kind, and `src_kind`/`dst_kind`
  equal it.
- **Node-valued references**, from the registry's `node_columns`: every node-valued column of every
  family table names a node of an allowed kind in `nodes`. The hand-kept `REFERENCES` holds only
  fact, provenance and composite references (slice-2 O8, closed).
- Evidence exists, in the kind's evidence table; support exists.
- `key:nodes`, where one id with two kinds is a collision, and `key:edges`.
- **Lineage from raw rows:** each source row yields exactly its declared edges, or its derived row
  carries a provider's reason. Where the edge is read straight off the same unfiltered table,
  the rule is an edit guard (§8), not a data check.
- **Partition of `pysa_calls`:** every row is a call-site row (lineage), an unresolved remainder
  counted on its `resolutions` or `argument_resolutions` row, a C2 site row (lineage through
  `site_targets`), or an identifier site (lineage through `identifier_targets`). No row is a gap
  since C3; the gaps rule stays as an edit guard over an empty table (review O2; §8). The
  remainders rule rejects a resolution that stops counting its remainder (C6 review F1).
- **Placement (C2):** every declaration, and every call outside an annotation, has its
  `syntax_nodes` row; a node is placed once; a placed child lies within its placed parent, in the
  same module; a Pysa site with no node, or a name with no reference, is a `typed:*` violation
  (never a reason).
- **Lexical (C3):** `id:scopes`, `id:bindings`, `id:references`; `typed:reference_resolutions` (no
  binding, builtin or reason); `typed:identifier_targets`; `typed:import_targets` (a reason only
  from Pyrefly's finder: an import we misname fails it, C3 review F2); every reference's name is a
  placed syntax node. Each rejects an injected violation (`compile.rs`, C3 review F7).
- **Earlier rules without a case until C6** (C6 review F1): `typed:exports`,
  `typed:ancestry_targets`, `typed:override_targets`, five `semantic:*`, `id:arguments`,
  `id:context_definitions`, `id:context_modules`, `placed:call_syntax` and "a run's release has
  modules or documents" now reject doctored views in `each_graph_rule_rejects_a_doctored_catalog`
  (`boundary-has-resolution` a raw mutation in `compile.rs`).
- **Source text and role (ADR-0015):** `semantic:source-text` (the text is present exactly when
  the bytes are UTF-8, with `byte_len` bytes); `semantic:source-role-by-run` (the `release` role
  exactly in a run that declares `exports`). Each rejects a doctored view.
- **Docs (C5):** `id:documents`, `id:passages`, `id:code_blocks`; `typed:mention_targets`,
  a run's release has modules or documents, and a document's release has a run (C6 review O6); a
  run declaring a family has what it covers (`coverage:family-has-scope`: a document for `docs`,
  a module for a code family); `coverage:complete` expects a
  `docs` row per document; `unique:release-paths` (an attempt's releases share no path, so a
  `@path` module reference names one file); `unique:type_terms` (one term id, one kind, detail and
  display, however many runs emit it). Each rejects an injected violation
  (`the_corpus_rules_reject_their_violations`, C5 review F7).
- **Docs components (A3):** the CHECKs `span_order`, `parent_before`, `inner_within` and
  `lead_within`; `semantic:doc-component-parent` (a parent exists, precedes its child, contains it
  and is one level up; a top-level component is at depth 0); `semantic:doc-component-in-passage`;
  `semantic:doc-attribute-component` (an attribute's component exists, and it has no name exactly
  when it is a spread and no value exactly when it is bare); and `semantic:docs-span-in-document`
  (every passage, code block, component, link and mention lies within its document's bytes: the
  docs family's missing span rule). Each semantic rule rejects an injected violation in the same
  test.
- **Types (C4):** `id:record_fields`; `typed:type_class_targets`; `typed:type_binders`; lineage
  for every observation, term argument, class-bearing term and record field. The three named
  rules reject injected violations on `type_shapes` (C4 review F1).
- **Typed targets:** a null target carries a reason.
- **Ids:** the Rust recipes equal their SQL form (`id:*`).
- A reference whose target is built from its own source column is not generated, because it
  cannot fail; the lineage rules that cannot are declared edit guards (§8).

**The registry as data.**
- `edge_kinds` publishes each kind's derivation class (`derivation_class`: extracted, analyzer,
  joined, recognizer), direction meaning, parallel policy, evidence table and endpoint kinds with
  every snapshot. A projection selects by them from the store, not from its own build (review F6).
- `graph_gaps` publishes each raw row the graph does not represent yet, with its reason
  (`not_requested`) and the slice that will represent it. After C1 these were Pysa's records at
  attribute, artificial and format-string sites (C2) and at identifiers (C3); since C3 it is
  empty. Nothing is left out silently (review F5).

**Measured** (`lctx compile fastmcp`, release build, FastMCP 4.0.5, this Linux host,
2026-09-22):
- 47,145 nodes and 65,176 edges, with every rule passing;
- every call target (17,174), ancestry entry (1,182) and overridden method (1,107) is a typed node;
- 978 exports have a target, and 335 were variables (`variable_origin`), every one a symbol Pyrefly
  itself calls a variable; after C3, 334 of them target their module-scope binding (one keeps
  `variable_origin`: its origin is a dependency);
- 17,282 Pysa rows were published gaps after C1: 8,083 artificial, 1,502 attribute and 1,942
  format-string sites for C2, and 5,755 identifier sites for C3. **After C2** only the 5,755
  identifier sites remain. Every other site lands on a placed node: 8,500 resolve to targets and
  3,027 are unresolved by Pysa itself (`unresolved_target`). Thirteen chained-comparison sites
  that no node spans attach to their comparison;
- **C2 on the pilot** (2026-09-23, as first committed): 83,627 placed syntax nodes; 113,897 nodes
  and 165,064 edges; every rule passing; 14.9 s, 2.91 GB. After its review every expression
  outside annotations is placed: 138,136 syntax nodes. 829 attribute-site records are `potential`
  and are `potential_target` edges;
- **C3 on the pilot** (2026-09-23): 3,667 scopes, 22,230 bindings, 39,950 references and 45,713
  resolutions. **No name is unresolved**: 45,591 resolutions read a binding (760 rows on 734
  references are closure captures) or a builtin function or class, and 122 read a builtin
  variable. Every Pysa identifier
  site lands on a reference (5,675 resolved, 80 unresolved by Pysa). All 4,526 imports resolve.
  `graph_gaps` is empty. 234,328 nodes and 335,442 edges; every rule passing; 17.5 s at 3.91 GB
  peak RSS, validation 3.4 s. The memory now reaches the C6 streaming trigger's question
  (§4.3). **After C3's compact review** (2026-09-23, snapshot `076151d4…`): 119 of those 122
  "builtin variable" reads were the module's own `__name__` and `__file__`, and now read its
  implicit bindings; 3 read real builtin variables (`NotImplemented`, `Ellipsis`). Still no name
  unresolved, now with the builtins, implicit globals and star sets taken from Pyrefly. Every
  import is found; one export keeps `variable_origin` (its origin is a dependency). The lambda in
  `debug.py`'s default now has the class scope as parent. 25,530 bindings (3,300 implicit),
  244,673 nodes and 391,193 edges; every rule passing; 19.0 s at 3.93 GB;
- **C4 on the pilot** (2026-09-23, fork `6a93da34`): 6,162 type terms and 6,013 term arguments;
  42,263 observations: 6,201 parameters (4,768 declared), 2,449 returns (2,284 declared), 13,871
  call results, 19,008 argument values and 734 raised exceptions. 773 record fields: 447 pydantic,
  258 dataclass and 68 `TypedDict`. All 2,416 class-bearing terms resolve to a node. 102 type
  variables, 90 with a binder; the 12 without are anchored in dependency modules. 126 terms are
  `other` (125 `super()` instances). No `types` boundary, no truncated term. 241,373 nodes and
  387,774 edges; every rule passing; the types pass takes 0.13 s; 18.4 s at 3.81 GB peak RSS
  (validation 4.3 s; a run on the unpushed fork clone peaked at 4.12 GB);
  **After C4's compact review** (2026-09-23, snapshot `69bc2a1e…`, a fresh store: the migration
  changes `type_terms`): 6,228 terms; the 792 annotated `async def` returns now read their
  annotation (0 declared coroutines; 28 unannotated ones keep Pyrefly's computed coroutine); 97
  anchored type variables, 92 bound to a release declaration and 5 anchored in dependencies
  (`scope_boundary`), and 0 variables split across terms (7 were); 17 enum literals carry their
  class; 771 record fields (2 inherited, method-assigned rows gone); 244 `types` boundaries for
  subjects Pyrefly does not type, in 65 modules whose `types` coverage is `partial`. 247,858 nodes
  and 397,961 edges; every rule passing; 20.1 s;
- **C5a on the pilot** (2026-09-23, snapshot `61ade81e…`): the corpus is FastMCP 4.0.5's tree at
  `004bf15a` (594 MDX pages, of which the selection keeps the 148 guide pages: `v2/`, `v3/` and the
  generated `python-sdk/` reference are excluded). 148 documents (144 parse; the 4 `snippets/`
  React components are `unavailable`), 1,608 passages, 1,357 code blocks (960 Python), 5,766
  links, 3,363 mentions: 77 exact and 3,286 lexical candidates, every one with a target. 247,786
  nodes and 397,521 edges; every rule passing; the documents take 0.20 s; 20.7 s at 3.91 GB;
- **C5b on the pilot** (2026-09-23, snapshot `82e00ed6…`): the usage run compiles 1,512 modules
  (125 examples, 427 tests and 960 materialized code blocks, every one parsing); 1,464 library
  definitions the usage code reaches all linked to the release's own (probe P4: the same keys), and
  18,529 usage call edges reached the release through them. 35 usage modules are `types`-partial.
  The whole CPG: 907,845 nodes and 1,452,970 edges; every rule passing; 46.3 s at 6.7–7.7 GB;
- **after the C5 review** (2026-09-23, snapshot `ab6d98a3…`): the corpus names the release's files
  by `@path`, so 18,564 usage call edges and 3,605 usage imports end on the release's own nodes
  with no link table, no class term names a release module by its dotted name, and `FastMCP` is one
  term; the corpus search path is `$release | $venv/lib/python3.14/site-packages`. Mentions are
  unchanged (77 exact, 3,286 lexical). 905,648 nodes and 1,449,162 edges;
- 329 dependency modules: 249 site-packages modules, all with their distribution, and 80 from
  Pyrefly's bundled typeshed;
- 1,913 external symbols;
- `nodes` and `edges` derive in 0.05 s and 0.08 s, and validation takes 1.3 s;
- the whole compile takes 14.4 s wall time at 2.49 GB peak RSS, against 7.9 s and 1.61 GB before
  C1. Of the difference, the Pyrefly check of the referenced dependency modules at
  `Require::Everything` and their definitions take 4.4 s, and validation (now 261 rules) most of
  the rest (review O4);
- `attrs` 26.1.0, which failed `key:edges` before review F2, publishes (4,061 nodes, 5,184
  edges, 1.5 s);
- two runs give one `content_digest`.

**What the catalogs never hold:**
- transitive closures, paths or all-pairs results (CI-08);
- a merged "best" target in place of a candidate set;
- graph-local indices (§5).

> Decision: ADR-0014
