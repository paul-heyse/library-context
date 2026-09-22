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