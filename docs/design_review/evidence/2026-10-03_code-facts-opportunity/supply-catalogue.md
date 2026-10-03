# Supply catalogue: provider code facts library-context does not consume (baseline pyrefly 1.4.0-dev.3 / ruff 0.0.14 / ty 0.0.14)

Prepared 2026-10-03 by a library-research worker for a design/target review. Supply only: no adoption is recommended.

## Baseline and method

- **Source of facts:** the shared skill `python-analyzers` code-facts model (`authoring/facts/facts.json`, 340 records; read-only). Every record whose id is not in `authoring/project/facts-overlay.json` `facts_used` is listed here: **299 of 340** (41 consumed). The overlay is approximate; the sibling code-mapper owns current use.
- **Target baseline:** pyrefly 1.4.0-dev.3 (commit `80cec3f5`, the skill's own pin, so every pyrefly record is present); ruff library crates 0.0.14 (= ruff tag 0.16.8; `ruff_linter` is published as 0.16.8); ty 0.0.14 (ty crates at the ruff 0.16.8 tag, `62914c4b`).
- **ty 0.0.14 / ruff 0.0.14 verification (2026-10-03):** sparse checkouts of astral-sh/ruff tags `0.16.8` (`62914c4b`) and `0.16.10` (`3265ed1f`, the skill's pin). `ty_python_core` at the 0.16.8 tag is byte-identical to crates.io `ty_python_core` 0.0.14. For each unconsumed ruff and ty record, every item name in its Rust surfaces was searched as a word in the owning crate's `src` at both tags, and the visibility of each matching `fn`/`struct`/`enum`/`mod` definition and `pub use` re-export was compared. Only the differences recorded under "Baseline" below were found. This verifies **item presence and visibility**, not behaviour: exactness, limits and counts in the records were observed at 0.0.16 and transfer to 0.0.14 is unverified unless stated. `ty_python_semantic` is published at 0.0.14 (crates.io, checked 2026-10-03); `ty_ide`, `ty_project` and `ty_server` are not published (crates.io returns "does not exist").
- **Claim class** defaults to the record's `fidelity` (syntactic -> exact syntactic, resolved, inferred -> type-level inferred, heuristic, diagnostic-only). "Evaluated verdict" is not a skill fidelity; this catalogue assigns it to 7 records and re-classes 5 as diagnostic-only; each override carries its reason in the entry (see also "Re-classed records").
- **Route** names the surface a consumer would reach it through at the baseline. For pyrefly: "no further patch" means public or doc-hidden-public items; "existing fork patch" means items under `pyrefly::report`, `pyrefly::export` or `commands::coverage::collect`, which the current patch's changes 3 and 5 already open (fork-patch brief); "patch extension" means a further visibility hunk. CLI-only pyrefly diagnostics are in-process through the error collection library-context already reads for ParseError.
- **Attachment** names the basis. At this baseline pyrefly and ty both parse with ruff 0.0.14, so a ty byte range, a ruff re-lex range and pyrefly's `TextRange` coincide for the same bytes. Node identity still differs between parses. ruff AST helpers can take pyrefly's retained AST directly, because they share the crate version.

## Attachment codes

The brief's four bases, refined by the route that reaches the fact:

| Code | Basis | Meaning |
|---|---|---|
| `PR-range` | pyrefly parse of record, `TextRange` | In-process pyrefly answer, binding key, collected error or Pysa/Glean collector, located by its `TextRange` on the retained parse. |
| `PR-key` | pyrefly parse of record, `TextRange` | Indexed by pyrefly `Idx<Key>`; every key carries a `TextRange`, so it attaches by range. |
| `PR-pos` | pyrefly parse of record, `TextRange` | Answered per queried position (LSP-style `Transaction` methods); byte offsets in-process, UTF-16 only at the LSP. |
| `PR-ast` | pyrefly parse of record, `TextRange` and node identity | A ruff 0.0.14 free function applied to pyrefly's retained 0.0.14 AST (same crate version at this baseline), so the result shares both range and node identity with the parse of record. |
| `PR-report` | pyrefly parse of record, via report positions | Only through a separate `pyrefly` CLI run over the same bytes (Glean byte spans, Pysa 1-based UTF-8 columns); no node identity. |
| `RF-lex` | second ruff 0.0.14 lex, byte range | Needs ruff's token stream. pyrefly keeps one, but `Transaction::get_parsed_module` is `pub(crate)`, so the bytes are re-lexed. The lexer line is the same, so ranges equal the parse of record's. |
| `RF-checker` | second ruff 0.0.14 parse, byte range | Lives in a populated `ruff_python_semantic::SemanticModel`, which only `ruff_linter`'s `Checker` builds (no public driver), over ruff_linter's own parse. Ranges are equal; node identity is not shared. |
| `RF-reparse` | second ruff 0.0.14 parse, byte range | A `ruff_python_parser` result recomputed on a second parse. |
| `RF-report` | report position | Only as findings (CLI JSON, 1-based character columns, or `ruff_linter` diagnostics); convert through a line index. |
| `TY-range` | ty second parse, byte range | Located by byte range on ty's own parse (`ruff_db::parsed_module`). At this baseline both parses use ruff_python_parser 0.0.14, so ranges agree byte for byte. The flow stage's TYPE_CHECKING rename keeps lengths, so ranges are unaffected. |
| `TY-pos` | ty second parse, byte range | Answered per queried offset (`ty_ide` / `SemanticModel` queries); results carry byte ranges. |
| `TY-id` | node identity (ty) | Keyed by ty semantic-index identity (`Definition`, `ScopeId`, `ScopedPlaceId`, predicate or loop-header ids). These are salsa-interned and run-local, and they attach only after mapping each id to its node's range. |
| `TY-report` | report position | Only through `ty check` output formats. |
| `ID` | node identity | The fact is itself a run-local identity (Pysa ids, `NodeIndex`, `ProgramFile`). |
| `TEXT` | byte offsets over the source text | No parse is involved: trivia tokenizer, line index. |
| `LEVEL` | module, file, project, name, rule or type level | No source range is attached, or the range is incidental. |

Baseline wording: "present at 0.0.14 (named items verified in source)" means every item named in the record's Rust surfaces exists, with the same visibility, at the 0.16.8 tag. pyrefly records are at the indexed pin itself.

## Counts

By provider: pyrefly 102, ruff 92, ty 105.

| Claim class | Facts |
|---|---:|
| exact syntactic | 90 |
| resolved | 96 |
| type-level inferred | 71 |
| evaluated verdict | 7 |
| heuristic | 12 |
| diagnostic-only | 23 |

| Attachment basis | Facts |
|---|---:|
| LEVEL | 71 |
| PR-range | 70 |
| TY-range | 47 |
| RF-checker | 22 |
| PR-ast | 17 |
| TY-pos | 17 |
| RF-report | 15 |
| TY-id | 12 |
| RF-lex | 9 |
| PR-pos | 6 |
| ID | 3 |
| TEXT | 3 |
| PR-report | 2 |
| PR-key | 2 |
| RF-reparse | 2 |
| TY-report | 1 |

**Concepts skipped (every provider fact consumed):** `calls.call-graph`, `conditions.predicates`, `exports.public-names`, `imports.resolution`, `scopes.places`.

**Concepts with no facts of their own** (blank, or covered only through `also_concepts`): `concurrency.threads`, `dataflow.effect-kinds`, `dataflow.interprocedural`, `dataflow.intraprocedural`, `dataflow.points-to`, `dataflow.taint`, `exceptions.propagation`, `native.extensions`, `project.unreferenced-definitions`, `runtime.execution-evidence`; see "Blank and partial concepts".

## Facts by concept

Each entry: **id** (provider; unit) - what it states. *Exact:* the record's exactness. Then claim class, attachment, baseline presence and route, and overlaps with consumed facts.

### callables

#### `callables.decorators`

- **`pyrefly.decorators`** (pyrefly; unit: function) - The decorators applied to each function or class, their resolved callees, special decorators recognised by pyrefly, and the undecorated versus decorated function type. *Exact:* Callees resolved by type.
  - Limits: Glean decorators are source text only.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch; parts behind private modules: covered by the existing patch (changes 3/5).
  - Overlaps: derived from consumed `pyrefly.expression-types`; equivalent (unconsumed) `ty.decorated-types`, `ruff.decorator-predicates`
- **`ruff.decorator-predicates`** (ruff; unit: function) - Whether a function is an overload, override, property, abstract, final, static/class method, test, magic, init, new, validator (pydantic/attrs), and its method visibility. *Exact:* By qualified name of decorators; `@override` only, no override checking.
  - Limits: No decorated vs undecorated signature; overloads are not grouped or chosen.
  - Claim: **resolved**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic`.
  - Overlaps: none recorded
- **`ty.decorated-types`** (ty; unit: callable) - The type a decorated `def` binds after applying its decorators, versus the undecorated function literal; decorators whose return type is dynamic are flagged. *Exact:* Exact for annotated decorators.
  - Limits: No public list of decorator expressions with their types beyond inferring each expression.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core); public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: derived from consumed `ty.definitions`

#### `callables.flags`

- **`ruff.function-kind`** (ruff; unit: function) - The kind of a function by its scope and decorators (configurable classmethod/staticmethod decorators), whether it is a stub, and whether it is subject to the Liskov principle. *Exact:* Decorators matched by qualified name.
  - Claim: **resolved**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic`.
  - Overlaps: overlaps consumed `pyrefly.function-flags`
- **`ty.function-flags`** (ty; unit: callable) - Decorator-derived flags (classmethod, staticmethod, overload, abstractmethod, final, override, no_type_check, type_check_only), property accessors, known stdlib functions, and the method kind a decorator implies. *Exact:* Decorators are recognised by their resolved type, not by spelling.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: overlaps consumed `pyrefly.function-flags`

#### `callables.generators`

- **`pyrefly.yields`** (pyrefly; unit: expression) - For each yield and yield-from, the yielded type and the value sent/returned, combined into a generator or coroutine return type; invalid-yield and not-async diagnostics. *Exact:* Per yield expression.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: equivalent (unconsumed) `ty.generator-kinds`
- **`ty.generator-kinds`** (ty; unit: callable) - Whether a function is a generator or coroutine through its inferred type (Generator/CoroutineType), yield and await validity, and coroutines whose result is discarded. *Exact:* Annotated functions only; unannotated returns are Unknown.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: none recorded

#### `callables.overloads`

- **`pyrefly.overloads`** (pyrefly; unit: call) - Overload definitions per function, the overload a call chooses (and all candidates), and overload consistency diagnostics. *Exact:* Exact range lookup in the answers trace.
  - Limits: Pysa call targets on Type::Overloaded receivers are not handled.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: derived from consumed `pyrefly.expression-types`; equivalent (unconsumed) `ty.overloads`
- **`ty.overloads`** (ty; unit: call) - The overloads and implementation of a function, which overload a call selects (the call type simplified by overload evaluation), and invalid overload sets. *Exact:* Follows the typing spec's overload evaluation (argument-type expansion, Any filtering).
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: none recorded

#### `callables.signatures`

- **`ruff.parameters`** (ruff; unit: function) - A function's positional-only, regular, *args, keyword-only and **kwargs parameters with defaults and annotations, and the return annotation, as AST nodes. *Exact:* Exact as written.
  - Limits: No decorator effects, no inherited or inferred signatures.
  - Claim: **exact syntactic**. Attach: PR-ast.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`.
  - Overlaps: overlaps consumed `pyrefly.function-signatures`; derived from consumed `ruff.ast`
- **`ty.signatures`** (ty; unit: callable) - Parameters with kinds (positional-only, positional-or-keyword, variadic, keyword-only, keyword-variadic), defaults, annotated types and return types of a callable, including synthesized ones (dataclass `__init__`, NamedTuple, constructors). *Exact:* Annotated types after specialization.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: overlaps consumed `pyrefly.function-signatures`

### calls

#### `calls.argument-binding`

- **`pyrefly.argument-names`** (pyrefly; unit: argument) - For each positional argument of a call, the parameter name it binds to, shown as inlay hints; Glean records argument spans and keyword names. *Exact:* Matches arguments to the chosen signature.
  - Limits: No public argument-to-parameter map as data; only hints.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.function-signatures`; equivalent (unconsumed) `ty.argument-binding`, `ruff.argument-lookup`; difference: ty has a public argument-to-parameter map; pyrefly only inlay names.
- **`ruff.argument-lookup`** (ruff; unit: call) - The argument expression passed for a parameter, given the parameter's name and position supplied by the caller, and whether *args/**kwargs could hide it. *Exact:* Syntactic; the caller supplies the signature knowledge.
  - Limits: No signature resolution.
  - Claim: **exact syntactic**. Attach: PR-ast.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`.
  - Overlaps: derived from consumed `ruff.ast`
- **`ty.argument-binding`** (ty; unit: call) - For a call, each candidate signature (one per overload or union member) with its label and displayed parameters, and for each argument the parameter(s) it matched (several for a splat) and whether matching succeeded; keyword arguments map to the parameter definition. *Exact:* Bindings are kept even when the call is invalid.
  - Limits: On demand, one call at a time.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: none recorded
- **`ty.call-argument-forms`** (ty; unit: call) - For each argument of a call, whether the callee interprets it as a value expression or a type expression (for example the first argument of `cast`, `TypeVar` bounds), or unknown. *Exact:* From full binding of the call.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: none recorded

#### `calls.call-hierarchy`

- **`pyrefly.call-hierarchy`** (pyrefly; unit: function) - For a function at a position, the functions that call it and the functions it calls, with call ranges. *Exact:* Built on find-references and goto-definition.
  - Limits: One item per request; indexed files only.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: equivalent (unconsumed) `ty.call-hierarchy`
- **`ty.call-hierarchy`** (ty; unit: callable) - For a function or class at a position, the call sites that call it across the project (incoming) and the callees in its body, decorators, annotations, defaults and bases (outgoing, nested bodies excluded). *Exact:* Incoming calls scan project files with an identifier text prefilter.
  - Limits: No whole-program export.
  - Claim: **type-level inferred**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_ide`. Route: public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded

#### `calls.call-sites`

- **`ruff.call-sites`** (ruff; unit: call) - Each call expression with its callee expression and arguments; helpers strip a call or subscript to its callee (`map_callable`, `map_subscript`). *Exact:* Exact as written; decorators, operators and implicit dunder calls are not call sites.
  - Claim: **exact syntactic**. Attach: PR-ast.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`.
  - Overlaps: overlaps consumed `pyrefly.call-sites`; derived from consumed `ruff.ast`

#### `calls.call-targets`

- **`ty.call-targets`** (ty; unit: call) - The inferred type of a call's callee (function, bound method with its receiver, class constructor, callable instance, union of these) and the definition each candidate signature comes from; non-callable callees are diagnostics. *Exact:* Per call site on demand.
  - Limits: No whole-program call-graph export.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: overlaps consumed `pyrefly.call-targets`

#### `calls.dispatch`

- **`pyrefly.singledispatch`** (pyrefly; unit: function) - For a `@singledispatch` function, its registered implementations and their dispatch types, checked for consistency (`bad-singledispatch-register`). *Exact:* Solver support for a behaviour typeshed cannot express.
  - Limits: Registrations are not exposed as data.
  - Claim: **diagnostic-only**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process through the collected errors already read (filter by ErrorKind; preset-dependent); CLI JSON otherwise.
  - Overlaps: none recorded

#### `calls.higher-order`

- **`pyrefly.higher-order-parameters`** (pyrefly; unit: argument) - At a call, which arguments are themselves callables (by argument index) and their resolved targets. *Exact:* By type of the argument expression.
  - Limits: When or whether the callee calls them is not analysed.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process through the existing fork patch (changes 3/5 open `report`/`export`/`coverage::collect`).
  - Overlaps: derived from consumed `pyrefly.call-targets`

#### `calls.implicit-calls`

- **`ty.implicit-calls`** (ty; unit: expression) - The dunder definitions a binary or unary operator resolves to, and the implicit `__iter__`/`__next__`, `__enter__`/`__exit__`, `__getitem__`, `__bool__`, `__call__` and descriptor calls ty checks. *Exact:* Follows Python's reflected-operator and descriptor rules.
  - Limits: Only operators have a public definition query.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: overlaps consumed `pyrefly.implicit-calls`

### classes

#### `classes.fields`

- **`ruff.class-members`** (ruff; unit: class) - Whether a class body declares a member satisfying a predicate (assignment, annotated assignment or method) and whether it is bound. *Exact:* Direct body only (plus same-module bases where the caller walks them).
  - Limits: No inherited members, descriptors or field flags.
  - Claim: **exact syntactic**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic`.
  - Overlaps: overlaps consumed `pyrefly.class-fields`; derived from consumed `ruff.ast`
- **`ty.class-fields`** (ty; unit: field) - Fields of dataclasses, NamedTuples, TypedDicts (with Required/NotRequired/ReadOnly) and pydantic models, with init/kw_only/default/alias flags; for a TypedDict subscript key, the owning class, declared type and docstring. *Exact:* As synthesized for `__init__`.
  - Limits: Only the TypedDict key view is public.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: overlaps consumed `pyrefly.class-fields`

#### `classes.hierarchy`

- **`pyrefly.type-hierarchy`** (pyrefly; unit: class) - For a class at a position, its supertypes and the classes that subclass it in indexed files. *Exact:* Subtypes limited to indexed files.
  - Limits: One class per request.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.class-mro`; equivalent (unconsumed) `ty.type-hierarchy`
- **`ty.type-hierarchy`** (ty; unit: class) - For a class, its direct supertypes and its direct subtypes across the project, as items with name, file and ranges. *Exact:* Subtypes are found by scanning project files.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source); also served by unpublished `ty_ide`. Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded

#### `classes.members`

- **`pyrefly.class-members`** (pyrefly; unit: member) - The attributes available on a class or instance type (own and inherited, descriptors, properties, class methods) with their types; completion and hover use it; missing-attribute diagnostics when absent. *Exact:* Attribute lookup follows the MRO and descriptor protocol.
  - Limits: No exported member table per class; per-query only.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: derived from consumed `pyrefly.class-fields`, `pyrefly.class-mro`; equivalent (unconsumed) `ty.members`
- **`ty.implicit-instance-attributes`** (ty; unit: class) - Attributes created by `self.x = ...` (and declared `self.x: T`) in a class's methods, with the assignments or declarations that define them. *Exact:* Index-level discovery; types come from inference.
  - Claim: **resolved**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core); richer parts are internal.
  - Overlaps: derived from consumed `ty.places`
- **`ty.members`** (ty; unit: type) - The attributes available on a type (own, inherited, metaclass, module members), each with its type and defining definition; the static (descriptor-object) type of an attribute without invoking the descriptor protocol. *Exact:* Follows descriptor and MRO lookup.
  - Claim: **type-level inferred**. Attach: LEVEL (per queried type; member definitions carry ranges).
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: none recorded

#### `classes.metadata`

- **`pyrefly.abstract-members`** (pyrefly; unit: class) - For each class, which abstract methods remain unimplemented (making it abstract), with diagnostics on instantiation or implicit abstract classes. *Exact:* Computed per class.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: derived from consumed `pyrefly.class-metadata`
- **`ruff.class-kinds`** (ruff; unit: class) - Whether a class is an enumeration, a metaclass (Yes/No/Maybe), might be generic, and (inside rules) a dataclass, Protocol, TypedDict or NamedTuple by its bases and decorators. *Exact:* By qualified names of bases/decorators.
  - Limits: Dataclass detection lives in private ruff_linter helpers; no field semantics.
  - Claim: **resolved**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic`.
  - Overlaps: overlaps consumed `pyrefly.class-metadata`
- **`ty.class-metadata`** (ty; unit: class) - Whether a class is a dataclass (with its parameters) or dataclass_transform-driven, an enum (members and values), a Protocol, a TypedDict, a NamedTuple, a NewType, final, abstract (unimplemented abstract methods), a pydantic model, has `__slots__`, its metaclass and disjoint-base status. *Exact:* Follows the typing spec for each construct.
  - Limits: Almost all internal; public route is hover and diagnostics.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: overlaps consumed `pyrefly.class-metadata`

#### `classes.mro`

- **`ruff.class-bases`** (ruff; unit: class) - Whether any base or super class of a class satisfies a predicate, following class bindings in the same module, and the qualified names of explicit bases. *Exact:* Syntactic base walk.
  - Limits: No MRO; bases defined in other modules are opaque qualified names.
  - Claim: **resolved**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic`.
  - Overlaps: overlaps consumed `pyrefly.class-mro`
- **`ty.mro`** (ty; unit: class) - A class's explicit bases and C3 method resolution order of class bases (including dynamic `type(...)` classes), or an error: cyclic definition, duplicate bases, inconsistent MRO, invalid or unsupported bases. *Exact:* On error ty falls back to an MRO of `Unknown` and `object` rather than a recovery prefix.
  - Limits: No public full-MRO query.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: overlaps consumed `pyrefly.class-mro`

#### `classes.overrides`

- **`ty.overrides`** (ty; unit: definition) - Whether a method or attribute override is compatible with the overridden member (Liskov), final members are overridden, `@override` is used without a base member or missing where required. *Exact:* Type-based; no public override graph.
  - Claim: **diagnostic-only**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: internal only (`pub(crate)`): no published route without a ty fork; CLI/LSP route only.
  - Overlaps: overlaps consumed `pyrefly.overrides`

### concurrency

#### `concurrency.async-flow`

- **`ruff.async-rules`** (ruff; unit: expression) - Blocking calls in async functions, missing awaits on trio/anyio, dangling asyncio tasks (RUF006), await/async comprehensions outside async contexts (via semantic syntax errors), and whether a node is in an async context. *Exact:* Pattern-based.
  - Limits: No await graph or task ordering.
  - Claim: **diagnostic-only**. Attach: RF-report.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic` (also CLI).
  - Overlaps: none recorded

#### `concurrency.resources`

- **`pyrefly.context-managers`** (pyrefly; unit: statement) - Whether a `with` target is a valid (async) context manager (`bad-context-manager`) and whether its `__exit__` may suppress exceptions, which decides reachability after the block. *Exact:* Type-based.
  - Limits: No resource lifetime tracking. Partial for this concept: protocol checks and suppression; no resource lifetimes.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: derived from consumed `pyrefly.expression-types`; equivalent (unconsumed) `ty.context-managers`
- **`ruff.resource-rules`** (ruff; unit: expression) - Files opened without a context manager (SIM115), nested with statements (SIM117), pathlib replacements for open/os calls (PTH), and unclosed-style patterns flagged by rules. *Exact:* Syntactic patterns; no lifetime tracking.
  - Claim: **diagnostic-only**. Attach: RF-report.
  - Baseline: present at 0.0.14 (named items verified in source). Route: CLI only (`ruff` 0.16.8 = crates 0.0.14 line), or `ruff_linter` 0.16.8 in-process.
  - Overlaps: none recorded
- **`ty.context-managers`** (ty; unit: statement) - The `__enter__`/`__aenter__` result bound by `with`, and whether `__exit__` may suppress exceptions (its return type), which decides whether code after the `with` is reachable. *Exact:* From `__exit__` return type (`bool` may suppress, `None` does not).
  - Limits: No pairing of opened and closed resources. Partial for this concept: protocol checks and suppression; no pairing of opened and closed resources.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core); richer parts are internal.
  - Overlaps: derived from consumed `ty.predicates`

### conditions

#### `conditions.match-exhaustiveness`

- **`pyrefly.match-exhaustiveness`** (pyrefly; unit: statement) - For a `match` over a closed type (enum, union of literals, sealed classes, bool), the cases no pattern covers, reported as `non-exhaustive-match`; `non-exhaustive-match-open-type` (off by default) does the same for open types. *Exact:* Exact for closed types; the open-type variant is a sub-kind, suppressed with its parent.
  - Limits: Missing cases appear only in the message text.
  - Claim: **evaluated verdict** (exhaustiveness verdict, carried only as a diagnostic). Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: needs a fork-patch extension (`pub(crate)`/private item); otherwise CLI/LSP only.
  - Overlaps: none recorded
- **`ty.match-exhaustiveness`** (ty; unit: statement) - Whether the patterns of a `match` exhaust the subject type, used to decide that the fall-through is unreachable and that an implicit `None` return cannot happen. *Exact:* Per pattern against the subject's inferred type.
  - Limits: No lint and no public query; observable only through reachability and return-type checks.
  - Claim: **evaluated verdict** (exhaustiveness verdict, internal; observable via reachability). Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: internal only (`pub(crate)`): no published route without a ty fork; no route.
  - Overlaps: none recorded

#### `conditions.narrowed-types`

- **`pyrefly.narrowed-types`** (pyrefly; unit: use) - At a use, the narrowed type of a name; the cinderx report records both the narrowed and the unnarrowed type and flags mismatches. *Exact:* Exact for pyrefly's narrowing rules.
  - Limits: --report-cinderx is hidden and internal-only.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.expression-types`; equivalent (unconsumed) `ty.narrowed-types`
- **`ty.narrowed-types`** (ty; unit: use) - The type of a name or member at a use after applying the tests that guard it: isinstance/issubclass, `is`/`==`/`in` (including containment), `len`, `callable`, `hasattr`, TypeGuard/TypeIs, truthiness, match patterns. *Exact:* As precise as ty's type algebra; `[analysis] strict-generic-narrowing` changes generic narrowing.
  - Limits: No table of narrowing facts; only the result type at each use.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: derived from consumed `ty.narrowing-constraints`

#### `conditions.narrowing-ops`

- **`pyrefly.narrowing-ops`** (pyrefly; unit: test) - Which test narrows which name or member chain (facets), and how: is/is not, ==, isinstance/issubclass, hasattr/getattr, TypeGuard/TypeIs, in, len comparisons, sequence/mapping patterns, truthiness, user calls; composed with and/or. *Exact:* 40 atomic ops at this pin, including negations and placeholders.
  - Limits: Carried inside Binding::Narrow; an internal representation.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: overlaps consumed `ty.narrowing-constraints`

#### `conditions.truthiness`

- **`pyrefly.redundant-condition`** (pyrefly; unit: expression) - Tests whose type makes them always true or false (`redundant-condition`), comparisons that are always unequal (`unnecessary-comparison`, `incompatible-comparison`), and objects used as bools without a meaningful __bool__ (`implicit-bool`). *Exact:* Type-based.
  - Limits: No truthiness verdict as data.
  - Claim: **evaluated verdict** (always-true/false test verdicts, carried only as diagnostics). Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process through the collected errors already read (filter by ErrorKind; preset-dependent); CLI JSON otherwise.
  - Overlaps: derived from consumed `pyrefly.expression-types`; equivalent (unconsumed) `ruff.truthiness`, `ty.truthiness`; difference: pyrefly states truthiness only as diagnostics.
- **`ruff.truthiness`** (ruff; unit: expression) - Whether an expression is a literal known to be True/False, truthy/falsey, None or unknown. *Exact:* Literals and builtin constructors (resolved through a caller callback) only.
  - Limits: Partial for this concept: literal and builtin-constructor truthiness only.
  - Claim: **exact syntactic**. Attach: PR-ast.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`.
  - Overlaps: derived from consumed `ruff.ast`
- **`ty.truthiness`** (ty; unit: expression) - Whether an expression's value is always truthy, always falsy or ambiguous, from its type; drives reachability and lints redundant conditions and suspicious truthiness tests. *Exact:* Type-based; `AlwaysTruthy`/`AlwaysFalsy` are types in ty's algebra.
  - Limits: No public per-expression query. Partial for this concept: no public per-expression query; visible through reachability and lints.
  - Claim: **evaluated verdict** (always-truthy/falsy/ambiguous verdict from the type). Attach: TY-range.
  - Baseline: present at 0.0.14; the three `truthiness-test-of-*` lints that make it visible as diagnostics exist only at 0.0.16. Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core); richer parts are internal.
  - Overlaps: none recorded

### control

#### `control.branch-nesting`

- **`ruff.branch-structure`** (ruff; unit: node) - Which branch (if/elif/else, try/except, match case, loop body) a node lies in, whether two nodes are on the same branch, and whether one node's branch dominates another's in the branch tree. *Exact:* Tree containment over branch ids, not CFG dominance.
  - Limits: Linter-internal. No tests attached to branches and no path conditions.
  - Claim: **exact syntactic**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic`.
  - Overlaps: derived from consumed `ruff.ast`

#### `control.cfg`

- **`ruff.cfg-stub`** (ruff; unit: block) - Basic blocks of a statement list split only at return and raise; compound statements (if, while, for, match, try, with) are no-ops; edges carry only Condition::Always. *Exact:* Incomplete by design at this pin.
  - Limits: Its only user, PLW0101 unreachable-code, is compiled only with the `test-rules` feature: it is absent from the shipped CLI (`ruff rule PLW0101` is an error at 0.16.10, and was test-only at 0.16.8 too). Partial for this concept: blocks split only at return and raise; compound statements are no-ops; ...
  - Claim: **exact syntactic**. Attach: PR-ast.
  - Baseline: present at 0.0.14 (same stub; PLW0101 test-only at 0.16.8). Route: crates `ruff_python_semantic`.
  - Overlaps: derived from consumed `ruff.ast`

#### `control.loops`

- **`pyrefly.loop-phis`** (pyrefly; unit: loop) - At a loop header, the binding merge of the pre-loop and back-edge definitions of each name (LoopPhi). *Exact:* Internal binding variant.
  - Limits: No loop or back-edge object; only the merge key. Partial for this concept: a merge key per loop header; no loop or back-edge object.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: equivalent (unconsumed) `ty.loop-headers`
- **`ty.loop-headers`** (ty; unit: statement) - Synthetic LoopHeader definitions at each loop head stand for bindings arriving over the back edge; their member bindings can be listed per place. *Exact:* Exact for the index.
  - Limits: No loop graph object; headers appear as definitions in use-def results. Partial for this concept: header definitions only; no back-edge graph.
  - Claim: **resolved**. Attach: TY-id (synthetic definitions carry the loop statement range).
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core).
  - Overlaps: derived from consumed `ty.use-def`

#### `control.reachability`

- **`pyrefly.unreachable-diagnostics`** (pyrefly; unit: suite) - Statements after a definite exit or in environment-independent dead suites (`unreachable`), except clauses that can never run (`unreachable-except-clause`), match cases that cannot match (`unreachable-match-case`), plus LSP `unreachable-code` hints per statement for configuration-dead branches. *Exact:* Type-aware at solve time (Never-returning calls, never-suppressing context managers, type-decided tests).
  - Limits: KeyExpect answers carry no verdict; reachability exists only as diagnostics. Code after a definite exit is still bound and typed.
  - Claim: **evaluated verdict** (suite-level dead-code verdicts, carried only as diagnostics). Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: overlaps consumed `ty.reachability-constraints`; derived from consumed `pyrefly.static-branches`, `pyrefly.expression-types`; difference: pyrefly gives suite-level diagnostics; ty gives per-range diagrams.
- **`ty.reachability-evaluation`** (ty; unit: range) - Ranges ty decides cannot execute, split into Unconditional (ALWAYS_FALSE in the index) and CurrentAnalysis (unreachable only under the configured version, platform or inferred types, for example after a `NoReturn` call). *Exact:* Type- and configuration-dependent; ambiguous ranges are treated as reachable.
  - Limits: Not a CLI diagnostic; no unreachable-code lint at this pin.
  - Claim: **evaluated verdict** (evaluated per range (Unconditional vs CurrentAnalysis)). Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source); also served by unpublished `ty_ide`. Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag); richer parts are internal.
  - Overlaps: derived from consumed `ty.reachability-constraints`

#### `control.static-branches`

- **`ruff.static-branch-classification`** (ruff; unit: statement) - Whether an if statement is a TYPE_CHECKING block or a sys.version_info comparison, and, through rules, whether a version block is outdated for the target version (UP036) or a version comparison is suspect (YTT). *Exact:* Classification only; branches are never pruned from the semantic model.
  - Limits: No sys.platform evaluation.
  - Claim: **resolved**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic` (also CLI).
  - Overlaps: overlaps consumed `pyrefly.static-branches`

#### `control.terminal`

- **`pyrefly.return-analysis`** (pyrefly; unit: function) - For each function, the type of falling off the end (`ReturnImplicit`, Never when the end is unreachable), each explicit return's type, and the combined inferred return type; missing-return and bad-return diagnostics. *Exact:* Never for ReturnImplicit means the end of the body is unreachable.
  - Limits: Per function, not per path; which paths raise is not recorded. Inference of returns depends on `infer-return-types` (never/annotated/checked).
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: derived from consumed `pyrefly.expression-types`; equivalent (unconsumed) `ruff.terminal`, `ty.terminal-calls`
- **`ruff.terminal`** (ruff; unit: function) - Whether a function body never exits explicitly, implicitly returns, always raises, raises NotImplementedError, always returns, raises or returns, or conditionally returns. *Exact:* Syntactic walk; calls to NoReturn functions are not known.
  - Claim: **exact syntactic**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic`.
  - Overlaps: derived from consumed `ruff.ast`
- **`ty.terminal-calls`** (ty; unit: statement) - Every call statement records an IsNonTerminalCall predicate that the semantic layer resolves to false when the callee returns `Never`/`NoReturn`; `finally` suites whose normal entry is impossible and context managers that may suppress exceptions are separate predicates. *Exact:* A call returning Unknown/Any is treated as non-terminal (never ambiguous).
  - Limits: Unannotated functions' return types are not inferred at this pin (`def h(): raise ValueError` reveals `-> Unknown`), so their calls never terminate flow.
  - Claim: **evaluated verdict** (IsNonTerminalCall resolved true/false by the semantic layer). Attach: TY-id (predicate per call statement).
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core).
  - Overlaps: derived from consumed `ty.predicates`

### dataflow

#### `dataflow.constants`

- **`pyrefly.division-by-zero`** (pyrefly; unit: expression) - Divisions whose divisor is known to be zero from Literal types (`division-by-zero`). *Exact:* Literal types only.
  - Limits: Values beyond Literal types are not tracked.
  - Claim: **diagnostic-only**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process through the collected errors already read (filter by ErrorKind; preset-dependent); CLI JSON otherwise.
  - Overlaps: derived from consumed `pyrefly.expression-types`
- **`pyrefly.literal-values`** (pyrefly; unit: expression) - Where an expression's inferred type is a Literal (int, str, bytes, bool, enum member), the exact value it holds, with the promoted class type; cinderx records the literal value and its promoted type index. *Exact:* Exact where pyrefly keeps the literal; values widen at joins and declared annotations.
  - Limits: No ranges, string values or abstract interpretation beyond Literal types. Partial for this concept: exact values only where a Literal type carries them.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: derived from consumed `pyrefly.expression-types`; equivalent (unconsumed) `ty.literal-values`
- **`ty.literal-values`** (ty; unit: expression) - Values an expression is known to hold through literal types: int, str, bytes, bool and enum-member literals, None and fixed-length tuples of literals, plus `index-out-of-bounds`, `division-by-zero` and `zero-stepsize-in-slice` checks on them. *Exact:* Exact while literal; promoted to the nominal type on widening.
  - Limits: No ranges or abstract values beyond literals. Partial for this concept: exact values only where a Literal type carries them.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: none recorded

#### `dataflow.side-effects`

- **`pyrefly.unused-results`** (pyrefly; unit: statement) - Expression statements whose call result is discarded where that is likely a mistake (`unused-call-result`) and coroutines never awaited (`unused-coroutine`). *Exact:* Type-based lints, not an effect analysis.
  - Limits: unused-call-result is off by default.
  - Claim: **diagnostic-only**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process through the collected errors already read (filter by ErrorKind; preset-dependent); CLI JSON otherwise.
  - Overlaps: derived from consumed `pyrefly.expression-types`
- **`ruff.side-effects`** (ruff; unit: expression) - Whether an expression contains an effect (calls other than known-pure builtins, await, yield, assignments...) and a three-valued SideEffect for removal decisions. *Exact:* Syntactic; builtins identified through a callback.
  - Limits: No purity analysis of user functions; any unknown call is an effect.
  - Claim: **heuristic**. Attach: PR-ast.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`.
  - Overlaps: derived from consumed `ruff.ast`

### diagnostics

#### `diagnostics.baselines`

- **`pyrefly.baseline`** (pyrefly; unit: diagnostic) - A stored set of errors (full or minimal format) and, for a run, which current errors match it (by column or concise description), which baseline entries are stale, and updated/pruned baselines. *Exact:* Matching mode decides how stable entries are across edits.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.type-errors`

#### `diagnostics.fixes`

- **`pyrefly.quick-fixes`** (pyrefly; unit: edit) - Edits that fix a finding at a position (including auto-import of an unknown name, redundant cast removal across a file, pytest fixture annotation), as raw text edits. *Exact:* Raw edits with no applicability class (unlike ruff's safe/unsafe).
  - Limits: One position per request.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.type-errors`; equivalent (unconsumed) `ty.quick-fixes`, `ruff.fixes`; difference: pyrefly edits have no applicability class.
- **`pyrefly.suppression-insertion`** (pyrefly; unit: edit) - Edits that insert `# pyrefly: ignore[kind]` comments above each line that has errors (placed after multi-line strings and continuations), or from a JSON error list. *Exact:* Writes files in place; the comment names the exact kinds.
  - Limits: Writes source files; there is no dry-run diff output.
  - Claim: **resolved**. Attach: PR-report (edits written to source lines).
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process through the collected errors already read (filter by ErrorKind; preset-dependent); CLI JSON otherwise.
  - Overlaps: derived from consumed `pyrefly.type-errors`; equivalent (unconsumed) `ruff.suppression-edits`, `ty.add-ignore`; difference: pyrefly writes files in place with no applicability class.
- **`ruff.fixed-source`** (ruff; unit: module) - The module text after iteratively applying all applicable fixes to convergence, the per-rule count of applied fixes, a source map between old and new text, and a unified diff. *Exact:* Exact; stops with a 'failed to converge' error after 100 iterations.
  - Limits: Organize imports (I001) is the only restructuring; no rename/extract/move refactors.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_diagnostics`, `ruff_linter` (also CLI).
  - Overlaps: none recorded
- **`ruff.fixes`** (ruff; unit: diagnostic) - The edits that fix a finding (content, start, end), the fix message, applicability (Safe, Unsafe, DisplayOnly) and isolation level (Group, NonOverlapping). *Exact:* Exact edits relative to the original source.
  - Limits: Per-rule fix safety at 0.16.8: 157 safe, 147 unsafe, 103 decided at run time, 12 mixed, 1 display-only, 59 not located, 491 none (skill join).
  - Claim: **resolved**. Attach: RF-report.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_diagnostics`, `ruff_wasm` (also CLI).
  - Overlaps: none recorded
- **`ruff.import-insertion`** (ruff; unit: module) - The insertion point for a new import: start of file after docstring and `__future__` imports, end of a given statement, into an existing from-import, or at the start of a block; inline (`;`) vs own-line. *Exact:* Exact positions respecting the module's style.
  - Claim: **exact syntactic**. Attach: RF-lex.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_importer`.
  - Overlaps: derived from consumed `ruff.ast`
- **`ruff.suppression-edits`** (ruff; unit: diagnostic) - The edits that add a noqa or ruff: ignore comment for given diagnostics, merging with existing directives. *Exact:* Exact edits.
  - Claim: **exact syntactic**. Attach: RF-report.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_linter` (also CLI).
  - Overlaps: none recorded
- **`ty.add-ignore`** (ty; unit: edit) - The edit that adds or extends a `# ty: ignore[rule]` comment for one diagnostic or for every diagnostic in a file. *Exact:* Executed: comments appended after existing trailing comments.
  - Claim: **exact syntactic**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: none recorded
- **`ty.fixes`** (ty; unit: edit) - Edits that resolve a diagnostic with ruff_diagnostics applicability (safe, unsafe, display-only); `ty check --fix` applies the safe ones. *Exact:* Shared Fix/Applicability model with ruff.
  - Limits: Only some lints attach fixes.
  - Claim: **resolved**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ruff_db); public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: none recorded
- **`ty.importer`** (ty; unit: edit) - The edit that makes `module.member` usable at a position, reusing an existing import, adding a `from` import or qualifying the name, given the members already in scope. *Exact:* Exact edit text; respects shadowing by names already in scope.
  - Claim: **resolved**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: derived from consumed `ty.module-resolution`
- **`ty.quick-fixes`** (ty; unit: edit) - For a diagnostic id and range: for `unresolved-reference`, preferred fixes that import or qualify the name; for any lint, an ignore-comment fix. *Exact:* Import candidates come from auto-import completion.
  - Claim: **resolved**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_ide`. Route: public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded

#### `diagnostics.formatting`

- **`ruff.formatted-source`** (ruff; unit: module) - The formatter's output for a module, notebook, Markdown code fences or a line range (best effort for ranges), whether a file would change, and the diff; `--check` with a structured format reports each unformatted file as an `unformatted` diagnostic carrying a safe fix edit. *Exact:* Deterministic output; range formatting may expand to enclosing statements.
  - Limits: `fmt: off/skip` regions are left verbatim. Markdown support formats Python fences only, not nested in block quotes.
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_markdown`, `ruff_python_formatter`, `ruff_wasm` (also CLI).
  - Overlaps: derived from consumed `ruff.ast`
- **`ruff.formatter-ir`** (ruff; unit: module) - The document of format elements (groups, indents, line breaks, best-fitting variants, source positions) the printer lays out for a module. *Exact:* Exact IR dump (debug text).
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_formatter`, `ruff_wasm`.
  - Overlaps: derived from consumed `ruff.ast`

#### `diagnostics.lint`

- **`ruff.lint-diagnostics`** (ruff; unit: diagnostic) - Each finding of an enabled rule: rule code and kebab name, message, primary range (and secondary annotations, sub-diagnostics, tags unnecessary/deprecated), file and notebook cell, the line where a noqa would go, documentation URL, severity, and an optional fix. *Exact:* Per rule; ranges exact, rendered as 1-based character columns.
  - Limits: JSON `severity` is always "error" unless --preview; under preview the JSON carries the rule's severity, `code` may be null, and human formats print rule names instead of codes. Selecting a Preview rule without --preview has no effect; 0.16.10 prints `warning: Selection X has no effect because ...
  - Claim: **diagnostic-only** (record says resolved; it is the finding stream itself). Attach: RF-report.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_db`, `ruff_linter`, `ruff_wasm` (also CLI).
  - Overlaps: derived from consumed `ruff.ast`

#### `diagnostics.metrics`

- **`pyrefly.error-counts`** (pyrefly; unit: project) - For a run, the number of errors of each kind (top N or all) and the number of errors grouped by a chosen path segment; with --summary=full, lines checked, time and memory. *Exact:* Counts of this run after severity filtering.
  - Limits: Printed as text only. Partial for this concept: counts only, printed as text; no complexity measures.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: needs a fork-patch extension (visibility of `pyrefly::error`); otherwise CLI only.
  - Overlaps: derived from consumed `pyrefly.type-errors`; equivalent (unconsumed) `ruff.statistics`
- **`ruff.metrics-in-messages`** (ruff; unit: function) - McCabe complexity (C901), counts of branches, returns, statements, arguments, positional arguments, locals, boolean expressions, nested blocks, public methods, try-clause statements (PLR09xx, PLR1702, PLW0717) and line/doc-line widths (E501, W505), each only when above a configured threshold. *Exact:* Exact numbers, but only in message text and only above the threshold; set thresholds low to observe values.
  - Limits: No data field; parse `({value} > {limit})` from the message.
  - Claim: **diagnostic-only**. Attach: RF-report.
  - Baseline: present at 0.0.14 (named items verified in source). Route: CLI only (`ruff` 0.16.8 = crates 0.0.14 line), or `ruff_linter` 0.16.8 in-process.
  - Overlaps: derived from consumed `ruff.ast`
- **`ruff.statistics`** (ruff; unit: project) - For a run: per rule code and name, the number of findings, whether the rule is fixable and how many findings are fixable. *Exact:* Exact counts for the run.
  - Limits: Partial for this concept: counts of findings per rule; no complexity or size measures.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: CLI only (`ruff` 0.16.8 = crates 0.0.14 line), or `ruff_linter` 0.16.8 in-process.
  - Overlaps: none recorded

#### `diagnostics.refactors`

- **`pyrefly.refactors`** (pyrefly; unit: edit) - Edits for extract function/variable/field/superclass, inline variable/method/parameter, introduce parameter, move module member, make a local function top-level, pull members up/push down, convert dict, convert star import, invert boolean, safe delete, change signature, rename (with prepare) and file renames that update imports. *Exact:* Raw workspace edits; no applicability class.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.expression-types`; equivalent (unconsumed) `ty.rename`
- **`ty.rename`** (ty; unit: edit) - Whether the symbol at a position can be renamed (its range) and the reference ranges a rename must edit, within the file or across files. *Exact:* Built on find_references in Rename/RenameMultiFile mode.
  - Limits: No other refactors (extract, inline, move).
  - Claim: **type-level inferred**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_ide`. Route: public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded

#### `diagnostics.rule-vocabulary`

- **`pyrefly.error-kind-catalog`** (pyrefly; unit: kind) - The ErrorKind enum: 148 kinds in kebab-case with default severity (error 80, warn 24, info 1, ignore 43), which presets report each, emission sites and documentation URLs. *Exact:* Exact enum at this pin.
  - Limits: The default severity describes only the default preset.
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: equivalent (unconsumed) `ty.lint-registry`, `ruff.rule-vocabulary`; difference: Vocabularies do not correspond one to one.
- **`ruff.linter-families`** (ruff; unit: rule) - The 59 upstream linters (pyflakes, pycodestyle, pylint...) with code prefixes, names and URLs, and upstream sub-categories (e.g. pylint C/E/R/W). *Exact:* Exact.
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_linter` (also CLI).
  - Overlaps: none recorded
- **`ruff.rule-mechanics`** (ruff; unit: rule) - For each rule: the checker pass (Ast, Tokens, PhysicalLines, LogicalLines, Imports, Noqa, Filesystem, Io, Toml), the AST node arm that dispatches it, whether it consults the semantic model, the preview gates it calls, and its fix-safety constructors. *Exact:* lint_source is exact; the per-rule joins are source-derived by the skill builder.
  - Limits: Skill tables are at 0.16.8 (970 rules); UP052 (Preview since 0.16.10) is not in them.
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: present at 0.16.8 (the record's tables are 0.16.8). Route: crates `ruff_linter`.
  - Overlaps: none recorded
- **`ruff.rule-vocabulary`** (ruff; unit: rule) - Every rule (971 at 0.16.10: 812 Stable, 142 Preview, 17 Removed) with code, name, linter, summary, message formats, fix availability (Always 250, Sometimes 230, None 491), category (correctness, suspicious, complexity, performance, style, security, formatting, pedantic, restriction), status with version, explanation and source location. *Exact:* Exact; generated from the rule registry.
  - Limits: Default-enabled categories are correctness, suspicious, complexity, performance and style. Test-only rules (e.g. PLW0101) are not listed.
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: present at 0.16.8 (970 rules per the skill's rule tables; UP052 arrives at 0.16.10); counts quoted in the record are 0.16.10. Route: crates `ruff_linter` (also CLI).
  - Overlaps: none recorded
- **`ty.lint-registry`** (ty; unit: rule) - The 138 lints ty declares (96 error, 25 warn, 17 ignore by default; 136 stable, `missing-direct-dependency` preview, `invalid-type-guard-call` removed), each with name, summary, documentation, default level, status with version, source location and aliases. *Exact:* Exact; 129 lints are declared in types/diagnostic.rs, 5 in suppression.rs, 4 in string_annotation.rs.
  - Claim: **exact syntactic**. Attach: LEVEL (rule).
  - Baseline: present at 0.0.14 with 134 lints, not 138: `invalid-init-type-variable`, `truthiness-test-of-callable`, `truthiness-test-of-iterable`, `truthiness-test-of-none-union` arrive at 0.0.16. Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: none recorded

#### `diagnostics.type-errors`

- **`pyrefly.expectations`** (pyrefly; unit: line) - Whether a module's errors match inline expectation comments, for conformance-style tests. *Exact:* Line-based comparison.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.type-errors`
- **`ty.check-reports`** (ty; unit: range) - `ty check` renders diagnostics as full (code frames), concise (`path:line:col: severity[rule] message`), GitLab Code Quality JSON (check_name, description, severity, fingerprint, location), GitHub workflow commands, or JUnit XML; there is no generic JSON format for diagnostics. *Exact:* Executed: all five formats.
  - Limits: Severities map to GitLab major/minor/info.
  - Claim: **diagnostic-only** (record says inferred; it is an output rendering of diagnostics). Attach: TY-report.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ruff_db).
  - Overlaps: none recorded
- **`ty.type-check-diagnostics`** (ty; unit: range) - The findings of checking a file or project: rule (lint) diagnostics with severity, primary and secondary annotated ranges, sub-diagnostics (info, help, the source of the Python version), tags (unnecessary, deprecated) and optional fixes; plus invalid-syntax, revealed-type, io, unknown-rule and configuration diagnostics. *Exact:* As precise as inference; `possibly-*` lints express ambiguous boundness.
  - Claim: **diagnostic-only** (record says inferred; the payload is a diagnostic stream). Attach: TY-range.
  - Baseline: present at 0.0.14 (lint set 134, see ty.lint-registry); also served by unpublished `ty_project`. Route: public in crates `cpg-flow` already links or their published siblings (ruff_db); public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); public in unpublished `ty_project` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: overlaps consumed `pyrefly.type-errors`; derived from consumed `ty.use-def`, `ty.parse-errors`

### directives

#### `directives.pragmas`

- **`pyrefly.generated-marker`** (pyrefly; unit: module) - Whether a file is marked `@generated`, and, with `ignore-errors-in-generated-code`, that its errors are not reported. *Exact:* Substring match on the text.
  - Limits: No other pragmas (fmt, isort) are recognised.
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: none recorded
- **`ruff.pragmas`** (ruff; unit: line) - `fmt: off/on/skip` and `yapf: disable/enable` comments and their own-line/end-of-line position; `isort: skip_file`, `isort: skip`, `isort: split`, `isort: off/on` directives with their lines. *Exact:* Exact.
  - Claim: **exact syntactic**. Attach: RF-lex.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_linter`, `ruff_python_trivia`.
  - Overlaps: none recorded

#### `directives.suppressions`

- **`pyrefly.suppressed-errors`** (pyrefly; unit: diagnostic) - For a run: the errors that inline suppressions silenced, separately from errors disabled by configuration and errors matched by a baseline. *Exact:* Exact per run; severity is applied at collection time.
  - Limits: CollectedErrors is in a private module: callable, but not nameable.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.type-errors`
- **`pyrefly.suppression-comments`** (pyrefly; unit: line) - Every suppression comment in a module with its line, tool (`type`, `pyrefly`, `pyright`, `mypy`, `ty`, `pyre`, `zuban`), the error codes it names and its effect; file-level `ignore-errors` (optionally with codes); which tools are honoured (`enabled-ignores`, `permissive-ignores`) and how unknown `type: ignore` tags behave. *Exact:* Parsed from comment tokens; a suppression naming the wrong kind suppresses nothing.
  - Limits: `# noqa` is not honoured. `# ty: ignore` only counts when the ty tool is enabled. The effect of a suppression appears only through CollectedErrors.suppressed.
  - Claim: **exact syntactic**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: equivalent (unconsumed) `ruff.noqa-directives`, `ty.suppressions`; difference: Each tool honours its own comment family; pyrefly accepts `ty: ignore` only when enabled and never honours noqa.
- **`pyrefly.suppression-inventory`** (pyrefly; unit: comment) - Per module, every suppression comment with its tool kind, the codes it names and its start location, plus the module's count of type ignores. *Exact:* Start position only (1-based line and column).
  - Claim: **exact syntactic**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: needs a fork-patch extension (visibility of `pyrefly::commands`); otherwise CLI only.
  - Overlaps: none recorded
- **`pyrefly.unused-suppressions`** (pyrefly; unit: comment) - Suppression comments that silenced nothing (`unused-ignore`, `unused-type-ignore`) and ones placed where they cannot apply (`misplaced-ignore`), with edits that remove them. *Exact:* Relative to the errors of this run and configuration.
  - Limits: unused-ignore and unused-type-ignore are off in the default preset; a different preset changes which comments count as used.
  - Claim: **diagnostic-only** (record says resolved; relative to the run's errors and preset). Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.type-errors`; equivalent (unconsumed) `ruff.unused-suppressions`, `ty.unused-suppressions`
- **`ruff.noqa-directives`** (ruff; unit: line) - Each `# noqa` / `# noqa: CODES` comment with its codes, the line range it covers (multi-line strings and continuations map to one noqa line), and file-level `# ruff: noqa[: CODES]` / `# flake8: noqa` exemptions. *Exact:* Exact; codes matched against rule codes and redirects.
  - Limits: ruff ignores `pyrefly: ignore`, `ty: ignore` and `type: ignore` (PGH003 only flags blanket type: ignore).
  - Claim: **exact syntactic**. Attach: RF-lex.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_linter` (also CLI).
  - Overlaps: none recorded
- **`ruff.range-suppressions`** (ruff; unit: range) - Suppression comments by kind: `# ruff: ignore[CODES]` for one (possibly multi-line) statement, `# ruff: disable[...]`/`# ruff: enable[...]` block pairs, and `# ruff: file-ignore[...]`, with their ranges. *Exact:* Exact.
  - Claim: **exact syntactic**. Attach: RF-lex.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_linter` (also CLI).
  - Overlaps: derived from consumed `ruff.ast`
- **`ruff.unused-suppressions`** (ruff; unit: line) - Findings that a noqa is unused or names unknown/redirected codes (RUF100, RUF101), a suppression comment is invalid or unmatched (RUF103, RUF104), a blanket noqa/type: ignore (PGH004, PGH003), or a misplaced formatter suppression (RUF028). *Exact:* Exact relative to the rules enabled in the same run (a noqa is only 'unused' for rules that ran).
  - Limits: RUF105/RUF106 (noqa-comments, codes in suppression comments) are Preview.
  - Claim: **diagnostic-only**. Attach: RF-report.
  - Baseline: present at 0.0.14 (named items verified in source). Route: CLI only (`ruff` 0.16.8 = crates 0.0.14 line), or `ruff_linter` 0.16.8 in-process.
  - Overlaps: none recorded
- **`ty.suppressions`** (ty; unit: line) - `# ty: ignore[rule, ...]` (per line, or file-level at the top) and `# type: ignore` comments with the rules and ranges they cover; `type: ignore` can be disabled by configuration. *Exact:* Matching by rule name and line range; comments before any code apply to the whole file.
  - Limits: Ignores other tools' comments (`noqa`, `pyrefly: ignore`).
  - Claim: **exact syntactic**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: internal only (`pub(crate)`): no published route without a ty fork; CLI/LSP route only.
  - Overlaps: derived from consumed `ty.parsed-module`
- **`ty.unused-suppressions`** (ty; unit: line) - Suppression comments that suppress nothing, name unknown rules, are malformed or blanket, as diagnostics with a removal fix. *Exact:* Executed: `--fix` removes unused comments.
  - Claim: **diagnostic-only** (record says syntactic; the result is relative to the run's diagnostics). Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: internal only (`pub(crate)`): no published route without a ty fork; CLI/LSP route only.
  - Overlaps: none recorded

#### `directives.todos`

- **`ruff.todo-directives`** (ruff; unit: line) - Comments carrying a TODO-style tag, with author/link/colon checks, reported by the TD and FIX rules. *Exact:* Exact tag detection; attributes only as findings.
  - Claim: **diagnostic-only**. Attach: RF-report.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_linter` (also CLI).
  - Overlaps: none recorded

### docs

#### `docs.deprecation`

- **`pyrefly.deprecation`** (pyrefly; unit: definition) - Definitions marked `@deprecated` (PEP 702) with their message, deprecated stdlib aliases, and `deprecated` diagnostics at uses; the deprecation is carried on exports, functions and classes. *Exact:* Decorator recognised by resolution.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch; parts behind private modules: covered by the existing patch (changes 3/5).
  - Overlaps: equivalent (unconsumed) `ty.deprecation`, `ruff.deprecation-diagnostics`
- **`ruff.deprecation-diagnostics`** (ruff; unit: expression) - Imports and uses of deprecated stdlib/typing names (UP035, UP026 mock, UP019...), configured banned APIs (TID251), Airflow removals (AIR3xx), NumPy/pandas deprecations (NPY201, PD); diagnostics tagged `deprecated`. *Exact:* Table- and configuration-driven by qualified name; does not read `@deprecated` decorators of user code.
  - Claim: **diagnostic-only**. Attach: RF-report.
  - Baseline: present at 0.0.14 (named items verified in source). Route: CLI only (`ruff` 0.16.8 = crates 0.0.14 line), or `ruff_linter` 0.16.8 in-process.
  - Overlaps: none recorded
- **`ty.deprecation`** (ty; unit: definition) - Whether a function, class or overload is marked `@deprecated` (PEP 702), with its message, and uses of it as `deprecated` diagnostics tagged Deprecated; symbol search reports deprecation too. *Exact:* Decorator recognised by type.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source); also served by unpublished `ty_ide`. Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded

#### `docs.docstrings`

- **`ty.docstrings`** (ty; unit: definition) - The docstring text of a module, class, function or attribute (including attribute docstrings after assignments), and for an overloaded stub the implementation's docstring. *Exact:* Exact string-literal extraction.
  - Claim: **exact syntactic**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core); public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: overlaps consumed `pyrefly.docstring-ranges`; derived from consumed `ty.definitions`, `ruff.ast`

#### `docs.parameter-docs`

- **`ty.parameter-docs`** (ty; unit: parameter) - A map from parameter name to its description, parsed from Google, NumPy or reST docstrings; used by signature help and hover. *Exact:* Style detection and section parsing are heuristic.
  - Claim: **heuristic**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_ide`. Route: public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: overlaps consumed `pyrefly.parameter-docs`

#### `docs.sections`

- **`ruff.docstring-diagnostics`** (ruff; unit: definition) - Missing docstrings per definition kind (D100-D107), style findings (D2xx-D4xx), undocumented parameters (D417), and pydoclint mismatches between documented and actual raises/returns/yields (DOC201-DOC502, Preview), with Google/NumPy/PEP 257 conventions. *Exact:* Sections are parsed internally; no data surface for docstring text or parameter maps.
  - Limits: DOC rules compare against `raise` statements syntactically, not exception propagation.
  - Claim: **diagnostic-only**. Attach: RF-report.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast` (also CLI).
  - Overlaps: derived from consumed `ruff.ast`
- **`ty.docstring-sections`** (ty; unit: definition) - Google, NumPy and reST sections (parameters, returns, raises and others) recognised so a docstring can be rendered as structured Markdown. *Exact:* Parsed for display only.
  - Limits: No data surface for sections other than parameters.
  - Claim: **heuristic**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_ide`. Route: public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded

### editor

#### `editor.code-lens`

- **`pyrefly.code-lens`** (pyrefly; unit: range) - Code lens entries for runnable `if __name__ == '__main__'` blocks and tests (Run / Test), with test and class names. *Exact:* Syntactic recognition of main guards and test functions.
  - Claim: **heuristic**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: none recorded

#### `editor.completions`

- **`pyrefly.completions`** (pyrefly; unit: position) - Names valid at a position (locals, members, keywords, dict keys, dataframe columns, module names) with kinds, and auto-import edits for names from other modules. *Exact:* Type-aware member lists.
  - Claim: **type-level inferred**. Attach: PR-pos.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: equivalent (unconsumed) `ty.completions`
- **`ty.completions`** (ty; unit: position) - Names valid at a position (scoped, attribute, import, from-import, submodule, expected string literals such as Literal or TypedDict keys) with kind (25 LSP kinds), type, qualified name, insert text, deprecation and type-check-only flags, and auto-import edits for unimported symbols. *Exact:* Scope-aware; attribute completions follow inferred types.
  - Claim: **type-level inferred**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); also served by unpublished `ty_ide`. Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded

#### `editor.document-symbols`

- **`pyrefly.document-symbols`** (pyrefly; unit: module) - The hierarchical outline of classes, functions, methods and variables in a module with kinds and ranges. *Exact:* From the AST and SymbolKind.
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.ast`; equivalent (unconsumed) `ty.document-symbols`
- **`ty.document-symbols`** (ty; unit: definition) - A module's symbols (12 kinds: module, class, method, function, variable, constant, property, field, constructor, parameter, type parameter, import) as a flat or hierarchical tree with ranges. *Exact:* Syntax-driven with constant-name heuristics.
  - Claim: **exact syntactic**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_ide`. Route: public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: derived from consumed `ty.parsed-module`

#### `editor.hover`

- **`pyrefly.hover`** (pyrefly; unit: position) - For a position: the type or signature, symbol kind and docstring (with verbosity levels). *Exact:* Same type as expression-types at that position.
  - Limits: UTF-16 positions.
  - Claim: **type-level inferred**. Attach: PR-pos.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: derived from consumed `pyrefly.expression-types`, `pyrefly.docstring-ranges`; equivalent (unconsumed) `ty.hover`
- **`ruff.noqa-hover`** (ruff; unit: position) - The explanation of the rule whose code sits under the cursor in a noqa comment. *Exact:* Exact for noqa codes; no hover anywhere else (no types, no names).
  - Limits: Partial for this concept: hover over a noqa code shows the rule's name and summary; no types or names.
  - Claim: **exact syntactic**. Attach: RF-report (LSP hover position in a noqa comment).
  - Baseline: present at 0.0.14 (named items verified in source). Route: CLI only (`ruff` 0.16.8 = crates 0.0.14 line), or `ruff_linter` 0.16.8 in-process.
  - Overlaps: none recorded
- **`ty.hover`** (ty; unit: position) - For a position: the signature, parameter, type or type alias of the symbol, TypedDict key info and the docstring (or fragment), as Markdown or plain text. *Exact:* Derived from expression types and docstrings.
  - Claim: **type-level inferred**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_ide`. Route: public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded

#### `editor.inlay-hints`

- **`pyrefly.inlay-hints`** (pyrefly; unit: position) - Inferred variable types, function return types, pytest fixture parameter types and call argument names shown inline, each with an insertable text edit. *Exact:* Same types as expression-types.
  - Claim: **type-level inferred**. Attach: PR-pos.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.expression-types`; equivalent (unconsumed) `ty.inlay-hints`; difference: pyrefly adds return-type and pytest-parameter hints.
- **`ty.inlay-hints`** (ty; unit: position) - Inferred variable types and call-argument parameter names shown inline in a range, with label parts that navigate to definitions and optional insertable text edits. *Exact:* Argument names come from argument binding.
  - Limits: No return-type hints.
  - Claim: **type-level inferred**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_ide`. Route: public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded

#### `editor.semantic-tokens`

- **`pyrefly.semantic-tokens`** (pyrefly; unit: token) - Token classifications by meaning (class, function, method, parameter, variable, property, module, type parameter...) with modifiers, optionally including syntax tokens, with full byte ranges. *Exact:* From bindings and SymbolKind.
  - Limits: Vocabulary differs from ty's.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: equivalent (unconsumed) `ty.semantic-tokens`; difference: Different token vocabularies.
- **`ty.semantic-tokens`** (ty; unit: token) - Each identifier and literal classified as one of 17 token types (namespace, class, parameter, self/cls parameter, variable, property, function, method, keyword, string, number, decorator, builtin constant, type parameter, operator, regexp) with modifiers definition, readonly, async, documentation. *Exact:* Classification uses inferred types (a callable variable can be a function token).
  - Claim: **type-level inferred**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_ide`. Route: public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: derived from consumed `ty.definitions`

#### `editor.server-capabilities`

- **`pyrefly.server-capabilities`** (pyrefly; unit: server) - The requests the language server answers (23 capabilities: call/type hierarchy, code actions, code lens, completion, declaration, definition, document highlight/symbols, folding, hover, implementation, inlay hints, notebook sync, references, rename, selection range, semantic tokens, signature help, type definition, workspace symbols, pull diagnostics), its position encoding (UTF-16 only), indexing modes, and the ... *Exact:* The servers' own initialize replies, captured by the skill.
  - Limits: One position per request; whole-project answers need the reports.
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: equivalent (unconsumed) `ty.server-capabilities`; difference: pyrefly's position encoding is UTF-16 only; ty negotiates.
- **`ty.server-capabilities`** (ty; unit: project) - `ty server` advertises: pull and workspace diagnostics, code actions (QuickFix), completion (triggers `.`, quotes), hover, signature help, definition, declaration, type definition, implementation, references, document highlight, rename with prepare, document and workspace symbols, semantic tokens (full and range), inlay hints, folding and selection ranges, call and type hierarchy, notebook sync, workspace folders, ... *Exact:* Read from source.
  - Limits: No code lens, formatting, call graph or type queries beyond hover.
  - Claim: **exact syntactic**. Attach: LEVEL (server).
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_server`. Route: internal only (`pub(crate)`): no published route without a ty fork; CLI/LSP route only.
  - Overlaps: none recorded

#### `editor.signature-help`

- **`pyrefly.signature-help`** (pyrefly; unit: position) - At a position inside a call: the candidate signatures (overloads), the active one, the active parameter, with parameter docs and byte ranges of parameters in the rendered signature. *Exact:* Chosen overload from the solver.
  - Claim: **type-level inferred**. Attach: PR-pos.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: needs a fork-patch extension (visibility of `pyrefly::error`); otherwise CLI only.
  - Overlaps: derived from consumed `pyrefly.parameter-docs`; equivalent (unconsumed) `ty.signature-help`
- **`ty.signature-help`** (ty; unit: position) - For a position inside a call, the candidate signatures with parameter labels and documentation, the active signature and the active parameter. *Exact:* From call_signature_details.
  - Claim: **type-level inferred**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_ide`. Route: public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded

#### `editor.structure`

- **`pyrefly.folding-and-selection`** (pyrefly; unit: range) - Foldable regions (code, comments, comment sections, `# region`) and syntactic selection expansion ranges. *Exact:* From the AST and comments.
  - Claim: **exact syntactic**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.ast`; equivalent (unconsumed) `ty.folding-ranges`, `ty.selection-range`
- **`ty.folding-ranges`** (ty; unit: range) - Foldable regions of a file: blocks, comments, import groups and `# region` markers. *Exact:* Exact over syntax and tokens.
  - Claim: **exact syntactic**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_ide`. Route: public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: derived from consumed `ty.parsed-module`
- **`ty.selection-range`** (ty; unit: position) - The nested syntactic ranges enclosing a position, innermost first. *Exact:* Exact over the AST.
  - Claim: **exact syntactic**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_ide`. Route: public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: derived from consumed `ty.parsed-module`

### environment

#### `environment.configuration`

- **`pyrefly.config-migration`** (pyrefly; unit: project) - A pyrefly configuration derived from an existing mypy or pyright configuration, including the error kinds that correspond to their settings. *Exact:* Mapping tables between tools' options; not a semantic equivalence.
  - Claim: **heuristic**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: needs a fork-patch extension (visibility of `pyrefly_config::migration`); otherwise CLI only.
  - Overlaps: none recorded
- **`pyrefly.effective-config`** (pyrefly; unit: project) - The configuration file in force (pyrefly.toml or [tool.pyrefly]), its sub-configs per path, the covered files, every setting after CLI overrides and presets, and configuration errors. *Exact:* Exact; dump-config is human text, not a data format.
  - Limits: dump-config is text only.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: equivalent (unconsumed) `ty.configuration`, `ruff.resolved-settings`
- **`pyrefly.presets-and-severities`** (pyrefly; unit: project) - Which error kinds are enabled at which severity for a module, from the preset (off, basic, legacy, default, strict, all) and per-kind overrides (`errors` table, --error/--warn/--info/--ignore), including per-path sub-configs. *Exact:* Exact; the enum default describes only the `default` preset.
  - Limits: `basic` does not report even bad-assignment; `strict` adds implicit-any-parameter.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: equivalent (unconsumed) `ty.rule-selection`
- **`ruff.config-schema`** (ruff; unit: project) - Every configuration key (183 at 0.16.10) with its documentation, default, value type, scope and example. *Exact:* Exact; generated from the options structs.
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: present at 0.16.8; the 183-key count is 0.16.10 (unverified at 0.16.8). Route: crates `ruff_options_metadata`, `ruff_workspace` (also CLI).
  - Overlaps: none recorded
- **`ruff.resolved-settings`** (ruff; unit: file) - The resolved settings that apply to one file after hierarchical config discovery and CLI overrides: enabled rule set, fixability, per-file ignores, target version, formatter options, file-resolver includes/excludes, analyze settings. *Exact:* Exact as ruff resolves it.
  - Limits: Only human-readable text on the CLI; use the Rust API or `ruff config` for structure.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_linter`, `ruff_workspace` (also CLI).
  - Overlaps: none recorded
- **`ty.configuration`** (ty; unit: project) - ty's settings as data: environment, src (include/exclude, ignore files, scripts), rules, terminal (output format, error-on-warning), analysis (strict generic narrowing, strict equality, respect `type: ignore`, allowed unresolved imports, imports replaced with Any) and per-file overrides; values from files keep their range; invalid settings become diagnostics. *Exact:* Exact; CLI `--config` overrides take precedence over files; `RangedValue` keeps where each value came from.
  - Limits: No CLI command dumps the merged configuration; ty.schema.json describes the schema only.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source); also served by unpublished `ty_project`. Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); public in unpublished `ty_project` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded
- **`ty.rule-selection`** (ty; unit: file) - Which lints are enabled at which severity for a file, after defaults, `[rules]`, CLI `--error/--warn/--ignore` and per-file overrides, with the source of each choice. *Exact:* Exact.
  - Limits: Unknown rule names produce `unknown-rule` diagnostics.
  - Claim: **resolved**. Attach: LEVEL (file).
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: none recorded

#### `environment.interpreter`

- **`ruff.target-version`** (ruff; unit: file) - The Python version ruff assumes for a file: `target-version`, else inferred from `project.requires-python`, overridden per path by `per-file-target-version`; drives the parser's version checks, stdlib tables and version-gated rules. *Exact:* Exact resolution of configuration; minimum version of a requires-python specifier.
  - Limits: No platform: ruff has no sys.platform setting (only version).
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_linter`, `ruff_python_ast` (also CLI).
  - Overlaps: overlaps consumed `pyrefly.sys-info`
- **`ty.program`** (ty; unit: project) - Each `Program` fixes a Python version, a platform (`all` or a `sys.platform` identifier) and a resolver environment; the version carries its source (config file with range, script metadata, pyvenv.cfg, installation layout, CLI, editor or default). *Exact:* Exact as configured; when unset the version is read from the environment (pyvenv.cfg or `lib/pythonX.Y` layout) or defaults.
  - Limits: `PythonPlatform::All` leaves `sys.platform` tests undecided.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core, ty_site_packages).
  - Overlaps: overlaps consumed `pyrefly.sys-info`

#### `environment.module-identity`

- **`pyrefly.module-identity`** (pyrefly; unit: module) - A module's dotted name and where it comes from: a file, a namespace directory, memory, bundled typeshed stdlib, bundled typeshed third-party or bundled third-party stubs; whether it is an `__init__`, a stub (interface) or executable source; whether it is first-party. *Exact:* Exact for the resolved handle.
  - Limits: Pysa module ids are run-local.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: equivalent (unconsumed) `ty.module-identity`, `ruff.module-identity`
- **`ruff.module-identity`** (ruff; unit: file) - The package root a file belongs to (walking `__init__.py`, honoring `namespace-packages`), whether a directory is a package, and the file's dotted module path. *Exact:* Directory heuristics; implicit namespace packages need configuration.
  - Claim: **heuristic**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_linter`, `ruff_python_ast`.
  - Overlaps: none recorded
- **`ty.module-identity`** (ty; unit: module) - For a resolved module: its dotted name, file (none for a namespace package), package or module, the search path it came from, whether it is a known stdlib module, whether it is `type_check_only`, and for a file the module it is (`file_to_module`) and, for a stub, the runtime module behind it. *Exact:* Exact over the search paths; `py.typed` partial/full is tracked internally (`PyTyped`).
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_module_resolver).
  - Overlaps: none recorded
- **`ty.module-listing`** (ty; unit: module) - Every top-level module or package (and submodules of a package) discoverable in an environment, respecting shadowing order. *Exact:* Directory enumeration over the search paths; used for import completions.
  - Limits: Enumeration can be disallowed for some directories (`enumeration_allowed`).
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_module_resolver).
  - Overlaps: none recorded

#### `environment.search-paths`

- **`pyrefly.build-system-db`** (pyrefly; unit: module) - For projects under a build system, which file each module name maps to per target, from a source database queried from Buck/Bazel or a manifest. *Exact:* As reported by the build system.
  - Limits: Requires the build tool; no output form beyond resolution itself.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch; parts behind private modules: need a patch extension.
  - Overlaps: none recorded
- **`pyrefly.search-paths`** (pyrefly; unit: project) - The ordered roots imports resolve against (search-path from args or file, inferred import root, fallback search path, site-package paths given or queried from the interpreter, build-system targets) with the source of each. *Exact:* Exact for the run's configuration.
  - Limits: Typeshed and bundled third-party stubs are searched after site-packages; the order is not a list element.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: equivalent (unconsumed) `ty.search-paths`, `ruff.analyze-search-paths`; difference: Different resolvers: ruff analyze graph uses ty's.
- **`ruff.analyze-search-paths`** (ruff; unit: project) - The first-party roots (`src`), the site-packages of the `--python` environment and the order they are searched in, for `ruff analyze graph`. *Exact:* Delegates to ty_module_resolver SearchPathSettings and ty_site_packages PythonEnvironment.
  - Limits: Built with an EMPTY vendored file system: no typeshed, so stdlib imports yield no edges. Without `--python` there is no site-packages; the environment is not auto-discovered (TODO in source).
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_graph` (also CLI).
  - Overlaps: derived from consumed `ty.module-resolution`
- **`ty.python-environment`** (ty; unit: project) - The interpreter environment ty analyses against (virtual or system), its `sys.prefix`, site-packages directories, real stdlib directory and version, with the origin of the prefix: config setting (with range), script metadata, `--python`, editor, uv metadata, `VIRTUAL_ENV`, `CONDA_PREFIX`, derived from pyvenv.cfg, local `.venv`, the running environment, or a Python binary. *Exact:* Exact over the files present; pyvenv.cfg is parsed (`PyvenvCfgParseErrorKind`); implementation (CPython, PyPy, GraalPy) is detected internally.
  - Limits: Discovery needs a real `System` (feature `os`). Conda base environments are distinguished from named ones via `CONDA_DEFAULT_ENV`/`_CONDA_ROOT`.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_site_packages).
  - Overlaps: none recorded
- **`ty.search-paths`** (ty; unit: project) - The ordered roots modules are resolved from: extra paths (including `PYTHONPATH` entries), first-party roots (project root or `src`), custom or vendored typeshed stdlib (or the real stdlib), site-packages, and editable-install roots read from `.pth` files; each path knows its kind. *Exact:* Exact over configuration and discovered environment.
  - Limits: `PYTHONPATH` is read by ty_project when building settings, not by the resolver crate.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_module_resolver).
  - Overlaps: none recorded
- **`ty.uv-environments`** (ty; unit: project) - When uv is available, the project's and each script's environment and workspace come from uv (`uv` metadata and sync), including workspace members and whether a script environment is available. *Exact:* As reported by the uv subprocess; failures become `uv-metadata` diagnostics (DiagnosticId::UvMetadata).
  - Limits: Requires a uv executable; otherwise ty falls back to its own discovery.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: partial at 0.0.14: `UvWorkspace` exists only at 0.0.16 (`UvEnvironments`, `UseUv` present); needs unpublished `ty_project`. Route: public in unpublished `ty_project` (git dependency on the ruff 0.16.8 tag); richer parts are internal.
  - Overlaps: none recorded

#### `environment.stdlib`

- **`pyrefly.bundled-typeshed`** (pyrefly; unit: project) - Which standard-library and bundled third-party stub modules exist for the configured version, and the resolved core types (`Stdlib`: int, str, tuple, typing forms...) every module uses. *Exact:* A specific typeshed snapshot bundled at this pin.
  - Limits: The typeshed snapshot can differ from ty's vendored typeshed and from ruff's stdlib table.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: equivalent (unconsumed) `ty.typeshed-versions`, `ruff.stdlib-modules`; difference: Three stdlib sources: pyrefly's bundled typeshed snapshot, ty's vendored typeshed VERSIONS, ruff's generated table.
- **`ruff.builtins-tables`** (ruff; unit: name) - Builtin names per minor version (with or without notebook/IPython names) and the version a builtin was added, builtin exceptions and iterators, magic globals, keywords and soft keywords, identifier validity, typing tables (standard generics, immutable types, PEP 585 aliases, TypedDict forms, simple magic-method return types), logging levels and open() modes. *Exact:* Static tables.
  - Limits: Hand-maintained; not derived from typeshed.
  - Claim: **exact syntactic**. Attach: LEVEL (name: static tables keyed by name and version).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_stdlib`.
  - Overlaps: none recorded
- **`ruff.stdlib-modules`** (ruff; unit: name) - Whether a top-level module name is in the standard library for Python 3.7 through 3.15, and whether it is a compiled-in builtin module. *Exact:* Static generated table keyed by minor version.
  - Limits: Generated from a script, not from typeshed VERSIONS: can disagree with ty's vendored typeshed and pyrefly's bundled typeshed.
  - Claim: **exact syntactic**. Attach: LEVEL (name: static table keyed by module name and minor version).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_stdlib`.
  - Overlaps: none recorded
- **`ty.known-symbols`** (ty; unit: definition) - Which stdlib modules, classes, functions and special forms ty recognises and gives built-in semantics (for example `KnownModule::Typing`, `KnownClass::Int`, `KnownFunction::IsInstance`/`Cast`/`RevealType`/`ImportModule`, `SpecialFormType`). *Exact:* Exact by module and name.
  - Claim: **resolved**. Attach: LEVEL (vocabulary of special-cased symbols).
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_module_resolver); public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: none recorded
- **`ty.typeshed-versions`** (ty; unit: module) - For each stdlib module, the Python version range typeshed's VERSIONS file gives; modules outside the target version do not resolve. *Exact:* Exact for the vendored (or custom) typeshed snapshot.
  - Limits: A different typeshed snapshot from pyrefly's bundled one and ruff's stdlib table.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_module_resolver, ty_vendored).
  - Overlaps: none recorded

### exceptions

#### `exceptions.handlers`

- **`pyrefly.except-handlers`** (pyrefly; unit: handler) - For each except clause, the exception class expression bound as a key and its type; clauses that can never run (`unreachable-except-clause`). *Exact:* Typed exception classes.
  - Limits: No propagation analysis.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: equivalent (unconsumed) `ty.try-except-flow`, `ruff.handled-exceptions`
- **`ruff.handled-exceptions`** (ruff; unit: statement) - The exception expressions an enclosing try handles, the handled-exception flags at a binding site (NameError, ModuleNotFoundError, ImportError, AttributeError), bound/unbound exception names, and handler findings (bare except E722, blind except BLE001, duplicate handler B025, try-except-pass S110). *Exact:* Syntactic over try/except nesting.
  - Limits: No exception hierarchy reasoning; no may-raise sets for calls. Partial for this concept: no exception hierarchy and no unreachable-handler reasoning.
  - Claim: **resolved**. Attach: PR-ast (`extract_handled_exceptions`) + RF-checker (binding flags).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`, `ruff_python_semantic` (also CLI).
  - Overlaps: derived from consumed `ruff.ast`
- **`ty.try-except-flow`** (ty; unit: statement) - Which bindings from a `try` body an `except`/`finally` suite can see, through exception checkpoints that assume any statement may raise; whether a caught type is a valid exception class. *Exact:* Conservative may-raise at every statement.
  - Limits: No per-call may-raise sets; no unreachable-handler detection. Partial for this concept: handler visibility only; no unreachable-handler analysis.
  - Claim: **resolved**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: derived from consumed `ty.use-def`

#### `exceptions.raised-types`

- **`pyrefly.raise-checks`** (pyrefly; unit: statement) - The type of each `raise` operand (in the type traces) and diagnostics for raising non-exceptions (`bad-raise`). *Exact:* Per raise statement.
  - Limits: No may-raise set per function.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: derived from consumed `pyrefly.expression-types`; equivalent (unconsumed) `ty.raise-checks`
- **`ty.raise-checks`** (ty; unit: statement) - The inferred type of a `raise` operand and its cause, and whether it is an exception class or instance. *Exact:* Per statement.
  - Limits: No aggregation per function.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: none recorded

### exports

#### `exports.dunder-all`

- **`ruff.dunder-all`** (ruff; unit: module) - The names in a module's `__all__` (assignments, `+=`, `.extend`, `.append`, list/tuple concatenation) with ranges, and flags when it is not a list/tuple of string literals. *Exact:* Literal extraction; `__all__` built from another module's `__all__` is recorded but not expanded.
  - Limits: One module only.
  - Claim: **exact syntactic**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic` (also CLI).
  - Overlaps: overlaps consumed `pyrefly.dunder-all`; derived from consumed `ruff.ast`
- **`ty.dunder-all`** (ty; unit: module) - The names in a module's `__all__`, understanding literal lists/tuples, `+=`, `.append/.extend/.remove`, and `__all__.extend(sub.__all__)` / `+= sub.__all__`; `None` when it cannot be determined statically. *Exact:* Exact for the supported forms.
  - Limits: Dynamic manipulation yields no answer rather than a partial list.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: internal only (`pub(crate)`): no published route without a ty fork; no route.
  - Overlaps: overlaps consumed `pyrefly.dunder-all`; derived from consumed `ty.parsed-module`, `ty.module-resolution`

#### `exports.module-exports`

- **`ty.module-exports`** (ty; unit: module) - The global names a module binds for importers, following `__all__` and re-export conventions (used for star imports, completions and auto-import). *Exact:* Exact over the module's global scope.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: internal only (`pub(crate)`): no published route without a ty fork; no route.
  - Overlaps: overlaps consumed `pyrefly.export-map`; derived from consumed `ty.definitions`

#### `exports.re-exports`

- **`ruff.explicit-reexports`** (ruff; unit: binding) - Whether an import binding is an explicit re-export (`import a as a`, `from m import x as x`, or listed in `__all__`), which F401 treats as used, especially in `__init__.py` and stubs. *Exact:* Syntactic convention; no cross-module public-name computation.
  - Claim: **exact syntactic**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic` (also CLI).
  - Overlaps: overlaps consumed `pyrefly.reexport-kinds`
- **`ty.reexport-flag`** (ty; unit: import) - Whether an import definition counts as a re-export (stub conventions: `import x as x`, `from m import y as y`, `from . import z`; submodule imports always). *Exact:* Exact convention test.
  - Claim: **exact syntactic**. Attach: TY-id.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core).
  - Overlaps: overlaps consumed `pyrefly.reexport-kinds`

#### `exports.star-imports`

- **`ruff.star-imports`** (ruff; unit: statement) - Each `from m import *` (module and level) recorded on its scope; names are never expanded. *Exact:* Exact as statements; a read in a scope with a star import resolves to ReadResult::WildcardImport.
  - Limits: Which names a star import binds is unknown to ruff.
  - Claim: **exact syntactic**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic` (also CLI).
  - Overlaps: overlaps consumed `pyrefly.wildcard-set`; derived from consumed `ruff.ast`
- **`ty.star-import-names`** (ty; unit: import) - One `StarImport` definition per name the target module exports (honouring `__all__`, excluding underscore names otherwise, fixpoint over cycles), each guarded by a `StarImportPlaceholder` predicate so a name that may not exist stays possibly-unbound. *Exact:* Exact for the export rules; an unresolved target gives no definitions.
  - Limits: The underlying `exported_names` is private (`mod re_exports`).
  - Claim: **resolved**. Attach: TY-id (one definition per exported name at the star-import range).
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core).
  - Overlaps: overlaps consumed `pyrefly.wildcard-set`; derived from consumed `ty.module-resolution`

### frameworks

#### `frameworks.orm-and-models`

- **`pyrefly.model-frameworks`** (pyrefly; unit: class) - Django model fields and reverse relations (per-module relation index), DRF serializer kinds, pydantic model kinds, config dicts, alias generators and validators, attrs field specifiers and decorator methods, marshmallow schemas and factory_boy factories, as class metadata and synthesized fields. *Exact:* Recognised by resolved base classes and decorators.
  - Limits: No report surface; in-process or via synthesized-field effects in types.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: derived from consumed `pyrefly.class-metadata`; equivalent (unconsumed) `ty.pydantic-models`, `ruff.framework-rules`
- **`ruff.framework-rules`** (ruff; unit: expression) - Findings specific to Django models and views (DJ), FastAPI routes (FAST), Airflow (AIR), pandas (PD), NumPy (NPY) and similar, by qualified name. *Exact:* Pattern-based by qualified names; no model or relation facts.
  - Claim: **diagnostic-only**. Attach: RF-report.
  - Baseline: present at 0.0.14 (named items verified in source). Route: CLI only (`ruff` 0.16.8 = crates 0.0.14 line), or `ruff_linter` 0.16.8 in-process.
  - Overlaps: none recorded
- **`ty.pydantic-models`** (ty; unit: class) - Pydantic `BaseModel` subclasses: model config booleans, field metadata and defaults, synthesized `__init__`, setattr behaviour, and discarded extra arguments. *Exact:* Built-in special casing.
  - Limits: Not public; no Django or attrs-specific model.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: internal only (`pub(crate)`): no published route without a ty fork; CLI/LSP route only.
  - Overlaps: none recorded

#### `frameworks.pytest`

- **`pyrefly.pytest-fixtures`** (pyrefly; unit: parameter) - Fixture definitions in a module and its conftest chain, which fixture binds each test parameter (with its type), fixture parameter references, and inlay hints / code actions for fixture types. *Exact:* Name-based fixture lookup with types from the fixture's return.
  - Limits: No public fixture graph as data.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.expression-types`; equivalent (unconsumed) `ty.pytest-fixtures`
- **`ruff.pytest-rules`** (ruff; unit: definition) - pytest fixture/parametrize/raises/assert style findings (PT001-PT031) and whether a function is a test by name. *Exact:* Syntactic by decorator qualified names; no fixture binding to test parameters.
  - Claim: **diagnostic-only**. Attach: RF-report.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic` (also CLI).
  - Overlaps: none recorded
- **`ty.pytest-fixtures`** (ty; unit: parameter) - For a test or fixture parameter, the fixture declaration it binds and the equally viable exposures (local binding, source binding, explicit name) through conftest, class, module and plugin scopes; which fixtures a definition exposes; global plugin files. *Exact:* Static fixture lookup over conftest and plugin modules.
  - Claim: **resolved**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: derived from consumed `ty.module-resolution`

#### `frameworks.tensor-shapes`

- **`pyrefly.tensor-shapes`** (pyrefly; unit: expression) - Shape types for arrays and tensors (ShapedArray, IntTuple, NamedInts, NNModule; jaxtyping and `Shaped[T, "..."]` annotations; einops/einsum), polars/pandas DataFrame and Series column schemas with in-place column mutations, and diagnostics for unknown/duplicate columns, schema and type mismatches, plus PyTorch efficiency lints. *Exact:* Experimental; enabled by configuration.
  - Limits: Off unless tensor-shapes is enabled; variants are experimental.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: derived from consumed `pyrefly.type-terms`

### identity

#### `identity.analyzer-introspection`

- **`pyrefly.debug-info`** (pyrefly; unit: key) - Per module, every key with its location, its binding (how it is computed) and its solved result as display strings, plus the module's errors. *Exact:* Display strings, not structured data.
  - Limits: For debugging; format not stable.
  - Claim: **type-level inferred**. Attach: PR-key (display strings).
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process through the existing fork patch (changes 3/5 open `report`/`export`/`coverage::collect`).
  - Overlaps: derived from consumed `pyrefly.expression-types`
- **`pyrefly.timings-and-memory`** (pyrefly; unit: module) - How long each analysis step (load, parse, bindings, answers, solutions) took per module, and the count and size of binding/key entries per module and type. *Exact:* Measurements of this run.
  - Limits: --report-timings re-processes each module individually; the file is CSV at this pin although the skill catalog lists it as JSON.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API); parts behind private modules: covered by the existing patch (changes 3/5).
  - Overlaps: none recorded

#### `identity.incrementality`

- **`pyrefly.session`** (pyrefly; unit: project) - The in-process session that produces every other fact: a State with config finder, transactions that load handles at a Require level (Exports, Errors, Everything, ...), run, invalidate and commit; what is retained (AST, answers, solutions) depends on the level. *Exact:* Facts are relative to the handles, Require level and configuration of the transaction.
  - Limits: Session API is #[doc(hidden)] and changes between dev releases. Answers are evicted unless retained; Solutions persist for exported keys.
  - Claim: **resolved**. Attach: LEVEL (project: the session itself).
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: none recorded
- **`pyrefly.solution-changes`** (pyrefly; unit: module) - For a re-solved module, which exported keys changed relative to the previous solutions or answers, driving incremental invalidation. *Exact:* Exact per pair of solutions.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.export-map`

#### `identity.provenance`

- **`ruff.config-value-provenance`** (ruff; unit: value) - For a configuration value: whether it came from a file (with its range), the CLI, a Python editor extension or a PEP 723 script block. *Exact:* Exact spans for TOML-sourced values.
  - Limits: ruff_workspace settings are plain values; ranged values are used by ty.
  - Claim: **resolved**. Attach: LEVEL (configuration value: TOML spans).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_ranged_value`.
  - Overlaps: none recorded
- **`ruff.version-identity`** (ruff; unit: run) - The ruff version and, for git builds, commit info; the version each rule was introduced, promoted or removed. *Exact:* Exact.
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: CLI only (`ruff` 0.16.8 = crates 0.0.14 line), or `ruff_linter` 0.16.8 in-process.
  - Overlaps: none recorded
- **`ty.program-file`** (ty; unit: file) - Semantic facts are keyed by `ProgramFile` = (`PythonFile`, `Program`), so a file imported by a project and by a PEP 723 script with a different version or environment gets separate indexes, types and diagnostics; parsing is shared by version, resolution by resolver environment. *Exact:* Exact identity; salsa-interned, so ids are run-local.
  - Limits: Salsa ids are not stable across runs; join on (path, range) instead.
  - Claim: **resolved**. Attach: ID (`ProgramFile`, salsa-interned, run-local).
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_module_resolver, ty_python_core).
  - Overlaps: none recorded

#### `identity.run-local-ids`

- **`pyrefly.pysa-module-ids`** (pyrefly; unit: id) - Pysa's numeric module ids, function ids (F:n, CF:class:n for synthesized, MTL/CTL for top levels) and class ids that cross-reference its definitions, call graphs and type tables. *Exact:* Stable within one run only.
  - Limits: Not stable across runs or edits; join across runs by qualified name and range.
  - Claim: **resolved**. Attach: ID (Pysa ids; stable within one run only).
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: none recorded
- **`ruff.node-index`** (ruff; unit: node) - A NodeIndex for each AST node, assigned in a parse, usable as a dense key within that tree. *Exact:* Unique within one parse.
  - Limits: Run-local: shifts with any edit; not an identity across runs or files.
  - Claim: **exact syntactic**. Attach: ID (one parse; pyrefly's retained 0.0.14 AST carries the same `NodeIndex` type).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`.
  - Overlaps: derived from consumed `ruff.ast`

#### `identity.stable-ids`

- **`ruff.diagnostic-fingerprint`** (ruff; unit: diagnostic) - A per-finding fingerprint hashed from rule name and project-relative path, disambiguated by collision order. *Exact:* Stable across line moves, but the n-th finding of the same rule in a file gets a salted hash that depends on ordering.
  - Limits: std DefaultHasher: deterministic for a given build, not a documented cross-version contract. Partial for this concept: deterministic for one build; not a documented cross-version identity.
  - Claim: **exact syntactic**. Attach: LEVEL (diagnostic: hash of rule + path, no position).
  - Baseline: present at 0.0.14 (named items verified in source). Route: CLI only (`ruff` 0.16.8 = crates 0.0.14 line), or `ruff_linter` 0.16.8 in-process.
  - Overlaps: equivalent (unconsumed) `ty.diagnostic-fingerprint`; difference: The same ruff_db GitLab renderer computes both, so the shape and hashing agree; only the rules differ.
- **`ty.diagnostic-fingerprint`** (ty; unit: range) - A hex fingerprint per diagnostic in the GitLab report, hashed from the rule name and the project-relative path (salted again on collision), independent of line numbers. *Exact:* Stable across edits that move a diagnostic within a file; order-dependent when one file has several diagnostics of the same rule.
  - Limits: Uses Rust's DefaultHasher, which is not guaranteed stable across Rust releases. Partial for this concept: deterministic for one build; not a documented cross-version identity.
  - Claim: **exact syntactic**. Attach: LEVEL (diagnostic: hash of rule + path, no position).
  - Baseline: present at 0.0.14 (CLI surface; behaviour transfer unverified). Route: CLI/report only (`ty` 0.0.14 binary built from the ruff 0.16.8 tag).
  - Overlaps: equivalent (unconsumed) `ruff.diagnostic-fingerprint`; difference: The same ruff_db GitLab renderer computes both, so the shape and hashing agree; only the rules differ.

### imports

#### `imports.categories`

- **`ruff.import-categories`** (ruff; unit: import) - The isort section an import belongs to: Future, StandardLibrary, ThirdParty, FirstParty or LocalFolder, or a user-defined section. *Exact:* Known lists, `src` roots and the stdlib table; no file resolution.
  - Limits: Not exposed as data on the CLI; an import missing from src roots defaults to third-party.
  - Claim: **heuristic**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_linter` (also CLI).
  - Overlaps: none recorded
- **`ty.search-path-category`** (ty; unit: module) - Whether a resolved module comes from the standard library, a first-party root, an extra path, site-packages or an editable install. *Exact:* By resolution, not by name lists: a local `json.py` on a first-party root is first-party.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_module_resolver).
  - Overlaps: derived from consumed `ty.module-resolution`

#### `imports.declared-dependencies`

- **`ty.declared-dependencies`** (ty; unit: import) - Whether an imported module belongs to a distribution the importing project (or PEP 723 script) declares directly: runtime and optional dependencies always, dependency groups only outside package code, the project's own distribution never; violations are `missing-direct-dependency`. *Exact:* Exact against uv's metadata (module owners, editable paths); nested projects use their own declarations.
  - Limits: Needs uv metadata; preview lint, off by default.
  - Claim: **resolved**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: derived from consumed `ty.module-resolution`

#### `imports.graph`

- **`pyrefly.demand-tree`** (pyrefly; unit: edge) - Every cross-module lookup a run made: from module, target module and kind (an answer for a named key such as KeyClassMetadata, or an export lookup with a reason such as get_wildcard, export_exists, is_special_export), plus the last step each module reached. *Exact:* Records actual demand, one entry per lookup (not deduplicated).
  - Limits: Very large: 254,315 edges (45 MB) for a two-file fixture; mostly typeshed-internal.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.import-resolution`
- **`pyrefly.module-dependency-graph`** (pyrefly; unit: module) - Direct module-to-module dependencies of every on-disk module in a run (absolute paths), and in-process the dependency graph, transitive reverse dependencies and whether anything depends on a module. *Exact:* Edges are the imports pyrefly actually loaded.
  - Limits: CLI output lists on-disk modules only (bundled stubs omitted) and is marked experimental/unstable. Module-level, not name-level.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API); parts behind private modules: covered by the existing patch (changes 3/5).
  - Overlaps: derived from consumed `pyrefly.import-resolution`; equivalent (unconsumed) `ruff.import-graph`, `ty.imported-modules`; difference: pyrefly: absolute on-disk paths only, module level; ruff: file level via ty's resolver.
- **`pyrefly.name-level-imports`** (pyrefly; unit: import) - For each import, the qualified name it introduces in the importing module and the qualified name it refers to, so imports form a name-to-name graph across files. *Exact:* Targets are fully qualified names; xrefs.XRefsByFile.1 adds the target file.
  - Limits: Order of facts varies between runs; compare as sets.
  - Claim: **resolved**. Attach: PR-report.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: CLI report only (second pyrefly run).
  - Overlaps: none recorded
- **`ruff.import-graph`** (ruff; unit: file) - For each Python file in scope, the set of project files it imports (dependencies) or that import it (dependents), with imports resolved by ty's module resolver; optionally including TYPE_CHECKING imports and string literals that look like module paths. *Exact:* Exact per the resolver; stubs are mapped back to their source file when one exists.
  - Limits: Values are file paths only; no module names, no per-import-statement mapping (the collector and resolver are private). No `--output-format`; a non-existent path yields `{}` and exit 0. TYPE_CHECKING detection is syntactic (`TYPE_CHECKING` / `typing.TYPE_CHECKING`; aliases missed). No vendored ...
  - Claim: **resolved**. Attach: LEVEL (file-to-file edges).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_graph` (also CLI).
  - Overlaps: derived from consumed `ruff.ast`, `ty.module-resolution`
- **`ty.imported-modules`** (ty; unit: file) - The module names each file imports (as written, absolute after relative resolution), the basis for the file's dependency edges. *Exact:* Names, not files: resolve each with `resolve_module` for edges.
  - Limits: No reverse (dependents) index and no whole-project graph export; `ruff analyze graph` builds one on this resolver.
  - Claim: **exact syntactic**. Attach: LEVEL (file: module names, no ranges).
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core).
  - Overlaps: none recorded

#### `imports.statements`

- **`pyrefly.import-bindings`** (pyrefly; unit: statement) - Each `import`/`from ... import` with its alias and the module/name it binds, as definitions (`DefinitionStyle::Import*`), binding keys (`Key::Import`) and Glean import declarations (`as_name` -> `from_name`), including star imports and their locations. *Exact:* Relative imports are absolutised against the importing module (ModuleName::new_maybe_relative).
  - Limits: Glean declares imports from an AST walk, including ones inside statically false branches.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: derived from consumed `pyrefly.ast`; equivalent (unconsumed) `ruff.import-statements`, `ty.import-statements`
- **`ruff.import-statements`** (ruff; unit: statement) - Each import and from-import with module, names, aliases and relative level as written; in the semantic model, the import bindings (Import, FromImport, SubmoduleImport, FutureImport) with their qualified names and PEP 810 laziness. *Exact:* Exact as written; laziness is Unknown when it depends on runtime state.
  - Claim: **exact syntactic**. Attach: PR-ast (statements) + RF-checker (import bindings, laziness).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`, `ruff_python_semantic`.
  - Overlaps: derived from consumed `ruff.ast`
- **`ty.import-statements`** (ty; unit: import) - Each `import`/`from ... import` alias as a definition (Import, ImportFrom, ImportFromSubmodule, StarImport) with its alias node, relative level and re-export flag. *Exact:* Exact; `from . import x` inside a package `__init__` can also create an `ImportFromSubmodule` binding.
  - Claim: **exact syntactic**. Attach: TY-id.
  - Baseline: present at 0.0.14, but there imports are declaration and binding; at 0.0.16 bindings only (migration page). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core).
  - Overlaps: derived from consumed `ruff.ast`, `ty.definitions`

### lexis

#### `lexis.style`

- **`ruff.stylist`** (ruff; unit: module) - The module's preferred quote character, indentation string and line ending, detected from the first occurrences in the token stream. *Exact:* First-seen style, not a vote; defaults when absent.
  - Claim: **heuristic**. Attach: RF-lex (module-level result).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_codegen`.
  - Overlaps: none recorded

#### `lexis.tokens`

- **`pyrefly.retained-tokens`** (pyrefly; unit: token) - pyrefly keeps ruff's token stream (`ParsedModule.tokens`) next to the AST when a module is kept at `Require::Everything`; it is used for comment scanning and syntax semantic tokens. *Exact:* Identical to ruff 0.0.14's lexer output over the same bytes.
  - Limits: Not reachable without a fork; re-lexing with ruff is the only public route.
  - Claim: **exact syntactic**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: needs a fork-patch extension (`pub(crate)`/private item); otherwise CLI/LSP only.
  - Overlaps: none recorded
- **`ruff.name-prefilter`** (ruff; unit: file) - Whether a source may contain a given identifier (NFKC-aware: sources that change under normalization are accepted) or keyword (literal spelling); a candidate filter, not an occurrence fact. *Exact:* No false negatives by design; matches inside comments and strings are included (false positives).
  - Limits: Callers must confirm candidates with the AST or semantic analysis.
  - Claim: **heuristic**. Attach: LEVEL (file: candidate filter).
  - Baseline: only at 0.0.16: `ruff_python_trivia::NameMatcher` is absent from ruff_python_trivia 0.0.14. Route: crates `ruff_python_trivia`.
  - Overlaps: none recorded
- **`ruff.tokens`** (ruff; unit: token) - Every token of a module with its kind (107 TokenKind variants, including Comment, NonLogicalNewline, Indent/Dedent, f/t-string parts, IpyEscapeCommand, keywords and soft keywords), flags (quote style, prefix, triple-quoted, unclosed, non-ASCII identifier) and exact byte range. *Exact:* Exact UTF-8 byte ranges; the BOM is consumed only when lexing from offset 0. Soft keywords used as names are re-kinded to Identifier by the parser.
  - Limits: 0.0.16 renamed TokenKind::Name to TokenKind::Identifier and TokenFlags::NON_ASCII_NAME / is_non_ascii_name to NON_ASCII_IDENTIFIER / is_non_ascii_identifier (pyrefly 1.4.0-dev.3 embeds 0.0.14 and still uses Name). `Tokens::before/after/in_range` panic on an offset inside a token.
  - Claim: **exact syntactic**. Attach: RF-lex.
  - Baseline: present at 0.0.14 with `TokenKind::Name` / `TokenFlags::NON_ASCII_NAME` (renamed `Identifier` / `NON_ASCII_IDENTIFIER` at 0.0.16). Route: crates `ruff_python_ast`, `ruff_python_parser`, `ruff_wasm`.
  - Overlaps: none recorded

#### `lexis.trivia`

- **`pyrefly.comment-sections`** (pyrefly; unit: comment) - Comment lines of the form `# Title ----` (any of -, #, = repeated four or more times), with their nesting level from the number of `#`, as foldable section headers. *Exact:* Regex over physical lines: `^(#+)\s+(.+?)\s+([-#=]{4,})\s*$`.
  - Claim: **exact syntactic**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: none recorded
- **`pyrefly.continuation-ranges`** (pyrefly; unit: range) - Sorted ranges of multi-line strings, backslash continuations and bracketed continuations in a module, used to place suppression comments on the line where they take effect. *Exact:* Computed from the AST and text.
  - Limits: Only consumed by suppression placement; no CLI output.
  - Claim: **exact syntactic**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.ast`
- **`ruff.comment-placement`** (ruff; unit: range) - For every comment: the node it is leading, trailing or dangling on, and whether it is own-line or end-of-line, as the formatter decides. *Exact:* Formatter's placement rules; textual (debug) output only.
  - Limits: Only a debug string; the comment map type is crate-internal.
  - Claim: **exact syntactic**. Attach: RF-lex (debug text only).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_formatter`, `ruff_python_trivia`, `ruff_wasm`.
  - Overlaps: derived from consumed `ruff.ast`
- **`ruff.comment-ranges`** (ruff; unit: range) - The byte range of every comment in a module, and whether a range or line contains, starts with or is followed by a comment; ranges of parenthesized expressions. *Exact:* Exact; built from Comment tokens.
  - Claim: **exact syntactic**. Attach: RF-lex.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_index`, `ruff_python_trivia`.
  - Overlaps: none recorded
- **`ruff.indexer-ranges`** (ruff; unit: range) - Line starts of backslash continuations, ranges of triple-quoted/multi-line strings, ranges of f-/t-strings, and whether a statement shares its line with another (`;`). *Exact:* Exact.
  - Claim: **exact syntactic**. Attach: RF-lex.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_index`.
  - Overlaps: none recorded
- **`ruff.trivia-tokenizer`** (ruff; unit: token) - Tokens (whitespace, comments, parentheses, commas, continuations, keywords) re-lexed forwards or backwards from any offset of raw text, without a parse. *Exact:* Exact within trivia; deliberately not a full lexer (strings are Other/Bogus).
  - Limits: Must start outside strings; behaviour inside string literals is undefined.
  - Claim: **exact syntactic**. Attach: TEXT.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_trivia`.
  - Overlaps: none recorded

### project

#### `project.file-set`

- **`pyrefly.project-files`** (pyrefly; unit: project) - The files in the project from project-includes/excludes (with heuristics, ignore files and sub-configs) or the explicit file list, and their count. *Exact:* Exact for the configuration.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: equivalent (unconsumed) `ty.project-files`, `ruff.file-set`
- **`ruff.file-set`** (ruff; unit: file) - The Python, stub and notebook files ruff would check or format under a path after include/extend-include, exclude/extend-exclude, .gitignore and force-exclude. *Exact:* Exact for the configuration.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_workspace` (also CLI).
  - Overlaps: none recorded
- **`ty.project-files`** (ty; unit: project) - The files ty checks: include/exclude globs, `.gitignore`/`.ignore` respect, force-exclude, PEP 723 scripts included or excluded, open files in the editor, with whether a path is included and why. *Exact:* Exact over the file system walk.
  - Claim: **resolved**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_project`. Route: public in unpublished `ty_project` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded

#### `project.symbol-index`

- **`pyrefly.workspace-symbols`** (pyrefly; unit: symbol) - Symbols matching a query across indexed modules, with fuzzy score, module, name, kind, range and immediate parent; and the fuzzy/exact export search used for auto-import. *Exact:* Over exports of indexed modules.
  - Limits: Depends on --indexing-mode and --workspace-indexing-limit.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.export-map`; equivalent (unconsumed) `ty.symbol-search`
- **`ty.symbol-search`** (ty; unit: definition) - Symbols matching a query across the workspace (workspace symbols) or across all modules importable from a file (all symbols), with qualified name, kind, module, file and deprecation. *Exact:* Fuzzy query match over module global symbols.
  - Claim: **resolved**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_ide`. Route: public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded

#### `project.tests`

- **`pyrefly.test-modules`** (pyrefly; unit: function) - Whether a module is a test module (Pysa is_test), and code-lens entries for runnable tests and main blocks with test name, class name and unittest flag. *Exact:* Name/import based recognition of test modules and test functions.
  - Limits: Not a pytest collection; parametrisation not expanded.
  - Claim: **heuristic**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API); parts behind private modules: covered by the existing patch (changes 3/5).
  - Overlaps: derived from consumed `pyrefly.ast`; equivalent (unconsumed) `ty.pytest-tests`
- **`ty.pytest-tests`** (ty; unit: definition) - The test functions and methods in a file under pytest's naming and class rules or because their class subclasses `unittest.TestCase`, with the binding, the function definition and enclosing classes. *Exact:* Default collection conventions only.
  - Limits: Ignores pytest configuration (python_functions, python_classes).
  - Claim: **resolved**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: derived from consumed `ty.definitions`

### references

#### `references.cross-module`

- **`pyrefly.cross-references`** (pyrefly; unit: span) - Each reference span in a file with the qualified name (and defining file) it targets, grouped by file and by target, including references inside string-literal annotations. *Exact:* Byte spans; targets resolved by pyrefly's bindings and types.
  - Limits: Order of facts varies between runs. No read/write kind per reference. Schema predicates DeclarationUses, MethodOverrides and DerivedClassToBase are not emitted.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process through the existing fork patch (changes 3/5 open `report`/`export`/`coverage::collect`).
  - Overlaps: derived from consumed `pyrefly.expression-types`
- **`pyrefly.find-references`** (pyrefly; unit: definition) - For a definition (from a position), all its references in the module and across the project, local occurrences and document highlights, and implementations. *Exact:* Exact within indexed files; LSP indexing mode decides scope.
  - Limits: One position per request; UTF-16 positions. Global references depend on `--indexing-mode`.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: equivalent (unconsumed) `ty.find-references`
- **`ty.find-references`** (ty; unit: use) - All references to the symbol at a position, across the project's files, each classified Read, Write or Other, optionally without the declaration; document highlights restrict this to one file. *Exact:* Semantic match per candidate after a text prefilter on the identifier.
  - Limits: On demand per position; no persistent cross-reference index.
  - Claim: **type-level inferred**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_ide`. Route: public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded

#### `references.local-resolution`

- **`pyrefly.name-resolution`** (pyrefly; unit: use) - For each name use in a module, the binding it forwards to (a definition, import, phi or narrow), and errors for names that are unknown or possibly unbound. *Exact:* Flow-sensitive: a use forwards to the phi/narrow reaching it.
  - Limits: Uses inside statically false branches are not bound. Session API is #[doc(hidden)] and changes between dev releases.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: equivalent (unconsumed) `ruff.references`, `ty.name-resolution`
- **`ruff.references`** (ruff; unit: use) - For each load of a name: the binding it resolves to, or why not (ImplicitGlobal, WildcardImport, UnboundLocal, NotFound); unresolved references with flags; a runtime-load simulation and symbol lookup for rules. *Exact:* Resolves to the latest visible binding in visit order; deferred function bodies see the scope's final bindings.
  - Limits: Linter-internal. Not a may-reach set: one binding per read.
  - Claim: **resolved**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic` (also CLI).
  - Overlaps: none recorded
- **`ty.name-resolution`** (ty; unit: use) - The definitions a name, attribute or imported symbol at a position resolves to (through scopes, imports and, for attributes, the inferred receiver type), optionally following import aliases to the original definition. *Exact:* Attribute resolution depends on inferred types; names follow use-def and the builtins fallback.
  - Limits: A `ResolvedDefinition` can be a module or a file range, not only a `Definition`.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: derived from consumed `ty.use-def`

#### `references.navigation`

- **`pyrefly.navigation`** (pyrefly; unit: position) - From a position: the definition(s), the declaration (following import chains less far) and the definition of the expression's type, each with module, range, docstring range and display name; the trace report records the first definition of every name and attribute. *Exact:* Follows bindings and types.
  - Limits: --report-trace keeps only the first definition per range.
  - Claim: **resolved**. Attach: PR-pos.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.expression-types`; equivalent (unconsumed) `ty.navigation`
- **`ty.implementations`** (ty; unit: definition) - For a method or class at a position, the overriding methods in subclasses and the subclasses themselves across the project. *Exact:* Scans project files for subclasses by inferred MRO.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source); also served by unpublished `ty_ide`. Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded
- **`ty.navigation`** (ty; unit: position) - From a position, the definition(s), declaration(s) or the definition of the inferred type's class, as navigation targets with full and focus ranges. *Exact:* Exact for resolvable names; attributes depend on inference.
  - Claim: **type-level inferred**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); also served by unpublished `ty_ide`. Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded
- **`ty.stub-mapping`** (ty; unit: definition) - For a definition found in a `.pyi` stub, the corresponding definition in the runtime `.py` module. *Exact:* By module and qualified path; overloads fall back to the implementation's docstring.
  - Claim: **resolved**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_module_resolver); public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: derived from consumed `ty.definitions`

#### `references.use-def`

- **`pyrefly.flow-merges`** (pyrefly; unit: use) - Where control flow merges, the set of binding keys that may reach a name (Phi, LoopPhi), each branch possibly narrowed; pruned branches contribute nothing. *Exact:* May-reach sets over the binding graph; no explicit path conditions.
  - Limits: No path-condition diagrams (unlike ty); only the narrowed type of each incoming branch.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: overlaps consumed `ty.use-def`; difference: ty keeps path-condition diagrams; pyrefly keeps only merged, narrowed branches.
- **`ty.declarations-at-binding`** (ty; unit: binding) - For a binding, the declarations of the same place that are visible there (and imported `Final` candidates), which decide the declared type the assignment is checked against. *Exact:* May-reach with reachability constraints.
  - Claim: **resolved**. Attach: TY-id.
  - Baseline: partial at 0.0.14: `UseDefMap::declarations_at_binding` present; `imported_final_candidates_at_binding`, `reachable_imported_final_candidates` and `ImportedFinalCandidate` exist only at 0.0.16. Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core).
  - Overlaps: derived from consumed `ty.definitions`

### runtime

#### `runtime.dynamic-attributes`

- **`pyrefly.dynamic-bases`** (pyrefly; unit: class) - Class bases that are computed at runtime (`unsupported-dynamic-base`) and attributes defined only outside the class body (`implicitly-defined-attribute`, `protocol-implicitly-defined-attribute`). *Exact:* Type-based.
  - Limits: getattr/setattr with computed names are not tracked.
  - Claim: **diagnostic-only**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process through the collected errors already read (filter by ErrorKind; preset-dependent); CLI JSON otherwise.
  - Overlaps: derived from consumed `pyrefly.class-fields`; equivalent (unconsumed) `ty.dynamic-attributes`
- **`ty.dynamic-attributes`** (ty; unit: expression) - Attribute access falls back to module-level and class `__getattr__` return types; `hasattr(x, "a")` narrows to an intersection with a synthesized protocol; module `__getattr__` calls are checked. *Exact:* Type-level only.
  - Limits: No tracking of setattr/getattr with computed names. Partial for this concept: `__getattr__` fallbacks and `hasattr` narrowing; computed names are not tracked.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: none recorded

#### `runtime.dynamic-imports`

- **`ruff.string-imports`** (ruff; unit: expression) - String literals with at least N dots that resolve to a module, added as import-graph edges. *Exact:* Any resolvable dotted string counts, whether or not it is ever imported.
  - Limits: No importlib/__import__ call analysis; literal-only.
  - Claim: **heuristic**. Attach: LEVEL (file edges in analyze graph).
  - Baseline: present at 0.0.14 (named items verified in source). Route: CLI only (`ruff` 0.16.8 = crates 0.0.14 line), or `ruff_linter` 0.16.8 in-process.
  - Overlaps: derived from consumed `ruff.ast`, `ty.module-resolution`
- **`ty.dynamic-imports`** (ty; unit: call) - `importlib.import_module("m")` and `__import__("m")` with a literal argument are inferred as the module `m`. *Exact:* Only literal strings.
  - Limits: Computed names give the declared return type. Partial for this concept: literal-string `import_module`/`__import__` only.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: derived from consumed `ty.module-resolution`

#### `runtime.exec-eval`

- **`ruff.dynamic-code-rules`** (ruff; unit: expression) - Uses of exec (S102) and eval (S307, PGH001-style), and getattr/setattr with a constant name (B009, B010) or attribute access via setattr patterns. *Exact:* Flags the call; nothing about the executed code or computed names.
  - Claim: **diagnostic-only**. Attach: RF-report.
  - Baseline: present at 0.0.14 (named items verified in source). Route: CLI only (`ruff` 0.16.8 = crates 0.0.14 line), or `ruff_linter` 0.16.8 in-process.
  - Overlaps: none recorded

#### `runtime.monkeypatching`

- **`pyrefly.patch-targets`** (pyrefly; unit: call) - `mock.patch`/`patch.object` target strings whose attribute does not exist (`missing-attribute-patch-target`). *Exact:* Resolves the dotted target string.
  - Limits: Only patch targets; no general monkeypatch tracking.
  - Claim: **diagnostic-only**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process through the collected errors already read (filter by ErrorKind; preset-dependent); CLI JSON otherwise.
  - Overlaps: derived from consumed `pyrefly.import-resolution`

### scopes

#### `scopes.bindings`

- **`pyrefly.binding-graph`** (pyrefly; unit: key) - Every binding key in a module (definitions, uses, imports, phis, loop phis, narrows, returns, yields, deletes, unpacks, captures, match subjects, exception classes) mapped to how its type is computed, plus per-concern tables for annotations, classes, fields, decorators, variance, exports and expectations. *Exact:* Statically false branches are never bound; 53 Binding variants at this pin.
  - Limits: Internal representation built for the solver; variants change between dev releases. Transaction::get_bindings was removed at this pin; go through get_answers(..).bindings().
  - Claim: **resolved**. Attach: PR-key.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: derived from consumed `pyrefly.ast`, `pyrefly.static-branches`; equivalent (unconsumed) `ruff.bindings`, `ty.live-bindings`; difference: pyrefly: flow-sensitive phi/narrow graph built for solving with pruned branches never bound; ruff: visit-order binding lists; ty: per-use reaching definitions.
- **`ruff.bindings`** (ruff; unit: binding) - Each binding of a name (21 BindingKinds: Assignment, Annotation, Argument, LoopVar, WithItemVar, Import, FromImport, ClassDefinition, FunctionDefinition, Deletion, BoundException, Global, Nonlocal, Builtin, Export...) with range, scope, execution context (Runtime/Typing), handled-exception context, flags (alias, explicit export, global, nonlocal, deleted, unpacked, in except handler, type alias, lazy), its ... *Exact:* Exact in visit order; function bodies are visited deferred, after the enclosing scope.
  - Limits: Linter-internal. No path conditions or may-reach sets; branch structure only through BranchId.
  - Claim: **resolved**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic`.
  - Overlaps: derived from consumed `ruff.ast`
- **`ty.live-bindings`** (ty; unit: binding) - For each place, the bindings and declarations that reach the end of its scope or are reachable anywhere in it, each with its reachability and narrowing constraint. *Exact:* May-reach, same semantics as `bindings_at_use`.
  - Limits: There is no ordered list of all bindings of a name with shadowing links (that is ruff's model).
  - Claim: **resolved**. Attach: TY-id (end-of-scope sets per place).
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core).
  - Overlaps: derived from consumed `ty.definitions`

#### `scopes.closures`

- **`pyrefly.captured-variables`** (pyrefly; unit: function) - For each nested function, the variables it reads or writes from an enclosing function (outer function and name), and per identifier the captured variables it refers to. *Exact:* Computed over the retained AST and bindings.
  - Limits: When a capture happens (definition time versus call time) is not stated.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process through the existing fork patch (changes 3/5 open `report`/`export`/`coverage::collect`).
  - Overlaps: equivalent (unconsumed) `ty.closure-snapshots`
- **`ty.closure-snapshots`** (ty; unit: binding) - For a name a nested scope reads from an enclosing one, the enclosing bindings visible to it: a snapshot at definition time for eager scopes (comprehensions, class bodies), all reachable end bindings for lazy scopes, and synthetic NestedBindings definitions for writes from nested scopes. *Exact:* Exact over the index; lazy capture is a may-set.
  - Claim: **resolved**. Attach: TY-id.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core).
  - Overlaps: derived from consumed `ty.use-def`

#### `scopes.definitions`

- **`pyrefly.definition-at-position`** (pyrefly; unit: position) - Which definition key is at a position and which definitions are visible at a position in a module. *Exact:* Flow-insensitive visibility from the binding table.
  - Claim: **resolved**. Attach: PR-pos.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: none recorded
- **`pyrefly.glean-declarations`** (pyrefly; unit: definition) - For every module, class, function, variable (locals included) and import: its declaration span, its definition span with container, bases, decorators (as text), parameters and annotation text, and the top-level declaration that contains it, keyed by qualified name. *Exact:* Byte spans; one run covers every checked module.
  - Limits: Built from an AST walk: declarations inside statically false branches are emitted (both `z = 1` under a pruned version check and `z = "s"` in its else), although pyrefly never binds the pruned one. Fact order varies between runs; compare as sets.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process through the existing fork patch (changes 3/5 open `report`/`export`/`coverage::collect`).
  - Overlaps: overlaps consumed `ty.definitions`; derived from consumed `pyrefly.ast`; difference: Only pyrefly emits project-wide declarations at once (Glean).
- **`pyrefly.global-variables`** (pyrefly; unit: variable) - Each module-level variable with its type and location, as Pysa global-variable definitions. *Exact:* Declared type when annotated, else inferred.
  - Limits: A typed table of module globals, not a points-to fact: no allocation sites or aliasing.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process through the existing fork patch (changes 3/5 open `report`/`export`/`coverage::collect`).
  - Overlaps: derived from consumed `pyrefly.expression-types`
- **`ruff.definitions`** (ruff; unit: definition) - The module, classes, nested classes, functions, nested functions and methods of a module as Definitions, each with public/private visibility (underscore and `__all__` rules). *Exact:* Exact enumeration; visibility by naming convention and `__all__`.
  - Limits: Linter-internal; the docstring text itself is extracted by a private ruff_linter module.
  - Claim: **exact syntactic**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic`.
  - Overlaps: overlaps consumed `pyrefly.definitions`; derived from consumed `ruff.ast`
- **`ty.unpack-targets`** (ty; unit: definition) - For an assignment, `for` or `with` target that destructures, the unpack (iterable, context manager or assignment) each element binding comes from. *Exact:* Exact structure; element types come from inference (internal `Unpacker`).
  - Claim: **exact syntactic**. Attach: TY-id.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core).
  - Overlaps: derived from consumed `ty.definitions`

#### `scopes.execution-context`

- **`pyrefly.type-checking-context`** (pyrefly; unit: region) - Whether code sits under `if TYPE_CHECKING:` (or pyrefly's `TYPE_CHECKING_WITH_PYREFLY`), whether a function is defined inside such a block, whether `from __future__ import annotations` (or a stub) defers annotations, and whether a module allows top-level await (notebooks). *Exact:* TYPE_CHECKING is decided true for analysis; the runtime branch of `if not TYPE_CHECKING` is pruned.
  - Limits: No per-annotation runtime/typing-only classification as data; future-annotations state is internal to the bindings builder.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: derived from consumed `pyrefly.static-branches`
- **`ruff.semantic-context`** (ruff; unit: node) - Whether a node is in a TYPE_CHECKING block, a typing-only or runtime-evaluated annotation, a runtime-required annotation (e.g. pydantic/dataclass contexts per settings), a string type definition, under `from __future__ import annotations`, in a stub, a boolean test, an exception handler, an f-string, `__all__`, an async context or `@no_type_check`. *Exact:* Syntactic classification combined with configuration (`lint.flake8-type-checking.runtime-evaluated-*`).
  - Limits: Linter-internal.
  - Claim: **resolved**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic` (also CLI).
  - Overlaps: derived from consumed `ruff.ast`

#### `scopes.global-nonlocal`

- **`pyrefly.mutable-captures`** (pyrefly; unit: name) - Names a scope redirects with `global` or `nonlocal`, as `MutableCapture` definitions and binding keys, with unknown-name errors when nothing is captured. *Exact:* Exact per statement.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: equivalent (unconsumed) `ruff.global-nonlocal`, `ty.global-nonlocal`
- **`ruff.global-nonlocal`** (ruff; unit: scope) - The names each function scope declares `global`, with the declaration ranges, and the Global/Nonlocal bindings and flags that redirect later writes. *Exact:* Exact.
  - Limits: Linter-internal.
  - Claim: **resolved**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic`.
  - Overlaps: derived from consumed `ruff.ast`
- **`ty.global-nonlocal`** (ty; unit: binding) - Whether a name is declared `global` or `nonlocal` in a scope and whether a symbol resolves to the global scope; `global` names with no module binding are flagged. *Exact:* Exact.
  - Claim: **resolved**. Attach: TY-id.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core).
  - Overlaps: derived from consumed `ty.places`

#### `scopes.qualified-names`

- **`pyrefly.qualified-names`** (pyrefly; unit: name) - The fully qualified name of every class, function and variable (`pkg.a.f.<locals>.v`), and for each reference the qualified name it targets; in-process `QName` formats a class's module-qualified name. *Exact:* Resolved through bindings, not by syntactic import tracking.
  - Limits: Locals are named with `<locals>`; unresolvable references are omitted from Glean.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: equivalent (unconsumed) `ruff.qualified-names`; difference: ruff resolves an expression's import path syntactically; pyrefly names definitions and reference targets after type resolution.
- **`ruff.qualified-names`** (ruff; unit: expression) - The fully qualified import path of a name or attribute chain (`contextlib.suppress`, `builtins.open`) by following import bindings in the module; typing/builtins matching; which well-known modules a module has seen. *Exact:* Follows import bindings syntactically within one module.
  - Limits: No file resolution, no types: a callee assigned from a function call or attribute of an instance does not resolve. Linter-internal.
  - Claim: **resolved**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`, `ruff_python_semantic`.
  - Overlaps: none recorded

#### `scopes.scopes`

- **`pyrefly.enclosing-scope`** (pyrefly; unit: scope) - For a range: the enclosing class, function definition ranges and the nesting context (toplevel, class, function) with its owner path, as used for qualified names. *Exact:* From binding scopes.
  - Limits: No public scope-tree object; scope kinds for comprehensions and lambdas are not exposed.
  - Claim: **exact syntactic**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: none recorded
- **`ruff.scopes`** (ruff; unit: scope) - The scopes of a module (Module, Class, Function, Lambda, Generator for comprehensions, Type for PEP 695 parameters, DunderClassCell) with parent links and the bindings each holds; whether a scope uses `locals()`. *Exact:* Exact scoping per Python rules, including class-scope skipping.
  - Limits: Linter-internal: populated only inside ruff_linter's Checker.
  - Claim: **exact syntactic**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic`.
  - Overlaps: derived from consumed `ruff.ast`
- **`ty.scopes`** (ty; unit: scope) - Each scope of a file with its kind (Module, TypeParams, Class, Function, Lambda, Comprehension, TypeAlias), node, parent, laziness (eager or lazy), and lookups from expressions and nodes to scopes, ancestors and visible ancestors (class scopes skipped). *Exact:* Exact.
  - Claim: **resolved**. Attach: TY-id (scopes map to ranged nodes).
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core); public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: derived from consumed `ty.parsed-module`

#### `scopes.unused`

- **`pyrefly.unused-bindings`** (pyrefly; unit: binding) - Imports, local variables and parameters that are never read, exposed by the bindings and shown as LSP 'unnecessary' hints. *Exact:* Flow-insensitive reads within the module.
  - Limits: No error kind reports them; only the LSP and in-process accessors.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: equivalent (unconsumed) `ruff.unused-bindings`, `ty.unused-bindings`; difference: pyrefly has no error kind for them; only LSP hints and accessors.
- **`ruff.unused-bindings`** (ruff; unit: binding) - Bindings with no references: unused imports (F401), local variables (F841), unpacked variables (RUF059), arguments (ARG001-005), loop control variables (B007), and redefinitions of unused names (F811). *Exact:* Per module; `__all__`, `locals()` use and dummy-variable regex suppress findings.
  - Limits: Cross-module use is not considered: a first-party import used only by importers via re-export is unused unless marked.
  - Claim: **resolved**. Attach: RF-checker (or RF-report via F401/F841).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic` (also CLI).
  - Overlaps: none recorded
- **`ty.unused-bindings`** (ty; unit: binding) - Local bindings never read in function, lambda and comprehension scopes (assignments, walrus, for/with/except/match targets, parameters), excluding overload stubs and names declared global/nonlocal; served as editor hints, not lint diagnostics. *Exact:* Use-def based: a binding is used if any use may reach it.
  - Limits: Module and class scopes and imports are not reported; no CLI output.
  - Claim: **resolved**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source); also served by unpublished `ty_ide`. Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: derived from consumed `ty.use-def`

### source

#### `source.files`

- **`pyrefly.module-source`** (pyrefly; unit: module) - For every module pyrefly loads: the exact text it analysed, its path (file system, namespace, in-memory or bundled), the source type (.py, .pyi, notebook), the line count, and whether the file carries the `@generated` marker. *Exact:* Exact: the parse step parses exactly `contents()`; byte ranges everywhere index this text.
  - Limits: Bundled typeshed modules report synthetic paths such as `bundled /crates/pyrefly_bundled/third_party/typeshed/stdlib/...`. `is_generated` is a substring test for `@generated` anywhere in the file, not a header check. gencode.GenCode.1 emitted no facts on a fixture without generated files; its ...
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: equivalent (unconsumed) `ruff.source-kind`, `ty.files`
- **`ruff.source-kind`** (ruff; unit: file) - Whether a path is a Python module, a stub, a Jupyter notebook or Markdown (from extension and the `extension` mapping), and the UTF-8 text ruff analyses for it (notebook code cells concatenated; Markdown fences extracted for formatting). *Exact:* Exact for UTF-8 input; the BOM is kept in the text and skipped by the lexer only at offset 0.
  - Limits: Only UTF-8 is accepted; there is no PEP 263 encoding-declaration decoding. `.pyw` is treated as Python although it is not importable. Markdown is a formatter input only; the linter does not analyse code blocks in Markdown.
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_db`, `ruff_linter`, `ruff_python_ast`, `ruff_source_file`.
  - Overlaps: none recorded
- **`ty.files`** (ty; unit: file) - Each module is a salsa `File` (system path, vendored typeshed path or editor virtual file) with existence, stub/package flags and a `PySourceType`; `source_text` gives its decoded text (notebook-aware) or the read error. *Exact:* Exact over what the `System` returns; revisions are tracked per file (`file_revision`) for incrementality.
  - Limits: `OsSystem` needs ruff_db feature `os`; tests use `TestSystem`/`MemoryFileSystem`. Vendored files are read from the zipped typeshed in ty_vendored.
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ruff_db); public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: none recorded

#### `source.notebooks`

- **`pyrefly.notebook-cells`** (pyrefly; unit: cell) - For an .ipynb module: its code cells concatenated into one buffer, each cell's index, URL and contents, and per-cell positions for diagnostics and LSP answers. *Exact:* Cell boundaries come from ruff_notebook's cell offsets.
  - Limits: Cell numbering differs between surfaces (JSON `cell` versus LSP cell URIs). How cell offsets interact with byte conversions was not examined by the skill.
  - Claim: **exact syntactic**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: equivalent (unconsumed) `ty.notebooks`
- **`ruff.notebook-index`** (ruff; unit: cell) - The code cells of a `.ipynb` concatenated into one source (each followed by a synthetic newline), the byte offset where each code cell starts, and a mapping from concatenated rows to (cell, row-in-cell). *Exact:* Exact for rows; columns are passed through untranslated.
  - Limits: Two numberings: raw-cell (1-based, all cells) in CLI output vs code-cell index (0-based) in CellOffsets. Cells starting with a non-Python cell magic (e.g. `%%markdown`) are excluded from the concatenated source; since 0.0.16 `Notebook` equality also compares the concatenated source, offsets and ...
  - Claim: **exact syntactic**. Attach: LEVEL (file: cell offsets).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_notebook`, `ruff_python_parser` (also CLI).
  - Overlaps: none recorded
- **`ty.notebooks`** (ty; unit: file) - A `.ipynb` file's concatenated cell source with cell offsets; parsed per cell; the server syncs notebook documents and maps positions back to cells. *Exact:* Cell offsets from ruff_notebook; diagnostics on the CLI are rendered with cell numbers by ruff_db's renderer.
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ruff_db).
  - Overlaps: none recorded

#### `source.positions`

- **`pyrefly.glean-file-lines`** (pyrefly; unit: module) - For each file in the Glean report, the length of every line, whether the file ends in a newline and whether it contains non-ASCII or tab characters, so byte spans can be turned into lines. *Exact:* Exact for LF files.
  - Limits: Wrong for CRLF files (the skill's measured trap); compute lines from the bytes instead.
  - Claim: **exact syntactic**. Attach: LEVEL (file: line lengths for span conversion).
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: none recorded
- **`pyrefly.line-index`** (pyrefly; unit: range) - Converts byte `TextRange`s of a module to and from display positions (1-based lines, Unicode code point columns, BOM skipped), LSP positions (UTF-16), Pysa locations (1-based, UTF-8 byte columns) and `PythonASTRange` (0-based columns), including notebook-cell-local positions. *Exact:* Exact and invertible for byte <-> UTF-8 locations; display conversions skip a BOM on line 1 and are not the inverse of byte offsets there.
  - Limits: Every output picks its own convention: check JSON and SARIF use code point columns (1-based); Pysa uses 1-based UTF-8 byte columns; Query and cinderx use 0-based columns; LSP is UTF-16 only; coverage reports start positions only; Glean uses byte spans. Wraps ruff_source_file 0.0.14's LineIndex.
  - Claim: **exact syntactic**. Attach: TEXT (position conversion over the parse-of-record text).
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: none recorded
- **`ruff.line-index`** (ruff; unit: position) - Line starts of a text and the conversions between byte offsets and (line, column) in UTF-8 bytes, UTF-16 units or Unicode scalar values, all 1-based. *Exact:* `source_location`/`offset` are an exact inverse pair for every encoding; `line_column` skips the BOM and is lossy on line 1.
  - Limits: CRLF, LF and a lone CR all end a line; `offset` clamps an over-long column onto the line terminator. Rendered CLI columns are characters, not bytes: never use them as byte offsets.
  - Claim: **exact syntactic**. Attach: TEXT (position conversion).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_source_file`, `ruff_text_size`, `ruff_wasm` (also CLI).
  - Overlaps: none recorded
- **`ty.line-index`** (ty; unit: file) - Line starts of a file's text, for converting byte offsets (`TextSize`) to one-based line/column and back. *Exact:* Same `ruff_source_file::LineIndex` as ruff; columns can be computed in UTF-8, UTF-16 or UTF-32 (`PositionEncoding`).
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ruff_db); public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: none recorded
- **`ty.output-positions`** (ty; unit: range) - CLI and report formats give one-based line and one-based character columns (a line with `"ééé"` puts column 16 where the byte column is 19); the language server negotiates the column unit, preferring UTF-8, then UTF-32, defaulting to UTF-16. *Exact:* Executed for the CLI; LSP selection read from source.
  - Limits: `github` output uses absolute paths; `gitlab` paths are relative to the project root.
  - Claim: **exact syntactic**. Attach: LEVEL (position convention).
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_server`. Route: public in unpublished `ty_server` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded

#### `source.script-metadata`

- **`ruff.script-metadata`** (ruff; unit: file) - Whether a file carries a `# /// script` block, its de-commented TOML text, and a source map from the TOML back to file ranges. *Exact:* Exact extraction of the block text.
  - Limits: Returns TOML text only; requirements and requires-python are not parsed here (ty's project layer and ruff_ranged_value consume it).
  - Claim: **exact syntactic**. Attach: LEVEL (file: TOML block with a source map to byte ranges).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`.
  - Overlaps: none recorded
- **`ty.script-metadata`** (ty; unit: file) - A file's `# /// script` block (dependencies, requires-python, `[tool.ty]`), which makes it a separate program with its own Python version (source: script metadata), settings and uv-provided environment; invalid blocks are diagnostics. *Exact:* Uses ruff_python_ast's script-tag parser; environment from uv when available.
  - Claim: **exact syntactic**. Attach: LEVEL.
  - Baseline: present at 0.0.14 (named items verified in source); also served by unpublished `ty_project`. Route: public in crates `cpg-flow` already links or their published siblings (ty_site_packages); public in unpublished `ty_project` (git dependency on the ruff 0.16.8 tag); richer parts are internal.
  - Overlaps: none recorded

### syntax

#### `syntax.ast`

- **`ruff.ast-predicates`** (ruff; unit: node) - Predicates over nodes: docstring statement, stub body (`...`/pass), on a conditional branch, in a nested block, dunder/sunder names, constants, unpacking assignment, dotted name, magic-variable access, relative-import resolution to a module path. *Exact:* Exact as syntactic predicates; nothing is resolved.
  - Claim: **exact syntactic**. Attach: PR-ast.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`.
  - Overlaps: derived from consumed `ruff.ast`
- **`ruff.node-lookup`** (ruff; unit: node) - The innermost node covering a range, the suite and following sibling of a statement, the if/elif/else branches of an if, and the range of the identifier of a def/class/parameter/alias. *Exact:* Exact.
  - Claim: **exact syntactic**. Attach: PR-ast.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`.
  - Overlaps: derived from consumed `ruff.ast`
- **`ruff.operator-precedence`** (ruff; unit: node) - The precedence class of an expression or operator, used to decide whether a rewrite needs parentheses. *Exact:* Exact per Python grammar.
  - Claim: **exact syntactic**. Attach: PR-ast.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`.
  - Overlaps: derived from consumed `ruff.ast`
- **`ruff.parenthesized-range`** (ruff; unit: node) - The range of an expression widened to its enclosing parentheses, when it is parenthesized. *Exact:* Exact given tokens or comment ranges.
  - Claim: **exact syntactic**. Attach: PR-ast (needs comment ranges from an RF-lex).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`.
  - Overlaps: derived from consumed `ruff.ast`

#### `syntax.codegen`

- **`ruff.codegen`** (ruff; unit: node) - Python source for a node or tree, rendered in the module's detected style (used to build fix edits). *Exact:* Round-trips semantics, not layout; comments are not preserved.
  - Limits: A transformation rather than a fact about the program; listed because fixes are built from it. A concept for unparse/codegen is missing from the taxonomy.
  - Claim: **exact syntactic**. Attach: PR-ast (renders text; not a fact).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_codegen`.
  - Overlaps: derived from consumed `ruff.ast`

#### `syntax.literal-internals`

- **`pyrefly.format-string-callees`** (pyrefly; unit: expression) - For each f-string, the artificial `__format__`/`__str__`/`__repr__` callees of its interpolations, as Pysa call-graph entries keyed `FormatStringArtificial`/`FormatStringStringify`. *Exact:* Callees resolved through the interpolated value's type.
  - Limits: Format specifications inside the f-string are not parsed into data.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process through the existing fork patch (changes 3/5 open `report`/`export`/`coverage::collect`).
  - Overlaps: derived from consumed `pyrefly.ast`, `pyrefly.expression-types`
- **`pyrefly.regex-validation`** (pyrefly; unit: expression) - Structural errors in regular-expression literals passed to `re` functions (the subset of Python regex syntax pyrefly understands), reported as kind `regex`. *Exact:* Validates a subset; unknown extensions are left to runtime to avoid false positives.
  - Limits: No data surface; the parsed pattern is not exposed.
  - Claim: **diagnostic-only**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process through the collected errors already read (filter by ErrorKind; preset-dependent); CLI JSON otherwise.
  - Overlaps: derived from consumed `pyrefly.ast`
- **`ruff.literal-internals`** (ruff; unit: node) - The parsed structure of `str.format` strings and format specs, `%`-format strings, escape sequences, float text and string/bytes prefixes inside literals. *Exact:* Exact parse of literal content.
  - Claim: **exact syntactic**. Attach: PR-ast.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`, `ruff_python_literal`.
  - Overlaps: derived from consumed `ruff.ast`

#### `syntax.parse-errors`

- **`ruff.parse-errors`** (ruff; unit: range) - Where the grammar failed (42 ParseErrorType and 13 LexicalErrorType kinds) with a range; the recovered tree is still returned. *Exact:* Exact ranges; error recovery may produce several cascaded errors for one mistake.
  - Claim: **exact syntactic**. Attach: RF-reparse (pyrefly already collects the same errors on its parse).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_parser` (also CLI).
  - Overlaps: none recorded

#### `syntax.semantic-syntax`

- **`pyrefly.semantic-syntax-errors`** (pyrefly; unit: range) - Errors CPython's compiler raises beyond the grammar (for example `return` outside a function, late `__future__`, misplaced `await` or `yield`), found by ruff's SemanticSyntaxChecker run during binding plus pyrefly's own binding-time checks, reported as `invalid-syntax`. *Exact:* ruff's checker semantics plus pyrefly checks at binding sites (record_return, comprehensions, stmt_impl).
  - Limits: Statically pruned branches are not bound, so errors inside them are not reported.
  - Claim: **exact syntactic**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: derived from consumed `pyrefly.ast`; equivalent (unconsumed) `ruff.semantic-syntax-errors`, `ty.semantic-syntax-errors`; difference: All run ruff's SemanticSyntaxChecker (pyrefly at 0.0.14); pyrefly reports them as invalid-syntax and skips pruned branches.
- **`ruff.semantic-syntax-errors`** (ruff; unit: range) - 41 SemanticSyntaxErrorKinds CPython raises at compile time: late `__future__` import, await/yield/return outside a function, rebound comprehension variable, duplicate type parameter or parameter, irrefutable case pattern, load before global/nonlocal, break/continue outside loop, lazy-import misuse, return in generator... *Exact:* Exact for the checks implemented; the host supplies scope facts through SemanticSyntaxContext.
  - Limits: Needs a host visitor implementing SemanticSyntaxContext (ruff's Checker or ty's index builder); not a free function over the AST.
  - Claim: **exact syntactic**. Attach: RF-checker (needs a `SemanticSyntaxContext` host).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_parser` (also CLI).
  - Overlaps: derived from consumed `ruff.ast`
- **`ty.semantic-syntax-errors`** (ty; unit: range) - Compiler-level errors beyond the grammar (`await` outside async, late `__future__`, duplicate parameters, `nonlocal` at module level...), collected while building the semantic index, which supplies the scope context ruff's `SemanticSyntaxChecker` needs. *Exact:* Same checker as ruff; ty answers its context queries (scopes, async, global declarations) from its own index.
  - Claim: **exact syntactic**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core).
  - Overlaps: none recorded

#### `syntax.string-annotations`

- **`pyrefly.string-annotations`** (pyrefly; unit: annotation) - The expression inside an annotation written as a string literal, parsed and then bound and typed like any annotation; Glean also emits cross-references inside such strings. *Exact:* Simple strings map exactly to source ranges; complex strings (escapes, implicit concatenation) are re-parsed at the literal's start offset with no exact mapping.
  - Limits: Ranges inside complex string annotations are synthetic.
  - Claim: **exact syntactic**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: derived from consumed `pyrefly.ast`; equivalent (unconsumed) `ty.string-annotations`
- **`ruff.string-annotations`** (ruff; unit: node) - The expression tree inside a string-literal annotation, and whether it is Simple (ranges map to the file) or Complex (implicit concatenation/escapes: ranges relative to the string). *Exact:* Exact for Simple; Complex ranges are synthetic.
  - Claim: **exact syntactic**. Attach: PR-ast (Simple: exact file ranges; Complex: string-relative).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_parser`.
  - Overlaps: derived from consumed `ruff.ast`
- **`ty.string-annotations`** (ty; unit: node) - The sub-AST of a quoted annotation, indexed under the string literal's node index; ty infers it in a deferred string-annotation context and lints raw, implicitly concatenated, escaped or unparsable forward annotations. *Exact:* Exact parse; nesting depth and index exhaustion produce a `StringAnnotationError` instead of a tree.
  - Limits: Sub-indices are limited: very long or deeply nested quoted annotations fail with an error message rather than a tree.
  - Claim: **exact syntactic**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ruff_db); public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: derived from consumed `ty.parsed-module`

#### `syntax.structural-equality`

- **`ruff.structural-equality`** (ruff; unit: node) - Whether two nodes are structurally equal ignoring ranges and node indices, with a hash consistent with that equality. *Exact:* Exact structural equality (string literal values compared by value).
  - Claim: **exact syntactic**. Attach: PR-ast.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_ast`.
  - Overlaps: derived from consumed `ruff.ast`

#### `syntax.version-syntax`

- **`pyrefly.unsupported-syntax`** (pyrefly; unit: range) - Syntax that parses but is not available in the module's configured Python version, reported as `invalid-syntax` from ruff's unsupported-syntax errors. *Exact:* Exactly ruff's UnsupportedSyntaxError set for the target version.
  - Limits: Shares the kind name invalid-syntax with semantic syntax errors and binding-time syntax checks; distinguish by message.
  - Claim: **exact syntactic**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: equivalent (unconsumed) `ruff.unsupported-syntax-errors`, `ty.version-syntax`; difference: pyrefly's target version is per handle from its config; ruff 0.0.14 versus 0.0.16 checker.
- **`ruff.unsupported-syntax-errors`** (ruff; unit: range) - Syntax that parses but needs a newer Python than the target version (21 kinds: match, walrus, except*, PEP 695 type params and aliases, PEP 701 f-strings, t-strings, parenthesized context managers, lazy import statement...), with range and target version. *Exact:* Exact per kind; relative to ParseOptions::with_target_version.
  - Claim: **exact syntactic**. Attach: RF-reparse (pyrefly reports the same set as invalid-syntax).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_parser` (also CLI).
  - Overlaps: derived from consumed `ruff.ast`
- **`ty.version-syntax`** (ty; unit: range) - Syntax that parses but is unavailable in the program's Python version (for example `match` before 3.10), reported as invalid-syntax; the version comes from the `PythonFile` key, so the same file can be parsed for two versions. *Exact:* Exact against the configured version; the version's provenance is attached as a sub-diagnostic (`inferred_python_version_source_annotation`).
  - Claim: **exact syntactic**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ruff_db); public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: derived from consumed `ty.parsed-module`

### types

#### `types.annotation-inference`

- **`pyrefly.inferred-annotations`** (pyrefly; unit: function) - For each function and variable, the annotation pyrefly can infer (returns, variables, optionally containers and parameters) and the imports needed, as edits or as data. *Exact:* Only types pyrefly can name; a return depending on an unannotated parameter stays unannotated.
  - Limits: Parameters are not annotated by default. --dry-run prints only counts per file.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.expression-types`; equivalent (unconsumed) `ty.annotation-edits`
- **`ty.annotation-edits`** (ty; unit: binding) - For an unannotated variable, the inferred type shown inline and the text edit that inserts it as an annotation. *Exact:* Only when the type is spellable; edits may add imports.
  - Limits: Variables only; no return-type or parameter annotation inference.
  - Claim: **type-level inferred**. Attach: TY-pos.
  - Baseline: present at 0.0.14 (named items verified in source); needs unpublished `ty_ide`. Route: public in unpublished `ty_ide` (git dependency on the ruff 0.16.8 tag).
  - Overlaps: none recorded

#### `types.coverage`

- **`pyrefly.type-coverage`** (pyrefly; unit: symbol) - Per module and per symbol (function, attribute, class, property): how many typable slots are typed, Any or untyped, coverage and strict coverage percentages, counts of functions/methods/params/classes/attrs/properties and type ignores, with a project summary; optionally only public symbols. *Exact:* Counts from pyrefly's inferred/declared types per slot.
  - Limits: Per-parameter ranks (SlotRank) are computed but not serialised; JSON gives only counts. Locations are start-only (1-based line/column). `pyrefly report` is a deprecated alias.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: needs a fork-patch extension (visibility of `pyrefly::commands`); otherwise CLI only.
  - Overlaps: derived from consumed `pyrefly.declared-types`, `pyrefly.expression-types`, `pyrefly.public-fqns`; equivalent (unconsumed) `ruff.missing-annotations`; difference: ruff flags missing annotations syntactically; pyrefly counts typed/Any/untyped slots from types.
- **`ruff.missing-annotations`** (ruff; unit: definition) - Parameters and returns without annotations (ANN001-206) and explicit `Any` (ANN401); fixes add `-> None` or a simple inferred return type for some functions. *Exact:* Exact presence checks; the return-type fix uses syntactic return-value kinds.
  - Limits: No coverage summary; counts only via --statistics.
  - Claim: **diagnostic-only**. Attach: RF-report.
  - Baseline: present at 0.0.14 (named items verified in source). Route: CLI only (`ruff` 0.16.8 = crates 0.0.14 line), or `ruff_linter` 0.16.8 in-process.
  - Overlaps: derived from consumed `ruff.ast`

#### `types.declared-types`

- **`pyrefly.type-aliases`** (pyrefly; unit: definition) - Each type alias (`X: TypeAlias = ...`, PEP 695 `type X = ...`, implicit aliases) with its resolved value, and invalid-type-alias diagnostics. *Exact:* Values are solved types.
  - Claim: **resolved**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch.
  - Overlaps: none recorded
- **`ty.declared-types`** (ty; unit: declaration) - The type a declaration states for a place (annotations, `def`/`class` statements), against which bindings are checked; conflicting or invalid declarations are diagnostics. *Exact:* Annotation types are evaluated as type expressions; no public query by place.
  - Limits: Public route: the inferred type of the annotation expression, or hover.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in crates `cpg-flow` already links or their published siblings (ty_python_core); richer parts are internal.
  - Overlaps: overlaps consumed `pyrefly.declared-types`
- **`ty.type-qualifiers`** (ty; unit: declaration) - Which qualifiers an annotation carries (Final, ClassVar, InitVar, Required, NotRequired, ReadOnly) and whether an annotation or definition is a type alias. *Exact:* Exact for recognised qualifier forms.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: none recorded

#### `types.expression-types`

- **`pyrefly.expected-types`** (pyrefly; unit: expression) - The type the context expects at an expression (assignment target, parameter, return annotation), separately from its inferred type. *Exact:* Only where the solver recorded a hint.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch (doc-hidden session API).
  - Overlaps: derived from consumed `pyrefly.expression-types`
- **`ty.expression-types`** (ty; unit: expression) - The inferred type of any expression node (all `ast::Expr` kinds), of a function, class, parameter, type parameter or type alias definition, of an import alias and of an except-handler binding. *Exact:* Gradual: unknowns are `Unknown`, explicit `Any` is `Any`; unannotated function returns are `Unknown`.
  - Limits: Type ids are salsa-interned; render with `Type::display`.
  - Claim: **type-level inferred**. Attach: TY-range.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: overlaps consumed `pyrefly.expression-types`; derived from consumed `ty.use-def`, `ty.narrowing-constraints`, `ty.module-resolution`, `ty.parsed-module`

#### `types.generics`

- **`ty.generics`** (ty; unit: definition) - Generic contexts of classes, functions and aliases (PEP 695 and legacy TypeVar/ParamSpec/TypeVarTuple), bounds, constraints and defaults, specializations, and inferred variance of PEP 695 type parameters. *Exact:* Variance inference follows the typing spec algorithm.
  - Claim: **type-level inferred**. Attach: TY-id.
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: overlaps consumed `pyrefly.generics`

#### `types.relations`

- **`pyrefly.subtype-query`** (pyrefly; unit: pair) - Whether one type expression (as a string, resolved in a module) is a subtype of another. *Exact:* pyrefly's assignability rules, affected by strict-callable-subtyping and strict-partial-subtyping.
  - Limits: Type expressions are parsed from strings; malformed input errors.
  - Claim: **type-level inferred**. Attach: PR-range.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: in-process, no further patch; parts behind private modules: need a patch extension.
  - Overlaps: derived from consumed `pyrefly.type-terms`; equivalent (unconsumed) `ty.type-relations`
- **`ty.type-relations`** (ty; unit: type) - Whether one type is assignable to, a subtype of, equivalent to or disjoint from another, solved with constraint sets over type variables; failures are explained as error-context trees in diagnostics. *Exact:* Gradual-typing semantics (materialization of Any/Unknown).
  - Limits: Only assignability is public.
  - Claim: **type-level inferred**. Attach: LEVEL (pair of types).
  - Baseline: present at 0.0.14 (named items verified in source). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: none recorded

#### `types.stubs`

- **`pyrefly.stub-generation`** (pyrefly; unit: module) - A .pyi per module with imports, classes (with synthesized dataclass __init__), functions and variables carrying declared or inferred types; unresolvable types become `Incomplete`; optionally private names and docstrings. *Exact:* Types as pyrefly displays them.
  - Limits: Output paths were flattened relative to the include root on the fixture (pkg/a.py -> a.pyi). Unused imports such as TYPE_CHECKING are kept.
  - Claim: **type-level inferred**. Attach: LEVEL.
  - Baseline: pyrefly 1.4.0-dev.3: yes (indexed pin). Route: needs a fork-patch extension (visibility of `pyrefly::stubgen`); otherwise CLI only.
  - Overlaps: derived from consumed `pyrefly.expression-types`, `pyrefly.declared-types`

#### `types.type-terms`

- **`ruff.typing-constructs`** (ruff; unit: node) - Which typing construct an expression names: Annotated/Literal/Optional/Union subscripts, PEP 585 generic equivalents, PEP 604 operators, and calls to cast, NewType, TypeVar, ParamSpec, TypeVarTuple, NamedTuple, TypedDict. *Exact:* Classification by qualified name; nothing is evaluated.
  - Claim: **resolved**. Attach: RF-checker.
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic`.
  - Overlaps: overlaps consumed `pyrefly.type-terms`
- **`ty.type-algebra`** (ty; unit: type) - Types as 36 variants: dynamic (Any, Unknown and its sub-kinds, Todo), Divergent, Recursive/RecursiveVar, Never, function and bound-method literals, Callable, module and class literals, generic aliases, `type[...]`, nominal and protocol instances, special forms and known instances, properties, slot descriptors, Union, Intersection (with negations), enum complements, AlwaysTruthy/AlwaysFalsy, literal values, type ... *Exact:* Exact representation; most constructors and accessors are pub(crate).
  - Limits: Matching a variant is public; walking inside many payloads (ClassLiteral, CallableType) is not.
  - Claim: **type-level inferred**. Attach: LEVEL (type: interned `Type`; attach through the typed node).
  - Baseline: partial at 0.0.14: `RecursiveType`/`RecursiveVar` exist only at 0.0.16, so the variant set differs (the 36-variant count is 0.0.16). Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph); richer parts are internal.
  - Overlaps: overlaps consumed `pyrefly.type-terms`
- **`ty.type-display`** (ty; unit: type) - The display string of a type (as in hover and diagnostics), with settings for qualification and multi-line; per-part details for navigable labels. *Exact:* Deterministic for a type; not a stable serialization.
  - Claim: **type-level inferred**. Attach: LEVEL (type: display strings).
  - Baseline: partial at 0.0.14: `Type::display_alias_declaration` exists only at 0.0.16. Route: public in `ty_python_semantic` 0.0.14 (published on crates.io, not linked today; brings type inference into the graph).
  - Overlaps: none recorded

#### `types.value-kinds`

- **`ruff.value-kinds`** (ruff; unit: expression) - Whether an expression or binding is a string, bytes, number (int/float/complex/bool), None, ellipsis, dict, list, set, tuple or generator; whether a name is bound to a list/dict/set/pathlib.Path etc. from its binding value or annotation; whether an expression is mutable. *Exact:* Literal shapes, binding values and annotations; not type inference.
  - Limits: Unknown for any call result other than recognised builtins.
  - Claim: **heuristic**. Attach: PR-ast (`ResolvedPythonType::from(&Expr)`) + RF-checker (binding-based `is_list`/`is_dict`).
  - Baseline: present at 0.0.14 (named items verified in source). Route: crates `ruff_python_semantic`.
  - Overlaps: derived from consumed `ruff.ast`

## Baseline differences found (0.0.14 against the indexed 0.0.16)

**Only at ruff 0.0.16 (absent at the baseline):** `ruff.name-prefilter` (`ruff_python_trivia::NameMatcher` does not exist in ruff_python_trivia 0.0.14).

**ty facts partly at 0.0.16 only:**
- `ty.declarations-at-binding`: `UseDefMap::declarations_at_binding` is at 0.0.14. The imported-`Final` candidate queries (`imported_final_candidates_at_binding`, `reachable_imported_final_candidates`, `ImportedFinalCandidate`) are 0.0.16 only.
- `ty.type-algebra`: the `RecursiveType` and `RecursiveVar` types are 0.0.16 only, so the 36-variant enumeration is a 0.0.16 count.
- `ty.type-display`: `Type::display_alias_declaration` is 0.0.16 only.
- `ty.uv-environments`: `UvWorkspace` is 0.0.16 only.
- `ty.lint-registry` / `ty.type-check-diagnostics`: 134 lints at 0.0.14 against 138. The four added at 0.0.16 are `invalid-init-type-variable` and the three `truthiness-test-of-{callable,iterable,none-union}` lints. The last three are the diagnostics through which `ty.truthiness` becomes visible.
- Behaviour, from the skill's migration page: at ty 0.0.14 imports (`Import`, `ImportFrom`, `StarImport`) are declaration plus binding; at 0.0.16 they are bindings only. This bears on `ty.import-statements`, `ty.live-bindings` and `ty.declarations-at-binding`. Dict-key assignment definitions descending into `**{...}` are a 0.0.16 change. ty 0.0.14 uses salsa `=0.28.2`.

**ruff 0.0.14 differences on unconsumed surfaces:** the `TokenKind::Name` to `Identifier` rename (`ruff.tokens`) and the `NameMatcher` addition. Nothing else was found among the items named by the 92 unconsumed ruff records, across ruff_python_ast, ruff_python_parser, ruff_python_semantic, ruff_python_trivia, ruff_python_index, ruff_python_codegen, ruff_python_literal, ruff_python_importer, ruff_python_stdlib, ruff_source_file, ruff_notebook, ruff_db, ruff_diagnostics, ruff_linter, ruff_workspace, ruff_graph, ruff_python_formatter, ruff_formatter, ruff_markdown and ruff_ranged_value. Rule and key counts quoted in records (971 rules, 183 configuration keys) are 0.16.10 figures; 0.16.8 has 970 rules per the skill's own tables, and the key count at 0.16.8 is unverified.

**Behaviour transfer.** For every ty and ruff record not listed above, the exactness and limits wording was observed at 0.0.16 and is **version-transfer unverified** at 0.0.14. The exceptions are ty index facts that the retired ty-flow skill covered at 0.0.14 (`~/skill-work/ty-flow-0.0.14-archive`) and identical item surfaces. Item presence is verified; behaviour is not.

## Facts that need an unpublished crate or a fork-patch extension

**ty, public only in unpublished `ty_ide` / `ty_project` / `ty_server`** (18): `ty.call-hierarchy`, `ty.quick-fixes`, `ty.rename`, `ty.parameter-docs`, `ty.docstring-sections`, `ty.document-symbols`, `ty.hover`, `ty.inlay-hints`, `ty.semantic-tokens`, `ty.signature-help`, `ty.folding-ranges`, `ty.selection-range`, `ty.uv-environments`, `ty.project-files`, `ty.symbol-search`, `ty.find-references`, `ty.output-positions`, `ty.annotation-edits`.

**ty, with an unpublished surface plus a published one** (10; the published part is usually `ty_python_semantic`): `ty.type-hierarchy`, `ty.reachability-evaluation`, `ty.type-check-diagnostics`, `ty.deprecation`, `ty.completions`, `ty.configuration`, `ty.implementations`, `ty.navigation`, `ty.unused-bindings`, `ty.script-metadata`.

**ty, no public Rust surface at all** (9; internal `pub(crate)`, CLI or report only): `ty.overrides`, `ty.match-exhaustiveness`, `ty.suppressions`, `ty.unused-suppressions`, `ty.server-capabilities`, `ty.dunder-all`, `ty.module-exports`, `ty.pydantic-models`, `ty.diagnostic-fingerprint`.

**ty, public in `ty_python_semantic` 0.0.14** (published; not linked by `cpg-flow` today): 51 facts.

**pyrefly, needs a fork-patch extension** (8):
- `pyrefly.match-exhaustiveness`: `pyrefly::alt::answers_solver::AnswersSolver::check_match_exhaustiveness` is `pub(crate)`.
- `pyrefly.error-counts`: `pyrefly::error::summarize::print_error_summary` is in a private module.
- `pyrefly.suppression-inventory`: `pyrefly::commands::coverage::types::ReportSuppression` is in a private module.
- `pyrefly.signature-help`: `pyrefly::error::signature_diff::render_signature_diff` is in a private module.
- `pyrefly.config-migration`: `pyrefly_config::migration::config_option_migrater::ConfigOptionMigrater::migrate_from_mypy` is in a private module; `pyrefly_config::migration::config_option_migrater::ConfigOptionMigrater::migrate_from_pyright` is in a private module.
- `pyrefly.retained-tokens`: `pyrefly::state::state::Transaction::get_parsed_module` is `pub(crate)`.
- `pyrefly.type-coverage`: `pyrefly::commands::coverage::types::FullReport` is in a private module; `pyrefly::commands::coverage::types::SymbolReport` is in a private module.
- `pyrefly.stub-generation`: `pyrefly::stubgen::extract::extract_module_stub` is in a private module; `pyrefly::stubgen::extract::ModuleStub` is in a private module; `pyrefly::stubgen::emit::emit_stub` is in a private module.

Reachable without a patch, with one private part: `pyrefly.build-system-db` (`pyrefly_build::source_db::query_source_db::QuerySourceDatabase` is in a private module); `pyrefly.subtype-query` (`pyrefly::solver::solver::Subset` is in a private module).

In `pyrefly.decorators`, `pyrefly.deprecation`, `pyrefly.module-dependency-graph`, `pyrefly.test-modules` and `pyrefly.timings-and-memory`, the private-module parts lie under `report`/`export` and are already opened by the existing patch (changes 3 and 5). A fact marked "no further patch" whose route names the doc-hidden session API relies on `#[doc(hidden)]` items that move between dev releases.

## Re-classed records (claim class differs from the record's fidelity)

- `pyrefly.match-exhaustiveness`: record fidelity `diagnostic-only` -> **evaluated verdict** (exhaustiveness verdict, carried only as a diagnostic).
- `ty.match-exhaustiveness`: record fidelity `inferred` -> **evaluated verdict** (exhaustiveness verdict, internal; observable via reachability).
- `pyrefly.redundant-condition`: record fidelity `diagnostic-only` -> **evaluated verdict** (always-true/false test verdicts, carried only as diagnostics).
- `ty.truthiness`: record fidelity `inferred` -> **evaluated verdict** (always-truthy/falsy/ambiguous verdict from the type).
- `pyrefly.unreachable-diagnostics`: record fidelity `diagnostic-only` -> **evaluated verdict** (suite-level dead-code verdicts, carried only as diagnostics).
- `ty.reachability-evaluation`: record fidelity `inferred` -> **evaluated verdict** (evaluated per range (Unconditional vs CurrentAnalysis)).
- `ty.terminal-calls`: record fidelity `inferred` -> **evaluated verdict** (IsNonTerminalCall resolved true/false by the semantic layer).
- `ruff.lint-diagnostics`: record fidelity `resolved` -> **diagnostic-only** (record says resolved; it is the finding stream itself).
- `ty.check-reports`: record fidelity `inferred` -> **diagnostic-only** (record says inferred; it is an output rendering of diagnostics).
- `ty.type-check-diagnostics`: record fidelity `inferred` -> **diagnostic-only** (record says inferred; the payload is a diagnostic stream).
- `pyrefly.unused-suppressions`: record fidelity `resolved` -> **diagnostic-only** (record says resolved; relative to the run's errors and preset).
- `ty.unused-suppressions`: record fidelity `syntactic` -> **diagnostic-only** (record says syntactic; the result is relative to the run's diagnostics).

## Blank and partial concepts

These are concepts where no provider covers the concept, or every provider covers it only partially, taken from the skill's `content/facts/blanks.json` (29 entries) with the status at the baseline added. "Unchanged" rests on item presence at the 0.16.8 tag. No blank concept gains a provider at ty 0.0.14 or ruff 0.0.14, and none loses one.

- **`directives.todos`** (partial): ruff: ruff.todo-directives: TODO parsing is pub(crate); attributes only as TD/FIX findings. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`exports.public-names`** (partial): pyrefly: Computed internally for coverage --public-only; not emitted as a list.; ty: all_symbols gives qualified names of module globals but does not follow re-export chains to public paths. Unchanged: pyrefly is at the indexed pin; ty's `all_symbols` is in the unpublished `ty_ide` at both versions.
- **`control.loops`** (partial): pyrefly: LoopPhi binding variant only; no loop/back-edge objects.; ty: LoopHeader definitions only; no back-edge graph. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`control.cfg`** (blank): No usable control-flow graph: ruff's `cfg` module is a stub that splits blocks only at return and raise (compound statements are no-ops; its one consumer builds only with test-rules). ty's use-def and reachability diagrams and pyrefly's binding graph encode flow without blocks or edges. A consumer needing basic blocks or dominators must build them. Unchanged at the baseline: `ruff_python_semantic::cfg::graph::build_cfg` is the same stub at 0.0.14, and PLW0101 was test-only at 0.16.8 too.
- **`conditions.truthiness`** (partial): pyrefly: Diagnostics only (redundant-condition, unnecessary-comparison, implicit-bool).; ruff: ruff.truthiness: literal and builtin-constructor truthiness only.; ty: Internal Type::bool; visible through reachability and lints. Narrower at ty 0.0.14: the `truthiness-test-of-{callable,iterable,none-union}` lints do not exist yet, so less of ty's truthiness is visible as diagnostics. `ty_python_core::Truthiness` and the reachability route are present.
- **`conditions.match-exhaustiveness`** (partial): pyrefly: the `non-exhaustive-match` diagnostic (warn) names the missing cases; no data surface.; ty: computed inside type inference (crate-private); visible only through reachability and implicit-return checks, with no lint. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`dataflow.intraprocedural`** (partial): ruff: Binding-to-reference lists within a scope only; no flow to sinks.; ty: Use-def gives def-use chains; no flow to sinks. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`dataflow.interprocedural`** (partial): pyrefly: Captured variables and inferred return types only; no read/write summaries. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`dataflow.points-to`** (blank): None of the three tracks objects: no allocation sites, aliasing or mutation. A field location or a name never proves which object it refers to, so claims about shared or mutated state stay unknown. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`dataflow.constants`** (partial): pyrefly: Literal types state exact values; division-by-zero uses them; no ranges, string values or abstract interpretation.; ty: Literal types only. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`dataflow.taint`** (blank): No source-to-sink taint among the three (Pysa, a separate tool built on pyrefly's reports, does this). Security-relevant flows need another tool. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`dataflow.side-effects`** (partial): pyrefly: unused-call-result / unused-coroutine lints only.; ruff: ruff.side-effects: syntactic contains_effect/side_effect heuristic. Unchanged: `ruff_python_ast::helpers::{contains_effect, side_effect}` are present at 0.0.14.
- **`dataflow.effect-kinds`** (blank): No provider states the effects of a call (I/O, mutation, resources); only lints hint at them. Effects must come from authored models or another analysis. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`exceptions.propagation`** (blank): No may-raise sets per call and no propagation graph: ty treats try bodies as AMBIGUOUS and calls as possibly non-returning; pyrefly reports raise types and dead handlers only. Which exceptions escape a function is unknown. pyrefly and ty type each `raise` operand (exceptions.raised-types) but aggregate nothing per function; ruff's DOC501/502 compare raised names with docstrings syntactically. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`diagnostics.metrics`** (partial): pyrefly: error counts per kind and per directory, as text; no complexity measures.; ruff: `--statistics` counts findings per rule (JSON available); complexity and size appear only inside messages above a threshold. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`docs.parameter-docs`** (partial): pyrefly: Sphinx and Google conventions only, by heuristic parsing.; ty: its parser is in a private module; reachable through hover and signature help.; ruff: docstring lints only (pydocstyle rules), no parameter table. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`docs.sections`** (partial): ruff: Diagnostic-only (D4xx, DOC rules).; ty: Parsed for rendering only. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`project.unreferenced-definitions`** (blank): No provider emits project-wide dead definitions. They can be derived from pyrefly's Glean cross-references or ty's find_references, but dynamic access (getattr, plugins, entry points) makes any such list unsound. ty's unused-binding hints exclude module and class scopes. Unchanged: `ty_python_semantic::types::ide_support::unused_bindings` exists at 0.0.14, still excluding module and class scopes.
- **`editor.code-lens`** (partial): pyrefly: run and test lenses from naming patterns (heuristic), through its language server.; ty and ruff: none. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`runtime.dynamic-attributes`** (partial): pyrefly: Diagnostics for dynamic bases and implicitly defined attributes only.; ruff: B009/B010 flag constant getattr/setattr; computed names not analysed.; ty: __getattr__ fallbacks and hasattr narrowing. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`runtime.dynamic-imports`** (partial): ruff: ruff.string-imports heuristic in analyze graph; no importlib analysis.; ty: Literal-string import_module/__import__ only. ty literal-string inference unverified at 0.0.14 (behaviour); `ruff analyze graph --detect-string-imports` is present at 0.16.8.
- **`runtime.exec-eval`** (partial): ruff: S102/S307 flag the calls (ruff.dynamic-code-rules). Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`runtime.monkeypatching`** (partial): pyrefly: Only missing-attribute-patch-target for mock.patch strings. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`runtime.execution-evidence`** (blank): All three are static: no coverage, profiles or executed paths. Runtime evidence needs instrumentation (sys.monitoring, coverage tools). Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`concurrency.async-flow`** (partial): pyrefly: unused-coroutine and not-async diagnostics; no await graph.; ruff: Diagnostic-only ASYNC/RUF006 and in_async_context.; ty: Awaitable checks only (invalid-await, unused-awaitable). Unchanged by item presence; ty lint differences between 0.0.14 and 0.0.16 do not touch async lints.
- **`concurrency.resources`** (partial): pyrefly: Context-manager protocol checks and exception suppression; no resource lifetimes.; ruff: Diagnostic-only SIM115/SIM117.; ty: Context-manager protocol and suppression; no resource pairing. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`concurrency.threads`** (blank): None of the three models threads, locks or shared mutable state; ruff's ASYNC rules flag blocking calls in async code only. Data races and lock discipline need another analysis. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.
- **`identity.stable-ids`** (partial): pyrefly: qualified names and Glean's content digests are the stable keys, filed under scopes and the project index.; ty: qualified names from symbol search; GitLab fingerprints are deterministic per build only.; ruff: GitLab fingerprints (rule, path, ordinal), deterministic per build only. Run-local ids are identity.run-local-ids. Unchanged in shape. The fingerprints come from the ruff_db 0.0.14 GitLab renderer, and their equality to the 0.0.16 hashing is unverified.
- **`native.extensions`** (blank): Extension modules are known only through their stubs; their behaviour beyond declared signatures is invisible to all three. Status at the baseline: unchanged as far as item presence shows; behaviour transfer unverified.

The skill also marks 19 individual records `partial: true`. Eighteen are unconsumed, and their entries above carry the partial wording in "Limits": `pyrefly.context-managers`, `pyrefly.error-counts`, `pyrefly.literal-values`, `pyrefly.loop-phis`, `ruff.cfg-stub`, `ruff.diagnostic-fingerprint`, `ruff.handled-exceptions`, `ruff.noqa-hover`, `ruff.statistics`, `ruff.truthiness`, `ty.context-managers`, `ty.diagnostic-fingerprint`, `ty.dynamic-attributes`, `ty.dynamic-imports`, `ty.literal-values`, `ty.loop-headers`, `ty.truthiness` and `ty.try-except-flow`. The nineteenth, `pyrefly.public-fqns`, is consumed.

## Skill records that look wrong or inconsistent

These are observations for the skill's maintainer. The skill was not edited.

1. **`ruff.terminal`**: the record has fidelity `syntactic`, `derived_from: [ruff.ast]` and the surface `Terminal::from_function -> Terminal{...}`. At both 0.0.14 and 0.0.16 the signature is `from_function(function: &StmtFunctionDef, semantic: &SemanticModel)`, and it calls `semantic.match_builtin_expr(exc, "NotImplementedError")`. It therefore needs a populated SemanticModel (no public driver), which makes it resolved in part and not a free function over an AST. Source: `crates/ruff_python_semantic/src/analyze/terminal.rs` lines 26 and 162 at tag 0.16.10.
2. **The embedded-line claim is narrower than SKILL.md states.** SKILL.md says the only 0.0.14 to 0.0.16 difference is the identifier-token rename. The table (`content/catalogs/pyrefly-embedded-ruff.md`, `index/ruff-lines.tsv`) covers only ruff_python_ast, ruff_python_parser, ruff_source_file and ruff_text_size. `ruff_python_trivia::NameMatcher` (the record `ruff.name-prefilter`) is new at 0.0.16 and absent at 0.0.14, and the record carries no version note.
3. **Fidelity is inconsistent across providers for diagnostic streams.** `ty.type-check-diagnostics` and `ty.check-reports` are `inferred`, `ruff.lint-diagnostics` is `resolved` and `ty.unused-suppressions` is `syntactic`. Yet `ty.unused-suppressions` is relative to the run's diagnostics, as is `pyrefly.unused-suppressions` (`resolved`), while `ruff.unused-suppressions` is `diagnostic-only`. This catalogue re-classes the five; see "Re-classed records".
4. **`pyrefly.timings-and-memory`**: the record notes that `--report-timings` writes CSV "although the skill catalog lists it as JSON". That is an inconsistency inside the skill, already self-reported.
5. **Counts quoted at 0.0.16 without a version mark**, which can mislead at 0.0.14: `ty.lint-registry` (138; 134 at 0.0.14), `ty.type-algebra` (36 variants, including two types that are 0.0.16 only) and `ruff.rule-vocabulary` / `ruff.config-schema` (0.16.10 counts). These are correct for the skill's pin; they are not errors at that pin.

No record names a Rust item that is missing at both tags. Every item named in the unconsumed ruff and ty records was found at 0.0.16, and all but those listed under "Baseline differences" at 0.0.14.

## Limits of this catalogue

- "Not consumed" follows the skill's approximate overlay (`facts_used`), not the repository. Some facts listed here may already be used through another route (for example, Pysa collectors already read beside the consumed ones). The sibling code-mapper decides.
- The version check is by item name and definition visibility at the two tags. Signature changes of items that keep their name were not compared for ty_python_semantic, ty_ide or ty_project. The skill's index diff covered `ty_python_core`, `ty_module_resolver` and `ty_vendored` and found no signature changes on the paths used. Behaviour (exactness, limits) at 0.0.14 is unverified unless stated.
- Attachment codes describe the natural route at the baseline. Several facts have more than one route (in-process and CLI); the code names the in-process one when it exists.
- Claim-class overrides (7 evaluated verdicts, 5 diagnostic-only) are this worker's judgement. Every other class is the record's own fidelity.
- Sources consulted: the skill's records, briefs (fork-patch), migration page, blanks and embedded-line table; astral-sh/ruff tags 0.16.8 and 0.16.10 (temporary sparse checkouts, removed after use; reproduce with `git clone --depth 1 --branch <tag> --filter=blob:none --sparse https://github.com/astral-sh/ruff`); crates.io metadata for ty_python_semantic, ty_ide, ty_project, ty_server and ruff crates (2026-10-03, generic User-Agent); `~/.cargo/registry` sources of ruff_python_trivia 0.0.14 and 0.0.16 and ty_python_core 0.0.14.
