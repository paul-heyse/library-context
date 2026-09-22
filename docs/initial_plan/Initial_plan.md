## Conclusion

**Your hypothesis is substantially correct as a choice of semantic front ends: Ruff and Pyrefly can provide the foundation for an exceptionally rich Python-library fact graph without introducing another parser or type checker. However, their existing outputs do not, by themselves, constitute a complete execution-oriented code property graph.**

The distinction is between:

1. **Facts already exposed through APIs or structured reports.**
2. **Facts available internally that need a Rust extraction adapter.**
3. **New analyses that must be implemented over those facts**, particularly full control flow, reaching definitions, heap/alias relationships, and program dependence.

The source audit uncovered two especially consequential findings. First, **Pyrefly has substantial structured Glean, Pysa, and CinderX exports**, including considerably more call-resolution and type information than its ordinary diagnostics suggest. Second, **Ruff’s exposed CFG builder is not a complete Python CFG builder**: at the inspected revision, branches, loops, exception handlers, and several other control constructs have empty handling arms.   

I created an accompanying specification with **53 logical tables, 35 node kinds, 94 edge kinds, native variant inventories, extraction classifications, and 31 pinned source anchors**:

**:chatgpt-content-reference{index="60"}**  
:chatgpt-content-reference{index="61"} · :chatgpt-content-reference{index="62"}

This is a **source-audited design**, not an implemented extractor. I validated the schema document’s internal consistency, but did not compile adapters, execute the reports, or run semantic correctness fixtures.

---

## 1. Audit scope and the meaning of “available”

I inspected these pinned development sources:

| Component | Inspected revision | Important scope distinction |
|---|---|---|
| Ruff | `660350be2648e60e0c241e24a6ed05a38b4098fa` | Ruff’s workspace also contains `ty`; I have not silently included the full `ty` type checker. |
| Pyrefly | `9733bdcfdf05355f816f8d8f919e01bd034efdb7` | Its inspected manifest identifies it as `1.4.0-dev.1`. Findings should not be assumed to apply to every released binary. |

Pyrefly already depends on Ruff’s AST/parser family. It does **not** thereby inherit Ruff’s linter semantic model. Also, `ruff_graph` uses the shared `ty_module_resolver`, which is a narrower dependency than adding `ty` type inference.   

Throughout this assessment:

| Classification | Meaning |
|---|---|
| **Exposed** | A parser API, query API, or structured report already provides the relevant information. Normalization is still necessary. |
| **Internal** | The analyzer computes or represents the information, but obtaining it structurally requires an adapter, instrumentation, or a small upstream change. |
| **Derived** | Producing the relationship requires additional analysis, not merely exporting another internal field. |

That distinction matters because “the type checker uses this information” does not necessarily mean “the type checker retains it in an exportable form.”

For the resulting graph, I recommend a layered, attributed multigraph. This is consistent with conventional CPG organization, which separates syntax, calls, control flow, and dependence layers, while allowing a richer Python-specific ontology than a minimal generic CPG. :chatgpt-content-reference{index="6"}

---

# 2. What Ruff can contribute

## 2.1 Source, tokens, syntax, and exact structural relationships

The parser is a strong foundation for the source-preserving layer. Its `Parsed<T>` contains **syntax, tokens, parse errors, and version-related unsupported-syntax errors**. Recovery-oriented entry points allow retaining a parsed result without reducing the entire file to its first syntax failure. Importantly, `has_valid_syntax()` does not include version-related unsupported-syntax errors, whereas `has_no_syntax_errors()` does.  

| Ruff component | Obtainable facts | Access |
|---|---|---|
| `ruff_python_parser` | Parsed modules and expressions; tokens; syntax errors; unsupported syntax for the target Python version; source-type/mode distinctions; parsed string annotations | Exposed |
| `ruff_python_ast` | Every represented statement, expression, pattern, declaration, argument, parameter, decorator, comprehension, annotation, and literal; ordered structural children and scalar fields | Exposed |
| `ruff_text_size`, `ruff_source_file` | Source ranges and position mapping needed to anchor facts in the exact parser input | Exposed |
| `ruff_python_index`, `ruff_python_trivia` | Comment ranges, continuation locations, multiline-string ranges, interpolated-string ranges, and source-trivia relationships | Exposed |
| Notebook support | Notebook/cell source representations and mappings that can be preserved alongside extracted syntax | Exposed, with normalization |

The AST inventory at this revision includes **25 statement variants, 33 expression variants, and eight pattern variants**, plus type-parameter and auxiliary node families. The accompanying specification enumerates them. Ruff’s `ast.toml` is particularly valuable: it is a machine-readable definition used to generate its AST model, rather than an informal description of that model.   

For your graph, retain the following distinctions rather than collapsing them during ingestion:

| Fact family | Structure worth preserving |
|---|---|
| Functions | Name, decorators and order, parameters, defaults, annotations, type parameters, return annotation, body, `is_async` |
| Classes | Name, bases and keyword arguments as syntax, decorators, type parameters, body |
| Assignments | Each target, assigned expression, annotation, operator for augmented assignment, unpacking structure |
| Calls | Callee expression, argument occurrences, positional/keyword/starred structure, ordering |
| Control constructs | Tests, bodies, alternatives, handlers, guards, finalizers, loop `else`, match patterns |
| Literals | Original spelling and parsed value; string prefixes; interpolation structure; implicit concatenation parts |
| Imports | Requested module/member, alias, relative level, wildcard form, lazy-import flag |
| Types as syntax | Annotation expression, string annotation, alias declaration, type-parameter bound/default syntax |
| Auxiliary structure | `Arguments`, `Parameters`, `ParameterWithDefault`, `Keyword`, `Alias`, `WithItem`, `MatchCase`, `Decorator`, and related nodes |

These distinctions are directly represented in Ruff’s AST schema. For example, synchronous and asynchronous functions share a node kind with an `is_async` field; `try` and `try*` share a node with `is_star`; f-strings and t-strings preserve constituent parts rather than simply becoming one concatenated string.  

**Do not treat the AST alone as a lossless concrete syntax tree.** Preserve the original source and token/trivia information as well. Ruff’s indexer explicitly handles information omitted from the AST, including comments and some continuation-related trivia. 

### Recommended extraction contract

Generate an exhaustive Rust exporter over the AST’s variants and fields. Its contract should preserve:

```text
AST variant
source anchor
named child field
child ordinal, where applicable
scalar field value
absent versus empty
original source spelling
```

This prevents a seemingly comprehensive visitor from quietly losing defaults, decorator ordering, keyword names, interpolation pieces, or auxiliary syntax.

---

## 2.2 Lexical scopes, bindings, references, and execution context

Ruff’s semantic model adds meaningful information beyond syntax.

A `Binding` carries its **kind, source range, scope, execution context, originating node, references, tracked exception context, and flags**. Its flags distinguish such things as explicit exports, imported symbols, aliases, global/nonlocal bindings, deletion, unpacked assignments, private declarations, and certain annotation contexts. 

The full inspected `BindingKind` inventory is:

```text
Annotation
Argument
NamedExprAssignment
Assignment
TypeParam
LoopVar
WithItemVar
Global
Nonlocal
Builtin
ClassDefinition
FunctionDefinition
Export
FutureImport
Import
FromImport
SubmoduleImport
Deletion
BoundException
UnboundException
DunderClassCell
```

The last two categories are important. `UnboundException` represents the implicit clearing of an exception target after an `except ... as name` block. `DunderClassCell` represents the special class closure used for `__class__` and `super` behavior. These are not equivalent to ordinary assignment syntax.  

The corresponding graph can retain:

| Semantic family | Available information |
|---|---|
| Scope hierarchy | Module, class, function, lambda, generator/comprehension, annotation/type, and implicit class-cell scopes |
| Binding history within a file | All bindings, including shadowed bindings, rather than only the final name-to-binding map |
| Name resolution | Resolved reference-to-binding relationships and unresolved references |
| Import identity | Qualified imported name, local alias, straight/from/submodule import distinctions |
| Export information | Explicit re-exports and recognized `__all__` contents |
| Reference context | Load context; typing-only versus runtime contexts; string annotations; type-checking blocks; annotation and assertion contexts |
| Global/nonlocal relationships | Links to the relevant outer bindings/scopes |
| Uncertainty | Wildcard-import uncertainty, unresolved annotations, limited handled-exception context |

Ruff exposes both final bindings and all bindings including shadowed ones. Its reference structures also preserve contextual flags and cases where a reference has no ordinary expression-node identity, such as some augmented-assignment or global-reference cases.  

Two limitations must remain visible in the ontology:

**First, Ruff’s exception mask is not a general exception-effect analysis.** It tracks four specific categories: `NameError`, `ModuleNotFoundError`, `ImportError`, and `AttributeError`. It cannot be relabeled as “all exceptions this operation can raise.” 

**Second, some scope semantics are deliberate approximations.** The source explicitly documents that its implicit `__class__` scope is introduced more broadly than Python creates it at runtime. Store this as an analyzer assertion under a named model, not an unconditional runtime truth. 

---

## 2.3 Ruff’s semantic extraction is not a standalone public pass

This is the principal integration issue on the Ruff side.

The linter’s crate-private `Checker` constructs the `SemanticModel` while traversing the AST and running checks. The traversal includes deferred work, notably function bodies after their surrounding scopes have been traversed. Consequently, constructing a `SemanticModel` does **not** amount to calling a complete public `analyze(source)` function. 

My recommendation is a narrow, pinned adapter inside or adjacent to that machinery that emits structural facts **after all required deferred semantic work has completed**.

That is extraction engineering, not a reason to add another semantic engine.

---

## 2.4 Module dependencies, diagnostics, and supporting crates

`ruff_graph` supplies resolved module dependency/dependent maps. It collects imports, resolves them against an environment, and has configuration for string-import and type-checking-import handling. Its result is useful as an import-dependency graph, not a call graph or a dataflow graph. 

Lint diagnostics, proposed fixes, and edits can be retained as optional analyzer-observation layers. The Ruff checker’s diagnostic machinery is integrated with semantic traversal, but diagnostics should not substitute for the underlying semantic facts. 

For your primary purpose, the formatter, code-generation, configuration, database, indexing, and utility crates are supporting infrastructure. I would not turn each helper crate into a separate conceptual graph layer merely because it can produce output.

---

## 2.5 The critical CFG limitation

At the inspected revision, Ruff’s `build_cfg` provides block and edge machinery, but its actual implementation has:

- Only `Condition::Always`.
- Empty handling arms for `if`, `match`, `while`, `for`, `try`, `with`, `break`, `continue`, and `assert`.
- Block splitting and terminal jumps for `return` and `raise`. 

**It would therefore be incorrect to say that importing `ruff_python_semantic::cfg` gives you a full Python control-flow graph.**

You can preserve its output as a limited analyzer graph. A full execution layer requires additional semantic lowering.

---

# 3. What Pyrefly can contribute

## 3.1 There are several distinct extraction surfaces

Pyrefly should not be approached only as “a type checker that emits diagnostics.”

| Surface | Main facts | Principal limitation |
|---|---|---|
| **Glean report** | Declarations, definitions, locations, cross-references, calls, containment, docstrings | Navigation/indexing-oriented, not a complete native semantic dump |
| **Pysa report** | Definitions, classes, captures, expression-type projections, rich call-resolution information, overrides, uncertainty | Types are projected; some calls are analysis-generated models |
| **CinderX report** | Structured type tables, located types, selected narrowed/unnarrowed/contextual information, class ancestry/tags | Compiler-oriented projection of the native type system |
| **Query/TSP** | Type queries, selected structured shapes, callees/attributes, imports and environment information | Experimental interfaces and important semantic caveats |
| **Debug/trace** | Bindings, results, types, definition locations | Significant payloads are rendered strings; trace is incomplete |
| **Native state/answers** | Full type variants, class metadata, binding IR, solver answers | Requires retained state and a version-pinned adapter |

These distinctions are established by the actual report drivers, type schemas, query implementation, and debug/trace serialization, rather than their names alone.      

## 3.2 Glean: the navigation and source-reference layer

The actual Glean converter accumulates facts for:

```text
modules
declaration locations
definition locations
wildcard-import locations
file calls
callee-to-caller relationships
containing top-level declarations
cross-references
name-based cross-references
declaration docstrings
name hierarchy
```

Its source spans are byte-oriented. Its definition lookup also has an explicit source-versus-stub preference, which is another reason not to merge `.py` and `.pyi` entities purely because they have the same qualified name. 

This report is a valuable existing export for agent questions such as “where is this defined?”, “what references it?”, and “what is its containing declaration?”

---

## 3.3 Pysa: a substantial call-resolution and object-model export

The Pysa report is the most important discovery for your architecture.

### Definitions and class structure

Its schema supports:

| Entity | Exportable information |
|---|---|
| Project/module | Module identity, source path/category, Python version/platform, interface/init/internal/test flags, load status |
| Function | Name, location, parent, method/property/overload/stub distinctions, defining class |
| Signature | Undecorated signature, parameter names/kinds/requiredness/annotations, return annotation, parameter-list versus ellipsis/ParamSpec forms |
| Class | Bases, reported MRO state, fields, declaration origins, synthesized/dataclass/NamedTuple/TypedDict distinctions |
| Closure | Captured-variable information |
| Global | Global-variable definitions and associated information |
| Override | Overridden-method and override relationships |
| Expression type | Located expression types with per-function deduplicated type tables |

These are concrete structures in the Pysa report schema and driver. They are not hypothetical facts that would require reimplementing a type checker.  

### Calls and dispatch

The call-related schema includes substantially more than a `caller → callee` pair:

| Relationship | Information preserved |
|---|---|
| Ordinary calls | Candidate call targets |
| Construction | Separate `__new__` and `__init__` targets |
| Method dispatch | Receiver information, class/static-method flags, implicit receiver behavior |
| Callable objects | Implicit `__call__` information |
| Higher-order calls | Callable targets associated with argument/parameter positions |
| Properties | Getter/setter targets associated with attribute access |
| Decorators | Decorator-related targets |
| Overrides | Targets that represent an override set rather than one concrete function |
| Name/attribute values | Potential targets **if the value is called** |
| Unresolved behavior | Explicit unresolved reasons, including mixed cases |
| Analysis models | Formatting and return-shim relationships |

The schema contains **14 unresolved-reason variants**. A resolution can contain useful known targets and still retain unresolved possibilities. 

Three normalization rules are essential:

**`ifCalled` is not a call.** A function-valued attribute can have potential callable targets without being invoked at that location.

**A synthetic shim is not necessarily a source invocation.** Formatting and return-shim relationships need explicit synthetic-model provenance.

**A candidate target is not proof of exhaustive runtime dispatch.** Retain the provider’s unresolved information and the analysis assumptions. 

### Pysa types are not the full native type graph

Pysa’s type representation contains display information, scalar classifications, extracted class information, modifiers, and related properties. It is useful, but it is not a lossless serialization of Pyrefly’s recursive native `Type` algebra. 

Also, the report does not provide a universal, complete actual-argument-to-formal-parameter binding graph for every call. The richer mappings it does expose should be preserved, but general argument binding remains a separate extraction/analysis task.

---

## 3.4 CinderX: structured types beyond display strings

Pyrefly also produces a CinderX report with:

```text
index.json
types/<module>.json
class_metadata.json
```

The per-module files contain deduplicated type tables and located types. The global class metadata includes qualified names, ancestry, and semantic tags. 

Its actual `StructuredType` enum has six variants:

```text
Class(qname, args, traits)
Callable(params, return_type, defining_func?)
OtherForm(qname, args)
BoundMethod(self_type, func_type, defining_class?)
Variable(name, bounds)
Literal(value, promoted_type)
```

Located observations can additionally contain:

```text
type
unnarrowed_type?
is_narrowed_mismatch
contextual_type?
```

That gives you useful existing structured output for distinguishing certain flow-narrowed types from their unnarrowed and contextual counterparts. 

Nevertheless, this remains a projection. For example, callable parameter type indices do not constitute a complete parameter declaration with names, kinds, and defaults.

Two implementation details deserve preservation in provenance:

**The reported ancestor list excludes the class itself and `object`.** It should not be labeled a literal complete runtime MRO.

**The CinderX type-table implementation deduplicates by a 64-bit hash without a structural equality check.** Preserve its hashes as provider identifiers, not as your canonical global type identity.  

---

## 3.5 The maximal native type layer

For maximal depth, export Pyrefly’s native type structures as a graph.

At the inspected revision, the native `Type` enum has **54 variants**. They span the following families:

| Family | Native information to preserve |
|---|---|
| Fundamental types | `Any` with style, `Never` with style, `None`, literals, literal strings, ellipsis |
| Class/object distinctions | Class definition, instantiated class type, type object, `Self`, `super` |
| Callables | Structural callable, declared function, bound method, overloads, overloaded result alternatives |
| Type composition | Union, intersection with fallback, tuple, unpacking |
| Generics | Quantification, applied arguments, type variables, ParamSpec, TypeVarTuple, concatenation, args/kwargs forms |
| Aliases | Alias declarations, recursive references, untyped alias forms |
| Refinement/metadata | TypeGuard, TypeIs, `Annotated` metadata, TypeForm |
| Structured records | TypedDict and partial TypedDict |
| Specialized models | Shaped arrays, integer/shape tuples, NN-module captures, DataFrame/Series schemas, type-level DSL applications |
| Analyzer machinery | Solver variables, special forms, keyword-call typing effects, materialization forms |

The specialized forms are particularly relevant to library understanding, but their presence does **not** imply that arbitrary NumPy, pandas, PyArrow, or user-defined data structures automatically receive complete schema inference. They are analyzer-supported representations whose applicability depends on the analyzed program and enabled modeling.  

A concrete exporter trap illustrates why this should be structural rather than visitor-only:

> Pyrefly’s generic `Type` visitor visits the underlying type of `Annotated`, but skips its metadata. Traversal of applied type arguments also does not substitute for traversal of declaration-owned type parameters.

A naïve recursive visitor could therefore lose exactly the metadata, bounds, and defaults that would make your agent knowledge richer. Export those fields explicitly.  

---

## 3.6 Rich class metadata and synthesized behavior

The internal class metadata includes more than the Pysa class projection:

```text
metaclass and class keywords
bases and generic-base information
protocol, enum, TypedDict and NamedTuple metadata
dataclass and dataclass-transform metadata
abstract/final/deprecation information
explicit slots: absent / unknown / known
synthesized fields
total-ordering metadata
framework-specific class classifications
shape-related constructor capture information
```

The inspected implementation includes classifications for Pydantic, attrs, Django, Django REST Framework, Marshmallow, and factory-boy-related models. These should be retained as **Pyrefly model assertions**, not independent claims that the graph has reconstructed every runtime behavior of those frameworks. 

---

## 3.7 Binding IR, narrowing, inference dependencies, and answers

Pyrefly’s internal bindings are a separate concept from Ruff’s name-binding events.

Its general `Key` enum includes **27 variants**, covering definitions, imports, mutable captures, explicit/implicit returns, expressions, context managers, match subjects, type joins, narrowing, unpacking, deletion, exhaustiveness, and related inference operations. There are also **25 typed key families** for class metadata, bases, fields, variance, annotations, decorators, type parameters, yields, and other computations.   

This supports an excellent **type-explanation graph**:

```text
analysis binding
    → depends on another analysis binding
    → corresponds to source construct
    → participates in type join or narrowing
    → produces an answer
```

But:

**Pyrefly’s `Phi` is not automatically an execution SSA phi-node, and its inference dependency graph is not automatically a runtime dataflow graph.**

Keep those layers separate until a validated derivation relates them.

For retained native extraction, `Require::Everything` is important: it keeps ASTs, answers, and answer traces. Lower retention levels intentionally discard some of that information. Retention itself also does not prove that every possible query has already been computed. 

---

## 3.8 Query interfaces: useful, but verify their semantics

Two current interface behaviors are especially important:

| Interface | Actual inspected behavior |
|---|---|
| TSP `getDeclaredType` | Currently delegates to computed `type_at_position`, despite its name |
| TSP `getExpectedType` | Returns contextual expected type where applicable, otherwise falls back to computed type |

Thus, you must obtain actual declaration annotations from syntax/native annotation information rather than trusting the `getDeclaredType` transport name.  

Pyrefly also exposes assignability computations through `is_subset_eq` and a reason-returning counterpart. These can support on-demand compatibility facts. However, the implementation describes the operation as **assignability**; with gradual types such as `Any`, this should not be indiscriminately renamed mathematical subtyping. 

Finally, Pyrefly explicitly warns that its Rust library interface is unstable. Its query module is also described as experimental. A pinned integration boundary is therefore part of the design, not an optional precaution.  

---

# 4. The maximal graph obtained by synthesizing these sources

I would define the first deliverable as:

> **A source-preserving, environment-qualified Python semantic graph with structural types, explicit resolution sets, and analyzer-explanation relationships.**

That is already a powerful foundation for agents. Execution dependence becomes a separately identified overlay.

## 4.1 The ontology should distinguish entities that are often incorrectly merged

Consider:

```python
def transform(value: int | str) -> str:
    if isinstance(value, int):
        result = str(value)
    else:
        result = value
    return result
```

A useful graph distinguishes the function’s syntax node, its semantic function entity, parameter declaration, parameter binding, individual references to `value`, annotation syntax, computed types at different occurrences, assignments to `result`, and any later execution points.

**A variable name is not a binding event; a binding event is not a reference; a type is not a declaration; an AST node is not an execution point.**

The proposed node ontology is:

| Layer | Node kinds |
|---|---|
| Source | `Artifact`, `SourceFile`, `Module`, `SyntaxNode`, `Token`, `Trivia` |
| Lexical semantics | `Scope`, `Symbol`, `Binding`, `Reference`, `Import`, `Export` |
| API/object model | `Function`, `Class`, `TypeAlias`, `Member`, `Signature`, `Parameter`, `TypeParameter` |
| Types | `Type` |
| Resolution | `CallSite`, `Argument`, `ResolutionSet`, `DispatchGroup`, `SyntheticCallable`, `ExternalSymbol` |
| Analyzer explanation | `AnalysisBinding` |
| Execution overlay | `ControlGraph`, `ControlPoint`, `MemoryLocation`, `Access` |
| Documentation/diagnostics | `Docstring`, `Diagnostic`, `Fix`, `Edit` |

This is a **code ontology**, not yet a domain-capability ontology. A later layer can connect these entities to concepts such as “columnar filtering,” “transactional table updates,” or “schema evolution,” but those classifications should carry their own evidence.

---

## 4.2 Exact core schema

The downloadable JSON contains every logical table, column, nullability declaration, key, reference, and edge contract. The central records are below.

All keys are snapshot-qualified. Here, `id16` means a 16-byte identifier, `hash32` a 32-byte digest, and `?` a nullable field.

```text
nodes(
    snapshot_id: id16,
    node_id: id16,
    node_kind: enum,
    existence_fact_id: id16
)

facts(
    snapshot_id: id16,
    fact_id: id16,
    run_id: id16,
    origin: enum,
    extraction_mode: enum,
    modality: enum,
    fidelity: enum,
    model_id: utf8,
    record_kind: utf8
)

edges(
    snapshot_id: id16,
    fact_id: id16,
    src_id: id16,
    edge_kind: utf8,
    dst_id: id16,
    role: utf8?,
    ordinal: int64?,
    resolution_id: id16?,
    guard_id: id16?
)

type_observations(
    snapshot_id: id16,
    fact_id: id16,
    subject_id: id16,
    type_id: id16,
    type_role: enum,
    program_point_id: id16?,
    requested_api: utf8?
)

resolutions(
    snapshot_id: id16,
    fact_id: id16,
    node_id: id16,
    domain: enum,
    status: enum,
    candidate_set_complete_under_model: boolean?,
    has_unresolved_remainder: boolean?
)

spans(
    snapshot_id: id16,
    span_id: id16,
    file_id: id16,
    start_byte: int64,
    end_byte: int64
)
```

The supporting tables connect `run_id` to the producer revision, adapter build, Python environment, import/search-path configuration, and enabled extraction families.

### Why facts are first-class records

A single node may receive assertions from multiple sources:

```text
Ruff identifies a lexical binding.
Glean identifies a definition candidate.
Pyrefly infers a type.
Pysa identifies possible dispatch targets.
A custom analysis derives value dependence.
```

These should not overwrite one another as mutable node properties. Each assertion needs its own provenance and evidence.

The proposed `origin` values are:

```text
input_context
source_observation
analyzer_assertion
derived_analysis
synthetic_model
```

The `fidelity` values are:

```text
raw
native_structural
normalized_structural
report_projection
display_only
```

This gives agents a programmatic way to distinguish a native type structure from a display string, or a source call from a synthetic analysis model.

---

## 4.3 Type observations must be contextual and multi-valued

Do not store:

```text
Symbol.type = "int"
```

Instead, store a separate observation for each relevant role:

```text
annotation
computed
expected
expected_or_computed
narrowed
unnarrowed
contextual
exported
decorated_callable
undecorated_callable
parameter
return
yield
send
native_answer
```

This accommodates both native Pyrefly information and the actual CinderX/TSP semantics without pretending they are interchangeable.

Type structure then becomes graph relationships:

| Relation | Meaning |
|---|---|
| `TYPE_COMPONENT` | Ordered constituent of a composite type |
| `HAS_TYPE_PARAMETER` | Declaration or type owns a parameter |
| `TYPE_PARAMETER_BOUND` | Upper bound |
| `TYPE_PARAMETER_CONSTRAINT` | One permitted constraint |
| `TYPE_PARAMETER_DEFAULT` | Default type argument |
| `OVERLOAD_ALTERNATIVE` | Alternative signature or overloaded result |
| `TYPE_ALIAS_TARGET` | Alias expansion/reference |
| `ANNOTATION_METADATA` | Metadata associated with an annotated type |
| `DECORATED_FROM` | Relationship between decorated and undecorated callable representations |
| `ASSIGNABLE_TO` | Recorded compatibility result under an explicit context |

Recursive types require back-references, not infinite tree expansion. Type variables must retain binder identity; two unrelated parameters both named `T` are not the same entity.

---

## 4.4 Resolution is a set with uncertainty, not a single edge

For a call whose known targets are `A.process` and `B.process`, with unresolved possibilities remaining:

```text
CallSite
    ← RESOLUTION_FOR — ResolutionSet
                           status = partial
                           has_unresolved_remainder = true
                           candidate_set_complete_under_model = null

ResolutionSet — CANDIDATE → A.process
ResolutionSet — CANDIDATE → B.process

CallSite — CALL_TARGET → A.process
CallSite — CALL_TARGET → B.process
```

Each target edge references the resolution set. Its evidence can retain receiver information, override dispatch, higher-order position, and the provider’s unresolved reasons.

A function-valued reference instead uses:

```text
Reference — POTENTIAL_CALL_TARGET → Function
```

It does not become a `CallSite` merely because Pysa supplies `ifCalled` information.

The graph also separates `DispatchGroup` from `Function`, so an override-family target does not masquerade as one resolved implementation.

---

## 4.5 Relation families

The complete registry contains 94 edge kinds. Its principal families are:

| Family | Representative edges |
|---|---|
| Source structure | `ARTIFACT_CONTAINS`, `MODULE_SOURCE`, `AST_CHILD`, `DECLARATION_OF` |
| Scope/binding | `OWNS_SCOPE`, `LEXICAL_PARENT`, `DECLARES`, `BINDS_SYMBOL`, `READS_BINDING`, `SHADOWS` |
| Closure/global semantics | `CAPTURES`, `GLOBAL_BINDING`, `NONLOCAL_BINDING` |
| Imports/exports | `IMPORTS_MODULE`, `IMPORTS_SYMBOL`, `ALIASES`, `EXPORTS`, `REEXPORTS`, `MODULE_DEPENDS_ON` |
| Types/signatures | `HAS_TYPE`, `HAS_SIGNATURE`, `HAS_PARAMETER`, `PARAMETER_TYPE`, `RETURN_TYPE`, type-component relationships |
| Classes | `BASE_CLASS`, `MRO_ENTRY`, `HAS_MEMBER`, `OVERRIDES`, `METACLASS`, `SYNTHESIZED_FROM` |
| Calls | `CALL_TARGET`, `INIT_TARGET`, `NEW_TARGET`, `HIGHER_ORDER_TARGET`, `DECORATOR_TARGET`, `MODEL_CALL_TARGET` |
| Analysis explanation | `ANALYSIS_FOR`, `ANALYSIS_DEPENDS_ON`, `TYPE_PHI_INPUT`, `TYPE_NARROW_INPUT`, `ANALYSIS_ANSWER` |
| Execution overlay | `CFG_NEXT`, `DOMINATES`, `POST_DOMINATES`, `CONTROL_DEPENDS_ON`, `REACHING_DEF`, `VALUE_DEPENDS_ON`, `MAY_ALIAS` |
| Evidence/navigation | `HAS_DOCSTRING`, `DOCSTRING_SYNTAX`, `DIAGNOSTIC_ON`, `HAS_FIX` |

The registry explicitly marks the execution relationships as custom-analysis outputs. Their presence in the schema does **not** assert that Ruff or Pyrefly already emits them.

---

## 4.6 Preserve unnormalized native detail without making JSON blobs the database

It is impractical to give every field of every evolving provider structure a dedicated top-level column immediately. But using only opaque JSON would undermine your objective.

The specification therefore includes a typed structural preservation table:

```text
record_fields(
    snapshot_id,
    record_id,
    field_path,
    value_kind,
    container_length?,
    variant_tag?,
    bool_value?,
    integer_decimal?,
    float_bits?,
    utf8_value?,
    binary_value?,
    node_value?,
    record_value?
)
```

This represents nested structs, variants, lists, maps, nulls, and back-references while preserving:

```text
missing versus empty
arbitrary-size integers
exact floating-point bit patterns
variant identity
ordered children
recursive references
```

Frequently queried concepts receive dedicated tables and edges. Less common native detail remains typed and programmatically accessible, rather than being discarded or reduced to `Display` output.

The native enum inventories are source-verified. I have not hand-flattened every auxiliary Rust payload struct in the repositories; the structural layer and exhaustive-exporter requirements make that remaining implementation work explicit.

---

## 4.7 Identity and evidence rules

These are essential to reliable synthesis:

| Rule | Consequence |
|---|---|
| Qualified names are labels, not globally sufficient IDs | Do not merge different package versions, roots, stubs, implementations, or redeclarations |
| Source spans alone are not unique node IDs | Include syntax kind and structural occurrence path |
| Provider arena IDs are local | Map them using run, module handle, ID kind, and local value |
| Source and stub are distinct | Add a supported `STUB_FOR` relationship rather than collapsing them |
| Source coordinates need normalization | Preserve exact UTF-8 parser input and explicit notebook/source mappings |
| Analyzer disagreement is evidence | Retain competing assertions instead of silently choosing the last one |
| Missing output is not negative evidence | Record unavailable, unrequested, failed, and unresolved separately |

Coverage should therefore be recorded by fact family and module/function, with statuses such as:

```text
complete_under_stated_model
partial
not_requested
unavailable
failed
```

“Complete under a stated model” is deliberately narrower than “complete knowledge of every possible Python execution.”

---

# 5. How this fits your Rust architecture

I recommend the following boundary:

```text
Pinned Ruff Rust adapter
    syntax + tokens + lexical semantic facts
                    \
                     → Arrow batches / Arrow IPC
                    /
Pinned Pyrefly Rust adapter
    Glean + Pysa + CinderX + native types/answers
                              ↓
                  Rust canonical normalizer
                              ↓
          DataFusion joins, validation, projections
                              ↓
                    Delta fact tables
                              ↓
                 Typed graph projections
                              ↓
             petgraph / graph-analysis routines
                              ↓
          Derived facts with evidence and lineage
```

Two separate Rust processes are a reasonable design, not a retreat from a Rust-native system. They isolate fast-changing analyzer internals from your Arrow/DataFusion/Delta dependency graph and avoid assuming that identically numbered Ruff crate dependencies are interchangeable with arbitrary Git revisions.

For storage, the specification uses:

| Logical value | Canonical persisted representation |
|---|---|
| IDs | Binary with a 16-byte length invariant |
| Digests | Binary with a 32-byte length invariant |
| Offsets and ordinals | Signed `Int64` |
| Enums | UTF-8 strings validated against versioned enumerations |
| Relationships | Flat typed edge/fact tables |
| Nested native details | Typed structural records |

Arrow can use fixed-size binary IDs or dictionary-encoded enums in memory, but the persisted mapping should be explicit. Delta’s primitive type surface includes binary and signed integer types rather than every Arrow-specific physical type. :chatgpt-content-reference{index="56"}

Use DataFusion for relational operations and selecting the relevant graph projection. Use petgraph for graph algorithms such as strongly connected components, traversal, topological processing, and dominance where appropriate. Those algorithms are useful building blocks; they do not supply Python evaluation semantics. :chatgpt-content-reference{index="57"}

I would keep these projections separate:

```text
module dependency graph
source invocation graph
potential-call graph
type structure graph
type-inference dependency graph
execution control-flow graph
value/dependence graph
```

Finally, publish an immutable snapshot manifest containing the exact Delta version of each constituent table. Readers should not independently choose “latest” for every table and accidentally assemble an inconsistent graph.

---

# 6. Remaining gaps and whether they justify more Rust libraries

## 6.1 Gaps that do not justify another parser or type checker

| Gap | Required work |
|---|---|
| Ruff semantic model is not directly exported | Add a narrow extraction hook after complete semantic traversal |
| Native Pyrefly types exceed report fidelity | Implement exhaustive structural native-type export |
| Some native class/solver facts are not in reports | Export retained answers and selected query results |
| Provider identities and coordinate systems differ | Build canonical identity/source mapping |
| Glean, Pysa, and CinderX overlap imperfectly | Preserve source-specific assertions and normalize them |
| General actual-to-formal argument binding is missing | Add candidate-signature-aware binding extraction/analysis |

These are primarily adapter and normalization gaps. Adding tree-sitter, another parser, or a second type checker would not inherently fix them.

## 6.2 Gaps requiring genuinely new semantic analysis

### Full control flow

You need explicit handling of:

```text
branches and short-circuit expressions
loops and loop else
break / continue / return / raise
try / except / finally
with entry, exit and exception suppression
match patterns and guards
definition-time defaults and decorators
comprehension execution
generator and coroutine suspension/resumption
```

Ruff provides the source structure to build this, but its inspected CFG implementation does not already supply that lowering. 

### Value flow and program dependence

You then need a declared abstraction for:

```text
reaching definitions
assignment/value dependencies
actual-to-formal transfer
return-to-call transfer
attribute and subscript locations
heap aliasing
mutation and invalidation
control dependence
exceptional completion
```

For example:

```python
a = []
b = a
b.append(1)
```

The lexical bindings and types are useful, but they do not by themselves constitute an alias-aware explanation of the mutation visible through `a`.

These analyses can be implemented over Ruff/Pyrefly facts in Rust. Whether an additional graph/dataflow library is worthwhile should be decided against those specific algorithmic requirements.

### Boundaries of the analyzed source universe

Native-extension implementation bodies, unavailable dependencies, dynamically manufactured attributes, arbitrary monkeypatching, and runtime-generated code need explicit boundary or model facts. Another Python parser cannot manufacture missing native-body semantics.

For library capability analysis, this means distinguishing:

```text
source implementation available
stub/API contract available
analyzer model available
external/native behavior unresolved
```

That distinction will be much more useful to agents than an apparently complete graph that silently omits unsupported behavior.

---

## Recommendation

**Adopt Ruff and Pyrefly as the initial fact-producing front ends. Do not add another parser or type checker at this stage.**

Build the first graph around **Ruff’s complete represented syntax and lexical semantics, Pyrefly’s Glean/Pysa/CinderX reports, and a loss-preserving native Pyrefly type-and-answer adapter**. Keep provenance, type roles, uncertainty, and provider projections first-class.

Then add a separately validated **Python execution-lowering and dependence layer** using your Rust graph stack.

The central architectural conclusion is:

> **The missing pieces are predominantly exporters, normalization, and execution analyses, not another source of Python syntax or another general-purpose type checker.**

That gives you a strong, defensible starting point for exceptional agent insight without prematurely expanding the library stack.

**Yes. I would make typed Arrow schemas the authoritative data contract, use DataFusion to construct most of the graph’s nodes and edges, and use petgraph only for analyses that require traversing or iterating over graph topology.**

The important distinction is:

> **A relationship does not require a graph algorithm merely because it has a source and destination.**  
> Creating `AST_CHILD`, `READS_BINDING`, `CALL_TARGET`, or `TYPE_COMPONENT` is primarily extraction, identity resolution, and relational transformation. Computing recursive call groups, dominance, or transitive reachability requires graph analysis.

There is also a necessary fourth component: **custom Rust semantic passes**. Neither Arrow, DataFusion, nor petgraph supplies Python evaluation semantics, type canonicalization, argument binding, or alias-analysis transfer functions.

I have translated the previous ontology into a reference package with **66 explicit Arrow table schemas**, including the original 53 tables plus normalization, graph-projection, and algorithm-result tables. It also includes typed category registries, physical storage conversions, DataFusion construction examples, and an Arrow → petgraph → Arrow bridge.

**:chatgpt-content-reference{index="22"}**  
:chatgpt-content-reference{index="23"} · :chatgpt-content-reference{index="24"}

The schema and registry documents passed consistency checks. **The Rust code has not been compiled, and the DataFusion plans and Delta round trips have not been executed.** This is a concrete implementation specification and reference code, not a completed analyzer pipeline.

---

# 1. The division of responsibility

I recommend the following architecture:

```text
Ruff / Pyrefly native structures and structured reports
                         │
                         ▼
          Version-pinned Rust extraction adapters
                         │
             Typed Arrow RecordBatches
                         │
                         ▼
          DataFusion normalization and construction
                         │
             Canonical typed fact tables
                         │
              ┌──────────┴───────────┐
              ▼                      ▼
       Delta persistence      DataFusion graph projection
                                     │
                        Compact vertex/edge Arrow tables
                                     │
                                     ▼
                       petgraph + custom Rust analyses
                                     │
                         Typed Arrow result batches
                                     │
                                     ▼
                    DataFusion identity/provenance joins
                                     │
                                     ▼
                         Derived facts in Delta
```

The previous schema already separates entity identity, factual assertions, provenance, and graph projections. I would preserve those distinctions while strengthening the physical Arrow contract. :chatgpt-content-reference{index="0"}

## Recommended ownership

| Work | Primary owner | Why |
|---|---|---|
| Walk Ruff syntax and native semantic structures | Rust extraction adapter | The provider already represents the structure and its meaning |
| Decode Pysa/Glean/CinderX reports | Typed Rust decoder | Interpret the provider’s exact schema once |
| Construct arrays, validity masks, offsets, and batches | Arrow | This is the typed data representation |
| Map provider-local identifiers to canonical identifiers | Rust identity logic + DataFusion joins | Identity rules are semantic; joining verified mappings is relational |
| Construct syntax, binding, type, class, and call-target edges | DataFusion over adapter-produced records | These are predominantly finite transformations and joins |
| Validate keys, foreign references, categories, and endpoint kinds | Arrow/Rust validators + DataFusion | Local array checks and cross-table checks have different scopes |
| Select a particular graph’s vertices and edges | DataFusion | Filter, project, join, and normalize before allocating a graph |
| Compute SCCs, reachability, topological schedules, and dominators | petgraph | These depend on topology and paths |
| Construct a complete Python CFG | Custom Rust semantic pass | Python evaluation and completion rules must be implemented |
| Compute reaching definitions, aliasing, or effect summaries | Custom Rust solver using graph adjacency | Requires transfer functions and an explicit abstraction |
| Persist and publish consistent snapshots | Delta + Rust orchestration | Persistence is separate from semantic and graph computation |

**A DataFusion execution plan is the plan for constructing your CPG. It is not the Python program’s control-flow graph.**

---

# 2. Make Arrow schemas authoritative

## 2.1 Separate the logical ontology from physical representation

I recommend one versioned `cpg_schema` Rust component containing:

```text
Arrow SchemaRef definitions
Rust category enums and fixed codebooks
Table-specific batch builders
Logical ID newtypes
Primary-key and foreign-key declarations
Allowed edge endpoint kinds
Validation rules
Physical storage mappings
```

The authoritative definitions should be Rust `Field`, `DataType`, and `Schema` values, not schemas inferred from JSON records or the first observed batch. Arrow’s `RecordBatch` explicitly associates a schema with equal-length typed arrays; its construction APIs support supplying that schema directly. :chatgpt-content-reference{index="1"}

### Proposed physical profiles

| Logical value | Computation Arrow type | Delta-input Arrow type | Additional invariant |
|---|---|---|---|
| Node, fact, run, snapshot ID | `FixedSizeBinary(16)` | `Binary` | Exactly 16 bytes |
| Content/schema digest | `FixedSizeBinary(32)` | `Binary` | Exactly 32 bytes |
| Closed categorical domain | `Int16` | `Int16` | Valid code in a versioned registry |
| Byte offset, ordinal, count | `Int64` | `Int64` | Appropriate range/nonnegative checks |
| Graph-local dense index | `UInt32` | `Int64` | Bounded and projection-local |
| Ordinary text | `Utf8` | `Utf8` | Field-specific validation |
| Optional factual flag | Nullable `Boolean` | Nullable `Boolean` | Unknown remains distinct from false |
| Timestamp | `Timestamp(Microsecond, UTC)` | Same | UTC semantics |

This deliberately strengthens the previous proposal, which used `Binary` IDs and `Utf8` categories as its default representation. It should therefore be treated as a **versioned physical migration**, not an implicit schema merge. :chatgpt-content-reference{index="2"}

Arrow supports fixed-size binary, signed and unsigned integers, nested types, and dictionary encodings. Delta’s logical storage types do not mirror every Arrow physical type, so the storage boundary should be explicit rather than relying on incidental writer coercions. :chatgpt-content-reference{index="3"}

### Why explicit category codes rather than arbitrary strings?

For stable domains such as `node_kind`, `edge_kind`, `type_role`, and `resolution_status`, use a fixed registry and Rust enums:

```rust
#[repr(i16)]
pub enum NodeKind {
    Artifact = 0,
    SourceFile = 1,
    Module = 2,
    SyntaxNode = 3,
    // Remaining assignments are explicitly registered.
}
```

The exact numbers are not semantically important; their **stability** is.

Keep the codebook append-only. Never regenerate codes from an alphabetically sorted list after adding a new variant. Keep provider-specific, open-ended names as `Utf8` until you deliberately define a stable domain for them.

An Arrow dictionary is a physical encoding, not your semantic enum registry. I would not use batch-local dictionary keys as persistent category identities.

---

## 2.2 Typed Arrow does not eliminate semantic validation

`FixedSizeBinary(16)` distinguishes a 16-byte value from a string. It does not distinguish a `NodeId` from a `FactId`.

I would therefore use both:

```rust
pub struct NodeId(pub [u8; 16]);
pub struct FactId(pub [u8; 16]);

pub struct TypedBatch<T: TableSpec> {
    // Validated RecordBatch plus a table-specific marker.
}
```

The package includes this pattern.

Use field metadata for declarations such as:

```text
cpg.logical_type = node_id
cpg.references = nodes.node_id
cpg.enum_domain = type_role
cpg.enum_version = 1
```

But treat metadata as a declaration to validate, not as an enforcement mechanism. At every materialization boundary, check the actual arrays, restore the authoritative schema metadata where necessary, and then validate the table contract.

**Do not assume metadata alone gives Arrow database foreign keys or domain-specific type safety.**

---

## 2.3 Core Arrow tables

The core fact tables should remain normalized and explicitly typed. For example:

```text
nodes
  snapshot_id         FixedSizeBinary(16)  not null
  node_id             FixedSizeBinary(16)  not null
  node_kind           Int16               not null
  existence_fact_id   FixedSizeBinary(16)  not null

edges
  snapshot_id         FixedSizeBinary(16)  not null
  fact_id             FixedSizeBinary(16)  not null
  src_id              FixedSizeBinary(16)  not null
  edge_kind           Int16               not null
  dst_id              FixedSizeBinary(16)  not null
  role                Utf8                nullable
  ordinal             Int64               nullable
  resolution_id       FixedSizeBinary(16)  nullable
  guard_id            FixedSizeBinary(16)  nullable

type_observations
  snapshot_id         FixedSizeBinary(16)  not null
  fact_id             FixedSizeBinary(16)  not null
  subject_id          FixedSizeBinary(16)  not null
  type_id             FixedSizeBinary(16)  not null
  type_role           Int16               not null
  program_point_id    FixedSizeBinary(16)  nullable
  requested_api       Utf8                nullable
```

Each factual assertion joins to `facts`, which retains its analysis run, origin, extraction mode, modality, fidelity, and model.

This prevents “the type of this variable” from becoming a single mutable property. Annotation, computed, narrowed, expected, contextual, decorated, and undecorated observations remain separate.

### Keep specialized tables

I would retain dedicated Arrow tables for:

```text
bindings and references
functions, signatures, parameters, and type parameters
types and type observations
classes and members
calls, arguments, resolutions, and resolution issues
control points, accesses, and dataflow details
```

A universal property bag would be a poor default for your objective. Agents should not have to reconstruct every signature from string-keyed generic properties.

Keep `record_fields` as a **loss-preserving fallback** for native details that do not yet have dedicated columns. That was its intended role in the prior schema. :chatgpt-content-reference{index="4"}

---

## 2.4 Use nested Arrow selectively

Nested Arrow is useful for shallow ingress records and agent-facing result bundles. For example:

```text
arguments: List<Struct<
    ordinal: Int64 not null,
    kind: Int16 not null,
    keyword_name: Utf8 nullable,
    expression_local_key: Utf8 not null
>>
```

However, for canonical storage, I would normally emit an `arguments` child table directly.

This has three advantages: argument relationships are independently queryable; evidence can attach to each occurrence; and you avoid repeatedly expanding nested arrays.

When expanding nested inputs, preserve ordinals before expansion and use one struct per element rather than independent parallel lists. DataFusion provides `unnest_columns` and configurable unnest behavior for list/struct expansion. :chatgpt-content-reference{index="5"}

**Recursive Python types should be represented as graph references, not recursively nested Arrow schemas.** A recursive alias needs a `Type` entity and component/back-reference relationships. A self-referential Arrow `Struct` is not the right representation.

---

# 3. Construct the canonical graph with Arrow and DataFusion

## Stage A: Establish the source and analysis universe

**Owner: Rust + Arrow, followed by DataFusion validation.**

Before joining semantic facts, emit:

```text
source_files
source_maps
modules
contexts
producers
runs
raw_records
```

The Rust adapter computes exact source hashes, parser-input identities, coordinate mappings, and configuration identities. Preserve ordered import/search paths in the environment manifest.

Then DataFusion checks uniqueness and joins module references to the correct source/context records.

The original identity rules explicitly prohibit merging solely on qualified names, byte ranges, or provider-local IDs. They also distinguish source nodes, semantic symbols, bindings, references, types, and execution points. :chatgpt-content-reference{index="6"}

No petgraph work is required here.

---

## Stage B: Emit typed provider facts

**Owner: native adapters + Arrow builders.**

Use two extraction paths:

| Input | Recommended ingestion |
|---|---|
| Ruff AST and retained semantic structures | Native Rust traversal directly into Arrow builders |
| Pyrefly native types, bindings, answers | Native Rust adapter directly into Arrow builders |
| Glean/Pysa/CinderX JSON or binary reports | Decode once into versioned provider structs, then Arrow |

Do not insert a generic “JSON → inferred DataFrame schema” step into the canonical pipeline.

Also, interpret omitted report fields according to that report’s schema. A field omitted because it equals a provider-defined default is not generically equivalent to an unknown value.

The output should already preserve structural child roles, ordinals, provider key namespaces, type-observation roles, synthetic-model distinctions, and unresolved-remainder information. These are extraction responsibilities, not graph algorithms.

### Arrow operations at this stage

| Operation | Appropriate use |
|---|---|
| Typed builders | Append primitive, binary, string, list, and struct values |
| `RecordBatch::try_new` | Assemble arrays against an explicit schema |
| Projection/slicing | Retain relevant columns or bounded batch portions |
| Filter kernels | Apply already-computed masks |
| `take` | Gather selected rows using an index array |
| Concatenation | Coalesce compatible arrays/batches |
| Explicit conversion | Change physical representation at a declared boundary |

Arrow’s selection-kernel crates provide the filter, take, concat, interleave, and related operations. Use these for array manipulation rather than hand-building row objects for every downstream transformation. 

Use bounded batches and backpressure. Raw source text and large docstrings should not be copied into every graph-processing batch.

---

## Stage C: Resolve provider-local identities

**Owner: Rust identity logic + DataFusion joins.**

Introduce:

```text
provider_node_map(
    snapshot_id,
    run_id,
    module_key,
    provider_kind,
    provider_local_key,
    node_id
)
```

The composite provider key is essential. Two modules can have the same local class index; two providers can use identical numeric IDs for entirely different objects.

The construction sequence should be:

1. Rust establishes candidate canonical identities using the declared identity rules.
2. DataFusion verifies that accepted mapping keys are unique.
3. DataFusion left-joins source and destination mappings onto staged relationships.
4. Fully mapped rows become canonical relationships.
5. Unmapped or ambiguous rows remain resolution/coverage issues.

The relevant validation pattern is:

```sql
SELECT
    snapshot_id, run_id, module_key,
    provider_kind, provider_local_key,
    COUNT(*) AS n
FROM provider_node_map
GROUP BY
    snapshot_id, run_id, module_key,
    provider_kind, provider_local_key
HAVING COUNT(*) <> 1;
```

This must return no rows before endpoint mapping.

**Do not use inner joins to silently discard unresolved targets and then report the remaining graph as complete.**

Deterministic identifier construction itself remains custom Rust logic. Use a documented, versioned, length-delimited encoding and collision checks. Arrow’s row-comparison format is useful for sorting/grouping, but I would not make it your durable cross-version identity encoding. Its documentation specifically frames it as a comparable row representation tied to a `RowConverter`. 

---

## Stage D: Construct the semantic relationships relationally

**Owner: DataFusion.**

Most initial CPG relationships should be constructed through the following recipes:

| Relationship | Construction |
|---|---|
| `AST_CHILD` | Child record + mapped parent and child IDs |
| `LEXICAL_PARENT` | Scope record + mapped enclosing scope |
| `DECLARES`, `BINDS_SYMBOL` | Declaration/binding records + canonical entity mappings |
| `READS_BINDING` | Reference assertion + resolved binding |
| `IMPORTS_MODULE`, `IMPORTS_SYMBOL` | Import record + accepted resolution |
| `HAS_SIGNATURE`, `HAS_PARAMETER` | Signature/parameter child records |
| `PARAMETER_TYPE`, `RETURN_TYPE` | Signature slots + normalized type IDs |
| `TYPE_COMPONENT` | Native type component + role/ordinal + type mapping |
| `BASE_CLASS`, `MRO_ENTRY` | Reported class relationships, preserving ordering |
| `CALL_TARGET`, `INIT_TARGET`, `NEW_TARGET` | Call resolution + candidate target |
| `HAS_TYPE` | A view/materialization of typed observations, preserving their role |
| `HAS_DOCSTRING`, diagnostic links | Anchored records + subject mappings |

DataFusion’s relevant operations are projections, filters, equijoins, unions, aggregates, distinct operations, window operations, and repartitioning. Its DataFrame methods build lazy logical plans that are subsequently executed. :chatgpt-content-reference{index="9"}

Two important design rules:

**First, preserve independent assertions.** Two providers supporting the same relationship are not necessarily duplicate records. Deduplicate repeated ingestion of the same assertion, not independent evidence or disagreement.

**Second, distinguish canonical facts from convenience graph views.** For example, a `HAS_TYPE` edge derived from `type_observations` should not become a separately mutable competing source of truth.

### Type and class normalization need semantic care

Preserve Pyrefly’s reported MRO ordering. Do not replace it with a topological ordering of inheritance edges.

Similarly, type assignability is not simply graph reachability. Structural type interning, binder identity, recursive aliases, and semantic compatibility need dedicated Rust/provider logic. Computing SCCs in a type graph can identify recursion; it does not prove that two recursive types are equivalent.

---

## Stage E: Construct the caller-to-callee projection

This example illustrates the boundary particularly well.

The base graph contains:

```text
CallSite ──CALLER─────→ EnclosingCallable
CallSite ──CALL_TARGET→ CandidateCallee
```

The caller-to-callee projection is a **join**, not a petgraph algorithm:

```sql
SELECT
    owner.snapshot_id,
    owner.dst_id  AS caller_id,
    target.dst_id AS callee_id,
    target.src_id AS call_site_id,
    owner.fact_id AS ownership_fact_id,
    target.fact_id AS target_fact_id
FROM eligible_edges owner
JOIN eligible_edges target
  ON owner.snapshot_id = target.snapshot_id
 AND owner.src_id = target.src_id
WHERE owner.edge_name = 'CALLER'
  AND target.edge_name IN (
      'CALL_TARGET',
      'INIT_TARGET',
      'NEW_TARGET'
  );
```

Here, `eligible_edges` is a snapshot-, context-, and policy-filtered view that joins numeric edge codes to their labels. Production Rust expressions can use the code constants directly.

The projection must exclude `POTENTIAL_CALL_TARGET` and model-only relationships when constructing a source-invocation graph. The previous audit explicitly distinguishes Pysa’s `ifCalled` possibilities and synthetic shims from source invocations. :chatgpt-content-reference{index="10"}

Also, do not determine evaluation ownership merely from the nearest enclosing textual function. Defaults and decorators need ownership assigned by the semantic traversal.

---

# 4. The Arrow → petgraph boundary

## 4.1 Construct a small, explicitly defined projection

Do not load the full ontology into one petgraph instance.

Instead, identify the projection precisely:

```text
snapshot and environment
included node kinds
included edge kinds
accepted assertion origins/fidelities
candidate-target policy
unknown-target policy
entry/exit policy, where applicable
analysis algorithm and model version
```

Examples include module dependencies, source calls, potential calls, type-component recursion, inference dependencies, and execution control flow. They should not be mixed merely because they share the same base tables.

### Projection schemas

The package adds these Arrow contracts:

```text
projection_nodes
  snapshot_id    FixedSizeBinary(16)
  projection_id  FixedSizeBinary(16)
  dense_index    UInt32
  node_id        FixedSizeBinary(16)

projection_arcs
  snapshot_id    FixedSizeBinary(16)
  projection_id  FixedSizeBinary(16)
  arc_index      Int64
  src_index      UInt32
  dst_index      UInt32
  arc_kind       Utf8

projection_arc_facts
  snapshot_id    FixedSizeBinary(16)
  projection_id  FixedSizeBinary(16)
  arc_index      Int64
  fact_id        FixedSizeBinary(16)
```

The first two are topology. The third preserves the evidence behind topology.

### Build vertices independently of edges

Select the eligible vertex universe separately, then add permitted external endpoints. Otherwise, isolated functions, modules, and CFG points disappear.

Next, assign a deterministic dense index inside the projection and join that mapping to both endpoints of every arc.

For a bounded projection, this can be a sorted canonical-ID list followed by checked enumeration. Do not depend on arbitrary DataFusion partition arrival order.

**Dense indices are temporary graph coordinates, not persistent entity identities.**

---

## 4.2 Use compact graph weights

My default would be:

```rust
DiGraph<(), i64>
```

The graph stores topology and an arc index. Rich properties, canonical IDs, and provenance stay in adjacent Arrow tables.

`petgraph::Graph` supports parallel edges and compact node indices. `GraphMap` does not support parallel edges, so it is not an appropriate default for a property graph whose multiple assertions or call sites may connect the same endpoints. :chatgpt-content-reference{index="11"}

For SCC or reachability, you may deliberately collapse parallel topology arcs. But preserve the many-to-many mapping from each collapsed arc to all supporting facts.

An immutable graph per analysis job is a good initial design. `StableGraph`, CSR structures, or custom Arrow-backed graph traits can be considered later where measurements justify them.

**Do not assume this conversion is zero-copy.** Conventional petgraph construction allocates its own graph representation. Keep projections bounded and give the graph worker an explicit memory budget.

---

# 5. What petgraph should actually compute

## 5.1 Native topology algorithms

| Desired output | petgraph operation | Additional work |
|---|---|---|
| Mutually recursive call/module/type groups | `kosaraju_scc` or `tarjan_scc` | Define the input projection and uncertainty policy |
| Dependency processing order | `toposort` over an SCC-condensed graph | Construct condensation and preserve provenance |
| Reachability or bounded impact neighborhood | `Dfs`, `Bfs`, reversed traversal | Define direction, roots, bounds, and eligible edges |
| Immediate dominators | `dominators::simple_fast` | Supply a valid CFG and entry |
| Postdominators | Dominance on a reversed, explicitly completed CFG | Define exits and nonterminating regions |

These are native graph-analysis capabilities, not new Python semantic analysis. The dominator implementation takes a graph and root; it does not construct the CFG for you. :chatgpt-content-reference{index="12"}

I would normally persist compact results such as SCC membership and immediate-dominator trees, rather than dense all-pairs reachability or all dominance pairs.

---

## 5.2 SCC condensation is a useful hybrid operation

The ownership split is:

```text
petgraph: compute SCC memberships
DataFusion: join memberships to both arc endpoints
DataFusion: remove intra-component arcs
DataFusion: deduplicate inter-component topology
petgraph: topologically order the resulting DAG
```

The DataFusion portion is:

```sql
SELECT DISTINCT
    a.snapshot_id,
    a.projection_id,
    s.component_index AS src_component,
    d.component_index AS dst_component
FROM projection_arcs a
JOIN scc_results s
  ON a.snapshot_id = s.snapshot_id
 AND a.projection_id = s.projection_id
 AND a.src_index = s.dense_index
JOIN scc_results d
  ON a.snapshot_id = d.snapshot_id
 AND a.projection_id = d.projection_id
 AND a.dst_index = d.dense_index
WHERE s.component_index <> d.component_index;
```

Preserve isolated components independently. Also retain condensed-arc lineage when the condensed graph is published as an artifact.

This is a good example of using graph algorithms only where needed, while keeping construction and evidence handling relational.

---

## 5.3 Return graph results to Arrow immediately

For SCCs:

```text
scc_results(
    snapshot_id,
    projection_id,
    dense_index: UInt32,
    component_index: Int64
)
```

For dominance:

```text
dominance_results(
    snapshot_id,
    projection_id,
    dense_index: UInt32,
    idom_index: UInt32?,
    reachable: Boolean
)
```

The explicit `reachable` flag matters because a root with no immediate dominator and an unreachable vertex should not be conflated.

DataFusion then joins these results to `projection_nodes`, restoring canonical IDs. Rust attaches analysis-run identity, derived fact IDs, and input provenance before persistence.

For dominance and other absence-sensitive properties, retain the **complete input projection manifest**. A few witness edges do not establish that no alternative path existed.

---

# 6. What requires custom Rust analysis rather than merely petgraph

## 6.1 Full Python control flow

A complete CFG should be built by a semantic lowering pass that emits typed Arrow tables for control points and transfers.

It must model branches, short-circuit expressions, loops and loop `else`, `break`/`continue`, exception handlers, `finally`, context-manager cleanup and suppression, match guards, definition-time expressions, and coroutine/generator suspension.

The prior audit found that Ruff’s inspected CFG builder does not implement those constructs comprehensively. That remains a gap in semantic lowering, not a missing Arrow or petgraph operation. :chatgpt-content-reference{index="13"}

The correct sequence is:

```text
Ruff/Pyrefly source and semantic facts
    → custom Python evaluation lowering
    → Arrow control_points / CFG_NEXT facts
    → DataFusion validation and projection
    → petgraph dominance and reachability
```

**Sorting syntax nodes by byte offset and connecting consecutive nodes is not a valid substitute.**

---

## 6.2 Reaching definitions and value flow

DataFusion can assemble analysis inputs: definitions, reads, writes, locations, CFG ownership, and transfer specifications.

The iterative solver should be custom Rust using graph adjacency and a stated abstract domain. For a simple reaching-definitions formulation:

\[
IN[b] = \bigcup_{p \in predecessors(b)} OUT[p]
\]

\[
OUT[b] = GEN[b] \cup (IN[b] \setminus KILL[b])
\]

The division is:

| Component | Responsibility |
|---|---|
| DataFusion | Assemble input relations, map identities, validate, normalize results |
| petgraph | Adjacency, predecessor/successor traversal, SCC-based scheduling |
| Custom Rust solver | Transfer functions, worklist iteration, termination, statement ordering |
| Arrow | Typed input and output batches |
| Delta | Persist the resulting assertions and their model/provenance |

Attribute/subscript writes need an explicit heap abstraction. Strong versus weak updates, unknown calls, and alias widening are solver decisions.

Do not rename Pyrefly inference dependencies or type-join nodes into runtime dataflow/SSA merely because their structures look similar. The previous schema explicitly separates those layers. :chatgpt-content-reference{index="14"}

### Interprocedural summaries follow the same pattern

Use a call projection to identify recursive SCCs, then iterate summaries within those groups. Propagate unknown effects from unresolved calls rather than treating the known target list as exhaustive.

Petgraph supplies scheduling primitives. The effect-summary semantics remain your Rust analysis.

---

# 7. Execution, validation, and Delta persistence

## 7.1 DataFusion should own substantial relational execution

Use DataFusion for joins, aggregates, anti-joins, projection preparation, and final result normalization rather than manually rebuilding these operations with per-row hash maps.

For small bounded inputs, register validated batches through `MemTable`. For persisted datasets, use an appropriate table provider. DataFusion’s provider interface supports projection and filter pushdown, which allows graph jobs to read only the relevant rows and columns. :chatgpt-content-reference{index="15"}

Use `execute_stream` or partitioned streaming at materialization boundaries rather than indiscriminately calling `collect()` on library-wide datasets. Streaming yields batches incrementally; it should not be interpreted as a guarantee that every operator or downstream graph job has constant memory. :chatgpt-content-reference{index="16"}

Avoid running Pyrefly or a whole graph algorithm inside a scalar UDF invoked once per row. A graph-analysis job belongs after an explicit projection/materialization boundary.

For ordinary counts, fan-in/fan-out, coverage, and grouped statistics, DataFusion is sufficient. Do not allocate a graph merely to count grouped endpoints.

---

## 7.2 Validation needs two levels

**Local Arrow/Rust validation** should check exact physical types, nullability, widths, code membership, array consistency, and applicable numeric bounds.

**Cross-table DataFusion validation** should check composite-key uniqueness, foreign references, permitted source/destination kinds, resolution completeness fields, source anchors, and projection integrity.

For example:

```sql
SELECT e.snapshot_id, e.fact_id, e.src_id
FROM edges e
LEFT ANTI JOIN nodes n
  ON e.snapshot_id = n.snapshot_id
 AND e.src_id = n.node_id;
```

This must return no rows for a published canonical edge table.

Additional fixtures should cover same-range syntax nodes, non-ASCII source coordinates, distinct generic binders named `T`, known-plus-unknown call targets, isolated vertices, parallel arcs, and dominance under exceptional/nonterminating control flow.

---

## 7.3 Delta is a separate physical boundary

Before writing:

```text
FixedSizeBinary(16/32) → Binary, with width checks
UInt32 graph indices → checked Int64
Int16 categories → unchanged, with versioned codebook
Authoritative schema metadata → registry/manifest representation
```

On read, perform checked conversion back to the computation profile.

Use Delta-aware reads for Delta tables, not a raw scan of every Parquet file in their directories. The table state is determined by the transaction log, not simply by which files are present. Delta’s transaction semantics are table-scoped. :chatgpt-content-reference{index="17"}

For the multi-table CPG, publish a manifest containing the exact version of every table only after all outputs pass validation. Readers should use that manifest rather than independently selecting each table’s latest version.

The 66 schemas do not imply 66 tables per module. Batch facts across modules into relation-family datasets; avoid creating thousands of tiny module-specific Delta tables.

---

## 7.4 Pin a coherent Rust dependency family

One compatibility issue deserves explicit attention.

The inspected **DataFusion `55.0.0` tag specifies Arrow `59.2.0`**, whereas the inspected DataFusion main branch specifies Arrow `60.0.0`. The inspected Delta workspace uses Arrow `59` and DataFusion `55`. These should not be combined merely by selecting independently current branches.   

The reference package consequently targets DataFusion `55.0.0`, Arrow `59.2.0`, and petgraph `0.8.3`. It intentionally omits an unverified Delta writer pin. The eventual workspace needs one verified dependency/feature combination and a committed lockfile.

---

# Recommended implementation sequence

I would build the system in three increments.

**First, implement the typed fact substrate:** Arrow schemas, category registries, native/report adapters, provider-to-canonical mappings, DataFusion construction plans, validation, and Delta publication. This produces the source/symbol/type/call-resolution graph without pretending to have execution dependence.

**Second, add graph projections and native topology analyses:** source-call and module-dependency projections, SCCs, condensed scheduling graphs, bounded reachability, and typed result normalization.

**Third, add execution semantics:** complete Python CFG lowering, dominance/postdominance under explicit policies, and then reaching definitions, aliasing, and interprocedural summaries.

The resulting boundary is:

> **Arrow defines the typed facts. DataFusion constructs and validates the relational graph representation. Petgraph analyzes selected topology. Custom Rust passes supply the Python semantics and fixed-point analyses that none of those libraries provides automatically.**

That keeps your fact store richly typed and queryable while making the graph-analysis layer a reproducible consumer and producer of facts, rather than a separate opaque database.

**I would narrow the first implementation to two agent-facing functions: “find the relevant built-in capability” and “show me how to use it correctly.”** Behind those functions, I would retain a mandatory analytics pipeline that derives useful relationships from the CPG and turns them into evidence-backed capability briefs.

The key simplification is:

> **Precompute a small collection of deeply interpreted capability briefs, rather than build a system that reasons over arbitrary library questions or synthesizes new compositions at query time.**

That preserves the difficult and valuable part of your objective: translating code structure into guidance that changes how an agent implements a task. It defers the general-purpose ontology discovery, composition planning, and constraint-solving machinery.

## 1. The first product: capability discovery and usage briefs

### Two queries, with a deliberately limited promise

| Query | What it does | What it does not promise |
|---|---|---|
| `search_capabilities(library, query, limit)` | Finds published capability briefs relevant to a task, including briefs whose API names differ from the task’s wording | Exhaustive discovery of everything the library could possibly do |
| `get_capability(snapshot_id, capability_id)` | Returns the public entry point, relevant controls, supported usage pattern, conditions, limitations, and evidence | A newly synthesized solution for arbitrary combinations of requirements |

For example:

```text
search_capabilities(
    library="pyarrow",
    query="rewrite a large dataset with different partitioning"
)

get_capability(
    snapshot_id=<snapshot returned by search>,
    capability_id=<selected result>
)
```

The search result resolves the library’s active release to an immutable snapshot. Subsequent inspection uses that same snapshot. This is evidence consistency, not historical-analysis functionality.

**The programming agent remains responsible for adapting the returned pattern to its task.** Your system makes the relevant mechanisms and conditions hard to overlook; it does not initially attempt to replace the agent’s planning.

### Scope one API family, not a whole library

I would start with one bounded subsystem and roughly **15–25 capability briefs**. That is a proposed pilot size, not a required architectural limit.

The PyArrow dataset scan/write path is a concrete candidate: the official guide demonstrates passing a scanner directly into `write_dataset`, and the Python implementation contains input normalization, configuration handling, and validation that can contribute code-grounded insights. :chatgpt-content-reference{index="0"}

For that pilot, native execution internals would remain an explicit evidence boundary. You would not add C++ or Cython analysis merely to complete the first briefs.

---

## 2. Make the capability brief the unit of interpreted output

A brief should cover **one useful outcome, anchored to a public operation, optionally including one supported producer-to-consumer handoff**.

It should not be a generic function summary, and it should not become a miniature encyclopedia of the subsystem.

I would give each brief this fixed structure:

| Section | Required content |
|---|---|
| **Outcome** | What this built-in mechanism lets the caller accomplish |
| **Public access** | The supported function, method, constructor, or configuration object to use |
| **Applicable input or mode** | The input forms and conditions to which the guidance applies |
| **Important controls** | The parameters that activate or materially change the behavior |
| **Usage pattern** | A short, supported example, including an adjacent API when needed |
| **Limits and prerequisites** | Important restrictions, lifecycle requirements, optional features, and unresolved assumptions |
| **Evidence** | The source facts, documentation, examples, and validation results supporting the claims |

A useful brief might explain:

> **Rewrite through a configured scanner.** Configure the input scan, then pass the scanner to the dataset writer. This uses the existing scanner-to-writer path rather than requiring a custom loop that dispatches batches to output partitions.

That interpretation is supported by the documented PyArrow composition. The brief should also attach the implementation restriction that a separate `schema` argument cannot be supplied when writing a scanner. :chatgpt-content-reference{index="1"}

### Preserve modes without building a general mode ontology

You still need conditional distinctions. However, v1 can represent them as a small number of named, evidence-backed cases:

```text
Writing from an in-memory table
Writing from an existing scanner
Writing from an iterable of batches
```

Do not enumerate every combination of configuration values. Create a separate case only when it changes the accepted interface, usage pattern, or important conditions.

**This is the central content boundary: deep guidance about a bounded mechanism, not unrestricted reasoning about the entire library.**

---

# 3. Keep three analytics passes mandatory

I would make the initial analytics deterministic and task-oriented, with LLM interpretation after the structural work.

You do not need unsupervised discovery to demonstrate meaningful graph analytics. The first passes should answer three specific questions:

> **Where is the built-in mechanism exposed? What activates or constrains it? What does it connect to directly?**

## Pass A: Public entry point and delegation analysis

**Question:** Which public API exposes the relevant internal mechanism, and what does it already coordinate?

### Inputs

Use public-export mappings, declarations, call sites, resolved targets, signatures, and directly associated documentation.

### Analysis

For each selected public entry point:

1. Map public access paths to implementing declarations.
2. Follow a bounded set of resolved delegation relationships.
3. Identify relevant configuration or helper objects used along those paths.
4. Preserve the path and its uncertainty rather than emitting only a list of reachable symbols.

Start with direct calls and a small traversal depth. Stop at unresolved calls, external/native boundaries, or the configured bound, and record that the neighborhood is incomplete.

### Interpreted output

The resulting brief can explain:

```text
Use this public entry point.

It already coordinates these mechanisms.

These lower-level helpers are implementation details,
not additional steps the caller normally needs to reproduce.
```

The last statement requires documentation or other supporting evidence about intended use. A call edge alone does not prove that an API is the recommended abstraction.

### Why it belongs in v1

This analysis directly supports your goal of avoiding unnecessary reconstruction of a library’s internal orchestration.

It also gives petgraph a real but bounded role. Its breadth-first traversal machinery can support neighborhood expansion; your code supplies the permitted edge kinds, depth limits, and stopping rules. :chatgpt-content-reference{index="2"}

---

## Pass B: Configuration and local restriction analysis

**Question:** Which controls expose the behavior, and which local conditions constrain their use?

### Inputs

Use parameters, defaults, name bindings, argument expressions, local branches, raises, and attached parameter documentation.

### Analysis

Initially support a restricted set of recognizable patterns:

```text
A parameter is forwarded directly to a downstream call.

A configuration object is constructed from named arguments.

A local branch selects an implementation or input-handling path.

A local guard raises for an unsupported argument combination.
```

For parameter forwarding, use binding identity and supported argument mappings, not matching variable names.

For guards, preserve the exact branch context. Do not turn arbitrary branch ancestry into a general control-dependence or precondition proof. Expressions involving complex mutation, aliasing, or unsupported control flow can remain unresolved.

### Interpreted output

```text
This option activates this documented behavior.

This public parameter is passed to the underlying mechanism.

This combination is rejected in this input mode.

Configure this object rather than implementing that behavior yourself.
```

A concrete example is PyArrow’s writer restriction for an already configured scanner. The source both selects the scanner’s projected schema and rejects a separately provided writer schema. That is a useful conditional insight assembled from related code facts, not merely a signature listing. :chatgpt-content-reference{index="3"}

### Important limit

Parameter propagation demonstrates that a value is passed onward. It does **not** establish the downstream behavior, memory bound, or performance guarantee by itself.

The interpreter must combine structural evidence with documentation, visible implementation, or a scoped test.

---

## Pass C: Direct handoff analysis

**Question:** Which nearby public APIs already work together without a custom adapter?

### Inputs

Use the CPGs of official examples and selected tests, supported local binding relationships, call arguments, signatures, and return-type information.

### Analysis

Recognize simple, bounded producer-to-consumer patterns:

```python
intermediate = producer(...)
consumer(intermediate, ...)
```

Start with direct expressions and unambiguous local bindings. Do not infer a handoff merely because two APIs occur in the same function or their types appear compatible.

Record:

```text
Producer public API
Produced expression or binding
Consumer public API
Consumer argument position/name
Relevant configuration
Example/test evidence
Unresolved conditions
```

A type-compatible connection can be retained as a candidate, but publication as a supported usage pattern should require stronger evidence, such as an official example, a relevant test, or a fixture you execute.

### Interpreted output

```text
Pass this object directly into that API.

This avoids an intermediate conversion or custom loop.

These conditions apply to the handoff.
```

For PyArrow, the official repartitioning example explicitly connects a configured scanner to `write_dataset`. The useful graph-derived output is the identified handoff and its supporting relationships, rather than merely retrieving the page containing both names. :chatgpt-content-reference{index="4"}

### Deliberately exclude general composition search

V1 would publish short patterns found and checked during ingestion. It would not discover arbitrary new chains in response to an agent’s request.

**That is a major complexity reduction while retaining the most immediately useful composition insights.**

---

# 4. Put an explicit interpretation stage after the analytics

The pipeline should be:

```text
Selected CPG facts + linked documentation/examples
                         ↓
Three bounded analytics passes
                         ↓
Structured findings with witness facts
                         ↓
LLM interpretation into capability assertions
                         ↓
Grounding checks and selected usage validation
                         ↓
Published capability briefs
                         ↓
Search and inspection queries
```

The crucial intermediate artifact is the **structured finding**.

For example:

```text
finding_kind:
    direct_handoff

producer:
    public API node

consumer:
    public API node

binding:
    the expression/name connecting them

conditions:
    source-linked conditions

witnesses:
    CPG facts and example spans
```

The LLM turns this into a useful explanation, but it does not invent the underlying relationship.

## Give the interpreter a constrained job

For each bounded evidence package, ask it to produce:

```text
The caller outcome
The public mechanism
The relevant configuration choices
The supported short usage pattern
The conditions and limitations
The source IDs supporting each assertion
```

Every important assertion should identify which evidence supports it.

Use a deterministic rendering template to assemble the final brief from these assertions. You can allow polished explanatory text, but it should remain attached to the structured claims.

## Separate grounding from truth verification

Some checks can be mechanical: the public symbol exists, the parameter exists, a cited branch is present, the witness path exists, and a snippet refers to real APIs.

Those checks do not prove every semantic claim. Preserve distinct evidence statuses such as:

```text
structurally_observed
documented
interpreted
fixture_checked
unresolved
```

A passing fixture supports the tested case, not universal correctness or performance.

For the small initial corpus, I would inspect the published assertions and patterns directly rather than build an elaborate automated review-agent hierarchy.

---

# 5. Keep the storage and execution design small

## Reuse the Arrow foundation, but implement only the required slice

Your existing construction plan already provides the right separation: Arrow for typed records, DataFusion for normalization and joins, and graph routines for path-dependent analysis. It also specifies an authoritative schema registry rather than inference from incoming records. :chatgpt-content-reference{index="5"} :chatgpt-content-reference{index="6"}

I would not make all 66 proposed tables, a complete execution CFG, or alias analysis prerequisites for the first enrichment result.

The initial dependency slice is:

```text
Public exports and declarations
Signatures and parameters
Names, bindings, references, and source anchors
Call sites and candidate targets
Selected type observations
Local syntax for supported guards and forwarding patterns
Documentation/example/test links
Provenance and coverage
```

Reuse additional CPG facts as they become available. Do not silently approximate missing facts merely to populate a brief.

## Add six enrichment table families

I would start with these additions rather than the full capability ontology from the previous proposal:

| Table | Purpose |
|---|---|
| `capability_briefs` | Outcome, primary public entry point, applicable case, publication status |
| `brief_members` | Supporting public symbols and their roles |
| `insights` | Delegation, configuration, restriction, and handoff findings plus their interpreted assertions |
| `insight_evidence` | Links to CPG facts, source spans, documents, and validation records |
| `usage_patterns` | Short examples, conditions, and validation status |
| `brief_embeddings` | One search representation per published brief |

Use your existing physical conventions: fixed-width computation IDs, explicit categorical domains, typed foreign references, and source-linked evidence.

For a core insight, the logical shape could be:

```text
insights
  snapshot_id          ID
  insight_id           ID
  brief_id             ID
  kind                 enum
  subject_node_id      ID
  related_node_id      ID?
  condition_text       Utf8?
  assertion_text       Utf8
  evidence_status      enum
  analysis_run_id      ID
```

Here, `ID` maps to the existing Arrow identifier type.

**Do not build a general condition-expression language in v1.** Preserve conditions as source-linked text and, where available, references to existing predicate nodes. That keeps the records typed without pretending every semantic condition has already been formalized.

## Execution ownership

DataFusion should assemble evidence packages, join identities, group findings, validate references, and construct serving records. Its DataFrame API supports operations over Arrow batches and incremental batch execution, which fits this materialization pipeline. :chatgpt-content-reference{index="7"}

Petgraph should handle bounded delegation neighborhoods and any topology-specific work actually required. Most direct handoff and parameter-mapping construction should remain relational or adapter-based.

Custom Rust code handles the supported local semantic patterns. The LLM supplies interpretation. Delta stores the authoritative records and published snapshot.

---

# 6. Simplify retrieval more aggressively than interpretation

I would retain **one embedding per capability brief**, plus exact lexical matching over titles, public symbols, parameter names, and brief text.

The embedding input can be a deterministic concatenation:

```text
Outcome
Applicable input/mode
Public API names
Important built-in controls
Usage-pattern description
Key limitations
```

For an initial corpus of a few dozen briefs, I would use exact vector scoring over the stored embeddings rather than make approximate nearest-neighbor indexing another prerequisite. An existing retrieval service can be reused, but it should not become a new implementation project.

At query time:

```text
Resolve active snapshot
    → lexical and vector candidate retrieval
    → combine candidate rankings
    → return published briefs
```

There is no whole-library graph traversal and no new capability generation during the request.

### Search does not certify task compatibility

A query containing “under 500 MB” should not cause the server to mark a brief as satisfying that memory limit merely because its description mentions batching.

Return relevant guidance with unresolved conditions intact. The caller agent decides what further validation is required.

When a brief is selected, attach its limitations through deterministic joins. Do not rely on semantic search to independently retrieve the warning.

## What to defer

| Defer | Initial substitute |
|---|---|
| Community detection | Public-entry-point-centered neighborhoods with explicit traversal rules |
| FCA/RCA | A small fixed set of insight kinds and evidence-backed applicable cases |
| General composition planner | Published, supported short usage patterns |
| General constraint solver | Source-linked conditions and explicit unresolved requirements |
| Multiple embedding views and graph embeddings | One brief embedding plus lexical retrieval |
| Whole-library autonomous exploration at query time | Precomputed interpretation and deterministic retrieval |

This does not discard the more advanced direction. It produces the reliable assertions and relationships those methods would later need.

---

# 7. Make the analytics requirement part of “done”

This is where I would prevent the implementation from drifting into “documentation search with nicer summaries.”

## The first deliverable must demonstrate all three derivation families

The system should be able to show an end-to-end example of:

**Delegation:** a public entry point linked to the mechanisms it coordinates, with a source-grounded explanation of what the caller does not need to reconstruct.

**Configuration:** a control or restriction derived from related parameter, binding, call, or branch facts, with conditions preserved.

**Handoff:** a producer-to-consumer relationship recovered from a supported example or test and published as a usable pattern.

Each should have:

```text
Input facts
Named analysis method
Structured derived finding
Interpreted assertion
Evidence links
Published brief
Successful retrieval by task wording
```

Not every capability needs an elaborate derivation. A simple API may be adequately explained by direct documentation. Mark that honestly. The **system-level acceptance test**, however, must exercise the graph analytics rather than allow the entire corpus to bypass them.

## Evaluate the interpretation, not just retrieval

I would use approximately 20 standalone task prompts and 5–10 small usage fixtures as an initial evaluation set.

Include tasks that require recognizing an existing integrated API, activating a less-obvious option, using a direct handoff, and noticing a restriction.

Then check two things:

| Evaluation | Question |
|---|---|
| **Agent usefulness** | Did the brief help the agent use the right built-in mechanism and avoid unnecessary custom behavior? |
| **Analytics contribution** | Did the graph-derived interpretation surface a useful relationship or condition that raw fact/document retrieval did not reliably present? |

For the second test, compare the same underlying evidence served as raw retrieval versus as compiled briefs, under similar context budgets. This tests the value of your enrichment pipeline without introducing any analysis of the user’s codebase.

Do not count fewer lines of code as success when they hide incorrect assumptions. Evaluate the behavior implemented, the conditions respected, and the claims supported.

---

## Recommended first implementation

I would specify v1 as:

> **For one bounded library subsystem, compile current-release code facts and selected official evidence into searchable capability briefs. Each brief explains a built-in outcome, its public entry point, important controls, conditions, and a supported short usage pattern. The compiler must derive delegation, configuration, and direct-handoff insights from the CPG. Agents can search these briefs and inspect them, but the system does not yet synthesize arbitrary compositions.**

That gives you **a small query surface, a meaningful interpretation pipeline, and a direct test of whether the resulting knowledge improves generated code**.

The first engineering milestone should be one complete path from **CPG facts → derived insight → interpreted brief → successful agent use**, not another expansion of the ontology or analytics catalog.

**I would implement v1 as an offline capability compiler plus a small retrieval service.** The compiler derives the three agreed insight families from the CPG, interprets them into capability briefs, and publishes a typed dataset. The retrieval service searches those briefs through LanceDB and returns their conditions and evidence.

The concrete defaults I recommend are:

| Component | V1 choice |
|---|---|
| Graph representation | Immutable, directed `petgraph::Graph` projections with lightweight weights |
| Graph analytics | Bounded delegation traversal; parameter-forwarding and local-guard analysis; direct producer-to-consumer handoffs |
| Relational processing | Rust Arrow + DataFusion |
| Interpretation | One structured-output pass using your selected code-capable LLM |
| Embeddings | `Qwen/Qwen3-Embedding-4B`, native 2,560 dimensions, normalized `Float32` vectors |
| Embedding target | One deterministic search representation per published capability brief |
| Retrieval | LanceDB exact vector search + native full-text search + native reciprocal-rank fusion |
| Canonical storage | Existing Arrow/Delta fact and provenance layers |
| Agent interface | `search_capabilities` and `get_capability` |
| Explicitly deferred | Community detection, RCA, graph embeddings, neural reranking, arbitrary composition planning |

The model and library choices above are recommendations, not claims of measured superiority on your corpus. The Qwen model supports a native 2,560-dimensional representation, and LanceDB exposes both exact vector search and full-text/hybrid retrieval through its Rust APIs. :chatgpt-content-reference{index="0"}

I created an implementation handoff containing the detailed design, application configuration, **12 Arrow schema constructors**, and a LanceDB query reference:

**:chatgpt-content-reference{index="17"}**  
:chatgpt-content-reference{index="18"} · :chatgpt-content-reference{index="19"} · :chatgpt-content-reference{index="20"} · :chatgpt-content-reference{index="21"}

I checked the cited APIs and parsed the configuration. **The Rust fragments have not been compiled, and model inference, database operations, IPC round trips, and semantic fixtures have not been executed.**

---

# 1. Architecture and the key integration decision

## Keep compilation and retrieval separate

```text
Current-library CPG
+ selected official documentation, examples, and tests
                         │
                         ▼
              capability-compiler
              Rust / Arrow / DataFusion
                         │
        ┌────────────────┼──────────────────┐
        ▼                ▼                  ▼
  Delegation       Configuration       Direct handoff
  traversal        and local guards    recognition
  petgraph         Rust + DataFusion   Rust + DataFusion
        └────────────────┼──────────────────┘
                         ▼
          Typed findings and witness facts
                         ▼
             LLM interpretation
                         ▼
        Checked capability briefs + evidence
                         ▼
              Embedding generation
                         ▼
           Published Arrow/Delta bundle
                         │
                    Arrow IPC
                         │
                         ▼
                capability-search
             Rust LanceDB + exact lookup
                         ▼
            Two agent-facing operations
```

This preserves your existing distinction between typed records, relational construction, and graph algorithms. The earlier Arrow plan already assigns joins and normalization to DataFusion and path-dependent work to graph routines. :chatgpt-content-reference{index="1"}

**The graph is used to compile useful knowledge. It is not traversed indiscriminately whenever an agent asks a question.**

That makes query behavior easier to inspect and keeps expensive interpretation out of the request path.

## A verified compatibility issue: LanceDB is not on the same dependency train

The inspected **LanceDB `v0.39.0`** manifest uses **Arrow 58, DataFusion 54, and Lance 12**. The inspected **DataFusion `55.0.0`** manifest uses **Arrow 59.2.0**. Therefore, the earlier Arrow 59/DataFusion 55 compiler cannot simply pass its Rust `RecordBatch` or `Expr` objects into that LanceDB build.  

My recommendation is:

> **Keep the compiler on your selected Arrow/DataFusion stack, and give the LanceDB retrieval worker its own matched dependency set. Exchange published batches through Arrow IPC.**

Arrow IPC is explicitly designed for serialized interchange of schemas and record batches. You should still test the exact writer/reader combination and the schema types you use; this is not a claim that every cross-version exchange is automatically tested or zero-copy. :chatgpt-content-reference{index="4"}

This does not require a sprawling service architecture. Two Rust executables in one repository, with a separate dependency workspace where necessary, are sufficient.

**I would not make a Lance/DataFusion forward-port a prerequisite for capability analytics v1.**

---

# 2. Configure petgraph around evidence, not general graph exploration

## 2.1 Use a lightweight immutable directed multigraph

I recommend:

```rust
use petgraph::{Directed, Graph};

#[derive(Clone, Copy, Debug)]
struct ArcRow {
    projection_row: usize,
}

type InvocationGraph = Graph<(), ArcRow, Directed, u32>;
```

The graph contains topology and references into an Arrow side table. Keep names, signatures, source spans, resolution information, and evidence in Arrow rather than copying them into every graph weight.

Petgraph’s `Graph` supports directed edges, compact indices, arbitrary weights, and parallel edges. Its indices can change when elements are removed, so an immutable projection is a straightforward way to avoid index invalidation during analysis. :chatgpt-content-reference{index="5"}

For v1, I would not add another graph library. Standard Rust collections plus petgraph are sufficient for these bounded analyses.

### Preserve three different identities

| Identity | Purpose |
|---|---|
| Canonical CPG node/fact ID | Persistent identity across the compiled dataset |
| Projection row | Identifies the source row and its evidence |
| Petgraph `NodeIndex`/`EdgeIndex` | Temporary coordinates inside this graph instance |

Never persist a petgraph index as the identity of a function or fact.

## 2.2 Build the invocation projection relationally

The initial graph should contain only the selected subsystem’s invocation relationships:

```text
Caller → Candidate callee
```

Construct that edge from the underlying call-site ownership and target facts:

```text
CallSite → Caller
CallSite → Candidate target
```

The corresponding Arrow projection should retain:

```text
snapshot_id
context_id
caller_id
callee_id
call_site_id
ownership_fact_id
target_fact_id
resolution_id
invocation_phase
branch_context_id?
has_unresolved_remainder?
```

DataFusion performs the joins. Petgraph receives the resulting topology.

Important projection rules:

| Rule | Reason |
|---|---|
| Include public entry points independently of their edges | Isolated APIs must remain visible |
| Preserve parallel call sites | Two calls between the same functions can have different arguments and conditions |
| Keep constructor phases tagged | `__new__`, `__init__`, and ordinary invocation are not interchangeable |
| Exclude potential-call-only relationships | A callable value is not necessarily invoked |
| Exclude synthetic model calls from source-call traversal | Analysis-generated relationships need separate interpretation |
| Preserve unresolved remainder | Known targets do not necessarily form an exhaustive dispatch set |

A source call-target graph remains an analyzer model. A path through it is not proof that one execution can traverse all of those calls under the same conditions.

## 2.3 Use bounded traversal with explicit witnesses

Petgraph provides `Bfs`, `Dfs`, and directional edge iteration. However, your traversal needs additional state: depth, selected witness paths, budgets, and stop reasons. I would implement that policy using `VecDeque` over `edges_directed(node, Outgoing)`. :chatgpt-content-reference{index="6"}

My initial application configuration would be:

```toml
[analysis.delegation]
max_call_depth = 2
max_vertices_per_seed = 128
max_edges_per_seed = 512
max_witness_paths_per_target = 3

preserve_parallel_calls = true
sort_adjacency_by_canonical_ids = true

expand_outside_selected_subsystem = false
include_potential_calls = false
include_synthetic_calls = false
cross_native_boundary = false
```

These are **starting budgets**, not empirically optimal thresholds.

The traversal should return more than reachable nodes:

```text
Root public entry point
Reached implementation/collaborator
Ordered supporting call-site facts
Depth
Retained branch/resolution context
Whether additional paths were omitted
Where and why traversal stopped
```

Sort adjacency by canonical identities before selecting paths. Otherwise, insertion order can influence which evidence survives a budget.

### Do not enumerate all paths

For an implementation neighborhood, keep a shortest witness and a bounded number of alternatives. Mark that witness set as nonexhaustive.

Parameter propagation needs a different visited key:

```text
(callable, formal_parameter, mapping_context)
```

Using only `visited_callable` would incorrectly merge two routes that reach the same helper with different parameter mappings.

An SCC computation is optional for labeling recursion. It is not required to terminate a bounded traversal, and I would not make centrality, community detection, or dominance part of this feature.

---

# 3. Detailed design of the three analytics passes

## 3.1 Pass A: public entry point and delegation

### Intended agent insight

> “This public entry point already coordinates the mechanisms needed for this outcome; the internal helpers are not necessarily steps the caller must reconstruct.”

### Construction

First, DataFusion assembles the public-entry mapping:

```text
Public import/access path
    → semantic public entity
    → implementing declaration
```

Next, the bounded invocation traversal finds relevant collaborators. Finally, DataFusion joins those results to their signatures, relevant parameter mappings, documentation, and source excerpts.

The result is a **public-operation-centered evidence package**, not a repository-wide graph summary.

### Findings to emit

| Finding | Meaning |
|---|---|
| Public alias mapping | A supported access path refers to this implementation |
| Direct delegation | This entry point has a recorded call target |
| Bounded delegation path | A short structural path connects the entry point to a collaborator |
| Implementation boundary | Further behavior lies outside the available source/model |
| Incomplete resolution | Known collaborators exist, but additional possibilities remain |

### Interpretation boundary

The interpreter may explain what those collaborators appear to accomplish when the accompanying evidence supports that meaning.

It may not conclude:

```text
Every invocation reaches this helper.
This entry point is always preferable.
Every operation inside the helper is guaranteed.
```

Those conclusions require stronger evidence than call-graph connectivity.

### Why this is valuable

The output helps an agent distinguish **using a public orchestration API** from **rebuilding the orchestration out of private mechanisms**.

That is a direct route from graph analytics to less bespoke code.

---

## 3.2 Pass B: configuration propagation and local restrictions

This is likely to yield some of your most useful early insights. I would implement four small recognizers rather than a general dataflow engine.

| Recognizer | Accepted v1 pattern | Output |
|---|---|---|
| Direct forwarding | A call argument resolves to a specific source parameter binding | Parameter-to-argument mapping |
| Identity alias | One unambiguous local assignment in a supported straight-line region | Alias-backed forwarding |
| Default/transformation observation | A literal, selected default, or expression is supplied downstream | Explicit transformed/defaulted argument |
| Local restriction | A supported predicate involving known inputs leads to a local raise | Conditional-raise finding |

### Parameter forwarding

For a hypothetical wrapper:

```python
def export(data, *, compression="zstd"):
    return writer(data, codec=compression)
```

The useful machine finding is:

```text
source parameter: export.compression
call site: writer(...)
target parameter: writer.codec
argument expression: compression
mapping kind: direct forwarding
```

The semantic statement “this controls output compression” still needs support from the consumer’s contract or implementation.

The mapping algorithm should handle known positional and keyword arguments **per candidate signature**, including implicit receivers for bound methods.

Do not guess through unresolved `*args`, `**kwargs`, ambiguous overloads, multiple writes, or property/subscript access.

### Limit aliases deliberately

Support:

```python
local_codec = compression
writer(codec=local_codec)
```

only when the relevant local binding is unambiguous and the intervening region is supported.

A name match is insufficient. Likewise, object-identity forwarding does not prove that its contents were not mutated.

### Local guards

For a hypothetical guard:

```python
if isinstance(data, Scanner):
    if schema is not None:
        raise ValueError(...)
```

emit:

```text
callable
outer branch context
predicate
parameter dependencies
raise site
enclosing handler context
```

The default interpretation is:

> “The implementation contains this conditional raise in this input-handling branch.”

It becomes a public precondition only when the supported path and evidence justify that promotion. An enclosing handler might catch it; a public path might never reach it.

### Useful Arrow output

```text
forwarding_findings
  source_parameter_id
  call_site_id
  target_callable_id
  target_parameter_id?
  argument_expression_id
  forwarding_kind
  branch_context_id?
  resolution_id
```

Keep these fields typed. Do not reduce the finding to a sentence before recording its structure.

---

## 3.3 Pass C: direct producer-to-consumer handoffs

### Intended agent insight

> “The library already accepts this produced object directly. You do not need an intermediate conversion or custom transfer loop.”

Analyze selected official examples and tests, not arbitrary co-occurrence across the source tree.

Start with:

```python
intermediate = producer(...)
consumer(intermediate, ...)
```

and direct nested calls.

### Acceptance conditions

I would initially require a supported straight-line region and an unambiguous producer result. Check for intervening reassignment, additional consumers, resource boundaries, and mutation or escape calls.

Those checks do not establish complete effect analysis. They determine whether the small recognizer has enough evidence to publish a simple handoff pattern or must preserve additional uncertainty.

The core record should contain:

```text
source_example_id
region_id
producer_call_id
consumer_call_id
producer_api_id
consumer_api_id
binding_id?
consumer_argument_id
handoff_status
```

### What not to infer

A producer return type matching a consumer parameter type is a **candidate connection**, not a supported recipe.

Similarly:

```text
A and B appear in one test file
```

is not equivalent to:

```text
The value produced by A is consumed by B,
and this example/test exercises their combination.
```

Publish a supported usage pattern only with an official example, relevant test, or explicitly executed fixture.

### Preserve necessary setup

When converting a test into a usage pattern, retain initialization, schema declarations, resource ownership, and required configuration.

Remove incidental test details only when doing so does not remove a precondition.

For v1, keep patterns to one principal operation or a direct two-operation handoff, plus necessary setup. Do not add arbitrary composition search.

---

# 4. Make findings, interpretation, and publication distinct stages

## A finding is not a generated claim

I would use the following sequence:

```text
Typed finding
    + ordered witness facts
    + relevant source/documentation
    + coverage and boundaries
                     ↓
               Evidence packet
                     ↓
         Structured LLM interpretation
                     ↓
          Grounding and semantic review
                     ↓
             Published insight
```

The evidence packet should include the public signature, the relevant finding payloads, supporting paths, conditions, selected documentation/example passages, and the complete boundary report.

**Do not omit warnings or unresolved conditions merely to fit a token budget.** Split the packet by applicable case or mark the resulting brief incomplete.

## Constrain the interpreter

Use your selected code-capable generation model through one configured endpoint. There is no need to introduce another model-selection project for interpretation.

Its output should contain atomic assertions:

```text
assertion
applicable case
supporting finding IDs
supporting source IDs
conditions
limitations
support status
```

A suitable v1 policy is one structured generation call per evidence package and at most one schema-repair attempt.

JSON-schema validity is not semantic verification. Check both:

| Check | Mechanism |
|---|---|
| Required fields and allowed categories | Typed decoding/schema validation |
| Public symbols and parameter names exist | CPG joins |
| Cited evidence belongs to this snapshot | Provenance checks |
| Witness path exists | Finding/witness validation |
| Claimed behavior stays within supported conditions | Evidence review |
| Usage pattern works for specified inputs | Optional scoped execution fixture |

I would manually inspect the initial small corpus before publication. That is simpler than constructing a multi-agent reviewer hierarchy and gives you concrete failure cases for later automation.

Treat repository text as untrusted input, not instructions to the interpreter. Run executable fixtures separately, without credentials or network access by default.

---

# 5. Embedding design

## 5.1 Model choice

For a new v1 deployment, I recommend:

```text
Model:             Qwen/Qwen3-Embedding-4B
Inference:         BF16
Output dimensions: 2560
Stored vectors:    Float32
Normalization:     L2
Distance:          cosine
Serving:           local vLLM embedding endpoint
```

The official model card specifies up to 2,560 dimensions and shows query-specific instructions, unprefixed retrieval documents, and normalized embeddings. :chatgpt-content-reference{index="7"}

The reason to choose it here is practical: you have a local GPU setup, the retrieval object is a rich textual brief, and the initial corpus is small enough that vector storage is not a meaningful pressure.

**I would not reduce dimensions, quantize the vectors, or benchmark a large menu of embedding models before the vertical slice works.**

An already-working 8B embedding service need not be replaced to make progress, but its vectors must use their own model/dimension contract rather than being mixed with this 4B/2,560-dimensional index.

## 5.2 Embed the interpreted capability, not the raw graph

The embedding target should be a deterministic projection of the published brief:

```text
Outcome:
  What the caller can accomplish.

Applicable case:
  The relevant input form or operating mode.

Public APIs:
  Exact public access paths.

Inputs and outputs:
  The important representations.

Built-in controls:
  Parameter names and supported effects.

Usage pattern:
  The operation or direct handoff.

Conditions and limitations:
  The qualifications necessary to interpret the capability correctly.
```

Do not embed an adjacency dump, all private helper names, every AST node, or an entire source file.

The graph has already contributed by revealing relationships that informed this text.

### Keep the embedding text shorter than the evidence bundle

I would target a few hundred to approximately one thousand tokens, with a configured hard maximum of 2,048 document tokens.

A brief that cannot be represented within that limit without losing essential conditions is probably too broad for this v1 unit. Split its applicable cases or revise it rather than silently truncate.

Full evidence remains available through `get_capability`; it does not all belong in the vector.

## 5.3 Query/document asymmetry

Use the Qwen instruction convention on the **query only**:

```text
Instruct: Given a programming task, retrieve Python-library capability
briefs explaining the built-in APIs, configuration, conditions, and
usage patterns needed to perform it.
Query: <agent's task>
```

Documents use the deterministic capability text without that query instruction. This follows the model’s documented retrieval pattern. :chatgpt-content-reference{index="8"}

The full-text search lane receives the plain task text, not the instruction prefix.

## 5.4 Store an immutable embedding specification

The embedding specification should record:

```text
model repository and revision
tokenizer revision
pooling configuration
inference build
query instruction template
document template
dimensions
output dtype
normalization
```

Compute an application-defined specification hash over that record.

Cache embeddings by:

```text
embedding_spec_hash + embedding_input_hash
```

A vector is not reusable merely because the visible text is unchanged when the model, pooling, or input convention changes.

## 5.5 vLLM and the Rust client

vLLM supports a pooling runner and an OpenAI-compatible `/v1/embeddings` endpoint. I would use that existing service boundary rather than embed a new inference runtime inside the Rust analytics process. :chatgpt-content-reference{index="9"}

The Rust client should use bounded requests and validate every response:

```text
response count matches input count
response indices are mapped back correctly
every vector has 2560 elements
all elements are finite
norm is nonzero
normalization matches the stored contract
server/model identity matches the expected deployment
```

Begin with batches of eight briefs and one in-flight request. Those are conservative defaults to test, not performance claims.

On your shared GPU, I would initially schedule bulk interpretation and bulk embedding separately. Do not assume independent vLLM memory reservations can safely coexist with a large generation model.

---

# 6. Typed Arrow and LanceDB storage

## 6.1 Keep canonical facts separate from search projections

I would maintain:

| Store | Authoritative responsibility |
|---|---|
| Existing Arrow/Delta tables | Findings, claims, evidence, brief content, validation, publication manifest |
| LanceDB | Rebuildable vector/full-text projection of published briefs |
| In-memory exact-symbol map | Deterministic symbol-to-brief lookup for the active generation |

This avoids having an LLM-generated search summary become the only surviving representation of a claim.

The existing Arrow plan’s fixed-width IDs, typed categories, and explicit schema registry remain appropriate. :chatgpt-content-reference{index="10"}

## 6.2 One row per published brief

The LanceDB table should have a declared Arrow schema, approximately:

```text
snapshot_id          FixedSizeBinary(16)           not null
brief_id             FixedSizeBinary(16)           not null
generation_key       Utf8                          not null

library              Utf8                          not null
release              Utf8                          not null
title                Utf8                          not null
outcome              Utf8                          not null

public_symbols       List<Utf8>                    not null
parameter_names      List<Utf8>                    not null
embedding_text       Utf8                          not null
lexical_text         Utf8                          not null

brief_hash           FixedSizeBinary(32)           not null
embedding_spec_hash  FixedSizeBinary(32)           not null
embedding_text_hash  FixedSizeBinary(32)           not null

vector               FixedSizeList<Float32, 2560>  not null
```

LanceDB’s Rust examples explicitly construct tables from Arrow `RecordBatch` values with fixed-size-list vector columns. 

Validate uniqueness of `(snapshot_id, brief_id)` within the published generation. One generation should use one embedding specification.

The string `generation_key` is a serving identifier for the immutable bundle; it does not replace canonical binary IDs.

## 6.3 Separate lexical text from embedding text

For `lexical_text`, include the same semantic material plus deduplicated components of public names:

```text
Original:
  pyarrow.dataset.write_dataset

Additional searchable forms:
  write_dataset
  write dataset
```

Preserve originals for display and exact matching.

Load an exact-symbol dictionary from the published `brief_members` data:

```text
qualified public symbol → brief IDs
```

That prevents punctuation and tokenization from becoming the only route to exact API discovery.

Do not infer a capability relationship from lexical similarity. This dictionary is derived from verified membership.

---

# 7. Retrieval: use LanceDB’s built-in capabilities

## 7.1 Exact vector search first

For the initial corpus, do **not** build an ANN index.

Use:

```rust
.bypass_vector_index()
.distance_type(DistanceType::Cosine)
```

LanceDB documents `bypass_vector_index()` as an exhaustive flat search. It also defaults to L2 distance unless another metric is specified, so set cosine explicitly. :chatgpt-content-reference{index="12"}

This removes ANN training, tuning, and recall loss from the first implementation.

At 2,560 `Float32` elements, one vector contains 10,240 bytes before table/index overhead. For a few dozen briefs, dimension reduction is not a useful initial optimization.

## 7.2 Native full-text search

Create one native FTS index over `lexical_text`:

```rust
table
    .create_index(
        &["lexical_text"],
        Index::FTS(FtsIndexBuilder::default()),
    )
    .execute()
    .await?;
```

LanceDB documents that index-building path and Rust full-text query interface. Its query API returns FTS results ordered by BM25 score. :chatgpt-content-reference{index="13"}

I would use this rather than add a separate Tantivy application integration for v1.

## 7.3 Native reciprocal-rank fusion

Combine vector and FTS results through `RRFReranker::default()`.

The inspected implementation provides native RRF with a default constant of 60. This is a rank-fusion algorithm, not a neural reranker and not a calibrated confidence estimator. 

A representative query is:

```rust
let stream = table
    .query()
    .only_if_expr(
        col("generation_key").eq(lit(generation_key.to_owned()))
    )
    .select(Select::columns(&[
        "brief_id",
        "snapshot_id",
        "title",
        "outcome",
        "brief_hash",
    ]))
    .full_text_search(
        FullTextSearchQuery::new(plain_query.to_owned())
    )
    .nearest_to(unit_query_vector)?
    .column("vector")
    .distance_type(DistanceType::Cosine)
    .bypass_vector_index()
    .rerank(Arc::new(RRFReranker::default()))
    .limit(20)
    .execute()
    .await?;
```

The referenced query methods are exposed in the Rust SDK. Use LanceDB’s own expression helpers inside this worker so the `Expr` belongs to its matched DataFusion dependency family. :chatgpt-content-reference{index="15"}

The package contains the imports and basic vector validation around this fragment.

### Exact-symbol results remain a separate signal

After hybrid retrieval, merge exact-symbol matches by canonical brief ID and return, initially, at most five results.

Record whether a result was promoted by exact symbol lookup rather than implying the final order is purely RRF.

No neural reranker is required initially.

## 7.4 Hydration is deterministic

Once the agent selects a brief:

```text
(snapshot_id, brief_id)
    → full published brief
    → all attached conditions
    → all limitations
    → supporting evidence
    → usage pattern and validation status
```

Those joins should not depend on another semantic search.

**The warning paragraph must not be optional merely because its embedding was less similar to the query.**

Also, nearest-neighbor search will produce neighbors even for an unrelated task. Return relevance and coverage information without interpreting a similarity score as proof that the brief satisfies arbitrary requirements.

---

# 8. How DataFusion participates in retrieval

There are two sensible integration levels.

## V1: Arrow result integration

Use the LanceDB SDK for its specialized retrieval query. Receive a small Arrow result set, map canonical IDs, and join those IDs to the full brief/evidence records.

With a matched dependency family, the small result batches can be registered as a DataFusion in-memory table. With the split dependency arrangement, serialize them through IPC first.

For a top-20 candidate set, bounded collection is entirely reasonable. You do not need a custom streaming table provider just to avoid collecting a few dozen rows.

## Later: matched provider/plan integration

Deeper Lance/DataFusion integration can be useful when you want broader SQL access over the retrieval dataset.

It is not required for v1, and it should not be confused with the SDK’s vector/FTS query path. A generic relational scan should not be assumed to invoke the desired retrieval operation automatically.

**Use the native retrieval API for retrieval and DataFusion for relational enrichment.** This is both simpler and more faithful to the available interfaces.

---

# 9. Publication, reproducibility, and operational behavior

## Publish immutable generations

I recommend this publication sequence:

```text
1. Compile and validate canonical findings and briefs.
2. Generate embeddings and serving batches.
3. Write a staging publication manifest.
4. Build a new LanceDB generation and FTS index.
5. Check row counts, hashes, dimensions, and smoke-test queries.
6. Atomically switch the active manifest.
```

The manifest should identify the CPG snapshot, canonical table versions, brief bundle, embedding specification, LanceDB table/generation, and validation report.

Existing requests hold the generation they resolved at request start.

This is a consistency mechanism for current-library knowledge, not a version-comparison feature. Do not assume cross-store ACID transactions between Delta and LanceDB.

## Make incomplete analysis visible

Record distinct boundary reasons:

```text
native implementation unavailable
unresolved call target
unsupported argument unpacking
ambiguous local binding
unsupported control flow
scope boundary
depth or size budget reached
missing documentation/example evidence
```

These should inform interpretation and coverage reporting.

Do not convert “the analysis stopped” into “the library has no further capability.”

## Keep operational dependencies modest

The supporting Rust dependencies can remain conventional:

| Need | Approach |
|---|---|
| Graph topology | `petgraph` |
| Queues, maps, bounded traversal state | Standard library |
| Typed data and relational processing | Arrow/DataFusion |
| Model HTTP calls and concurrency | `reqwest` + `tokio` |
| Typed model/API records | `serde`, with schema validation |
| Deterministic fingerprints | Existing canonical hash component |
| Search/index operations | `lancedb` |
| Operational tracing | Existing logging/tracing setup |

I would not introduce a second graph framework, a distributed task engine, or a new inference engine for this feature.

---

# 10. Tests that establish the analytics are actually working

I would organize testing around four boundaries.

## Graph and recognizer fixtures

Use small synthetic Python programs with expected typed findings:

| Fixture | Failure it should catch |
|---|---|
| Two parallel calls with different arguments | Accidental edge collapse |
| Two paths to one helper with different parameter mappings | Incorrect visited-state deduplication |
| Reassigned local variable | False direct handoff |
| Unknown `**kwargs` | Invented parameter mapping |
| Guard inside a caught exception region | False universal precondition |
| Callable value that is never invoked | Potential-call/source-call confusion |
| Native or unresolved target | False completeness |
| Shuffled input row order | Nondeterministic witness selection |

## Interpretation checks

Check that assertions retain conditions, use public APIs, cite actual findings, and do not inflate structural evidence into behavioral guarantees.

In particular, require specific support for claims about transactionality, hard memory limits, all-input ordering, thread safety, or exhaustive exception behavior.

## Embedding and database checks

Verify vector dimensions, finiteness, normalization, query/document conventions, model-spec consistency, and no silent truncation.

For the small corpus, compare LanceDB’s flat-search ranking with a straightforward cosine reference, allowing floating-point tolerances and tied distances. Test exact qualified-name lookup and ensure full conditions are hydrated with every selected brief.

## Product checks

Use held-out standalone programming tasks and ask:

```text
Did the relevant built-in capability appear?
Did the agent discover the important configuration?
Did it use a supported direct handoff?
Did it avoid reconstructing library behavior?
Did it respect the attached limitations?
```

Then compare raw-evidence retrieval against the compiled briefs under similar context budgets. That tests whether the graph analytics and interpretation add value, without introducing user-codebase or library-version comparison scope.

---

# Recommended implementation order

| Increment | Concrete deliverable |
|---|---|
| **1. One complete path** | One public entry point → typed finding → interpreted brief → embedding → search → evidence hydration |
| **2. All three analytics families** | Delegation, configuration/local guards, and direct handoffs passing synthetic fixtures |
| **3. Bounded real corpus** | Approximately 15–25 reviewed briefs for the selected subsystem |
| **4. Reliable serving** | Immutable publication, native hybrid retrieval, exact-symbol lookup, and coverage reporting |
| **5. Agent evaluation** | Held-out tasks showing whether briefs improve built-in feature use |

The central design decision is to put sophistication into **the evidence-to-insight transformation**, not into the number of query modes or retrieval algorithms.

**For v1, petgraph discovers bounded structural relationships; small Rust recognizers identify controls and handoffs; the LLM explains their practical significance; and LanceDB makes those explanations discoverable without separating them from their conditions and evidence.**