# `typing.assert_type` identity oracle (2026-09-25)

The Stage 3 model treats `typing.assert_type(val, typ)` as a no-effect identity on
`val`. The CPython typing documentation says runtime execution performs no check
and returns `val` unchanged. The pinned catalog binds the `val` formal against
the analysis context before publication.

From this directory, in an isolated process with CrossHair 0.0.110 and Python
3.14:

```sh
timeout 60s uvx --python 3.14 --from crosshair-tool==0.0.110 crosshair diffbehavior probe.actual probe.wrong
timeout 60s uvx --python 3.14 --from crosshair-tool==0.0.110 crosshair diffbehavior probe.actual probe.modeled
```

The wrong control exited 1 with `value=0`, returning 0 versus 1. The identity
comparison exited 0 with `No differences found. (attempted 2 iterations)` and
`All paths exhausted, functions are likely the same!`. This is **Tested** on
the pure `int` specialization only. The focused compiler test separately
checks pinned target/formal resolution and exact source-argument attribution;
neither test proves a composed summary or served verdict.
