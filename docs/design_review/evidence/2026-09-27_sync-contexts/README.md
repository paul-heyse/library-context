# Synchronous context source boundary

**2026-09-27 · Tested interface observation and bounded lifecycle implementation.** `source_probe.py` links
`source_probe.rs` against this checkout's current cached release artifacts and runs the real
in-process extractor on generated source. It never executes that Python source or reads the
FastMCP evaluation gold. Reproduce after a release build with:

```sh
uv run --no-sync python docs/design_review/evidence/2026-09-27_sync-contexts/source_probe.py
```

The compiler-88/extractor-32 observation is in `raw/source.txt` (Git LFS). The generated source
uses `nullcontext()`, `nullcontext(value) as chosen`, `suppress(TypeError)`, two context items and
an opaque manager. Constructors emit separate `new` and `init` observations. Implicit entry has
site kind `artificial_call` and call phase; no exit is reported. The opaque manager is unresolved.
The class has no signature; `nullcontext.__init__` has two signatures and `__enter__` one.
`suppress`'s stub entry targets `AbstractContextManager.__enter__`; this is a provider observation,
not the runtime method identity. Source inspection of the installed CPython 3.14.7 `contextlib.py`
shows a separate `suppress.__enter__` returning None, and exit using `issubclass` plus separate
BaseExceptionGroup handling. `nullcontext` returns its stored entry result and preserves outcomes.

Context7 resolved `/python/cpython` and fetched the official `Doc/library/contextlib.rst`
nullcontext/suppress documentation. Its main-branch excerpts were navigation only; the installed
3.14.7 source supplied pinned behavior. Class-specific runtime semantics must therefore remain
synthetic authored model assertions, distinct from stub method facts and constructor signatures.
[ADR-0059](../../../adr/0059-synchronous-context-protocols.md) records the applicability decision.
The source probe alone does not qualify completion. The focused controls below exercise the implemented subset; Stage 3 remains incomplete.

Identifier observations for bare `TypeError` also carry new/init phases; those are not executed
constructor calls. The binder must match the explicit call source role and exact range. WithItem
identity includes the `as` target while construction/entry ranges cover the context expression.
The observed entry flag `implicit_dunder_call=false` does not replace that structural role.
The final probe queries fail closed; initial exploratory queries with wrong column names were
corrected before recording this output. This remains interface evidence for binder design.


## Independent runtime challenge

`runtime_oracle.py` runs 14 generated, finite programs in separate bounded CPython 3.14.7
workers. It never reads or executes the fixture package or compiler inputs. `sys.monitoring`
records actual contextlib entry, exit and unwind events. The receipt records the interpreter pin
and contextlib source hash; monitored method code comes from that runtime, not the compiler model.

```sh
uv run --no-sync python docs/design_review/evidence/2026-09-27_sync-contexts/runtime_oracle.py
```

The cases distinguish preservation from suppression, matching from nonmatching classes, normal
return from raises, short-circuit matching from an invalid first class, and break/continue from
exception suppression. Partial constructor failure does not register an exit. Failed `as`
unpacking does register the successful entry and unwinds in reverse; an exit TypeError replaces
the pending exception and can be suppressed by the outer context. Each generated program also receives a fresh opaque object, separately from the monitored integer
invocation, to distinguish entry-value identity from equality. The compiler's bounded value proof
now joins the explicit parameter argument and unique `as` return within that active context;
general alias propagation remains outside this slice.

The real `context_protocols_bind_class_and_constructor_roles_independently` release test compiles
source through the extractor, Delta publication and FORMAT 9 native loading, and compares the
positive returned paths with independent outcomes. Fake embeddings only satisfy fixture storage;
no live embedding qualification is claimed. The native reader rejects missing lifecycle support,
missing/duplicate completion certificates, deleted match evidence, wrong condition scope, omitted
lifecycle groups and same-function site swaps. Shared source reconstruction rejects forged model
and return-completion certificates. Pure certificate tests cover ordered evidence and explicit
condition specialization. Original proof limits remain unchanged.


**Focused receipts (2026-09-27).** The final Rust contract/completion/finite/context and generation
selection passed 57 cases (147 skipped), `/tmp/lctx-stage3-context-final2.log`. After rebuilding the
editable native package, the real context replay passed one case (30 skipped),
`/tmp/lctx-stage3-context-current-native.log`. Python server/digest checks passed 15 cases and the
generation type check reported zero errors. Initial runs failed on implementation/test adapter
corrections and expected schema migrations; the reviewed migration was accepted before these final
checks. Full gates, fresh pilot, clean wheel and live embedding remain `not_run`.


**Entry-value extension (2026-09-27; compiler 90, focused Tested).** The source/Delta/native
controls admit positional and keyword entry arguments and preserve the correct formal when two
inputs exist. Rebinding, deletion, nested nonlocal mutation, omitted entry values, suppressor entry,
completed sibling contexts, opaque predecessors and overridden returns withhold the certificate.
Raw nonidentity/through-call flags and open coverage are retained. Missing/duplicate/foreign
certificates and omission of both certificate and value witness are refused. A pure structural
control moves the value after cleanup, before assignment or under a foreign return scope; all
refuse. Publication reconstructs the exact certificate from source. The final selection passed
45 cases (160 skipped), `/tmp/lctx-stage3-context-value-final.log`; full Stage 3 stays open.

The rebuilt-native replay passed one case (30 skipped), 6.796 seconds,
`/tmp/lctx-stage3-context-value-native-current.log`, after the opaque-object assertion was added.
Python server/digest checks passed 15 cases; generation type checking reported zero errors.
