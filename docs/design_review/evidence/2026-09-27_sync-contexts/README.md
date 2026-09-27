# Synchronous context source boundary

**2026-09-27 · Tested observation, Proposed model implementation.** `source_probe.py` links
`source_probe.rs` against this checkout's current cached release artifacts and runs the real
in-process extractor on generated source. It never executes that Python source or reads the
FastMCP evaluation gold. Reproduce after a release build with:

```sh
uv run --no-sync python docs/design_review/evidence/2026-09-27_sync-contexts/source_probe.py
```

The compiler-88/extractor-32 observation is in `raw/source.txt` (Git LFS). The generated source
uses `nullcontext()`, `nullcontext(value) as chosen`, `suppress(TypeError)`, two context items and
an opaque manager. Constructors emit separate `new` and `init` observations. Implicit entry has
site kind `attribute_access` and call phase; no exit is reported. The opaque manager is unresolved.
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
This probe neither qualifies the pending completion implementation nor establishes Stage 3 exit.

Identifier observations for bare `TypeError` also carry new/init phases; those are not executed
constructor calls. The binder must match the explicit call source role and exact range. WithItem
identity includes the `as` target while construction/entry ranges cover the context expression.
The observed entry flag `implicit_dunder_call=false` does not replace that structural role.
The final probe queries fail closed; initial exploratory queries with wrong column names were
corrected before recording this output. This remains interface evidence for binder design.
