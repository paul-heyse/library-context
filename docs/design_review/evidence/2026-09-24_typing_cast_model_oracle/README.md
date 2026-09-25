# `typing.cast` identity model oracle (2026-09-24)

The committed Stage 3.1 catalog models `typing.cast(typ, val)` as an identity transfer from
`Parameter[val]` to `ReturnValue`. The local pinned CPython 3.14.7 signature check reported
`(typ, val)`. CrossHair 0.0.110 ran in an isolated `uvx --python 3.14` environment with a
60-second process timeout (`uvx ... python --version` reported 3.14.7); it did not analyze or
execute any project fixture or pilot library.

From this directory:

```sh
timeout 60s uvx --python 3.14 --from crosshair-tool==0.0.110 crosshair diffbehavior probe.actual probe.wrong
timeout 60s uvx --python 3.14 --from crosshair-tool==0.0.110 crosshair diffbehavior probe.actual probe.modeled
```

The wrong-model control returned exit 1 and a witness `value=0` (`0` versus `1`). The identity
comparison returned exit 0: `No differences found. (attempted 2 iterations)` and `All paths
exhausted, functions are likely the same!`. This is **Tested** evidence for the pure `int`
specialization only. It does not prove every generic `typing.cast` input, the pinned typeshed
signature, or model application in published summaries. CrossHair executed wrappers here, not
the compiler, so the compiled-row and serving gates remain separate.
