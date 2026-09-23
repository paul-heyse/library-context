# Analyzed libraries

Every Python library the compiler builds a code property graph for is a small, committed uv project
here, one directory per library (DESIGN §4.0, ADR-0013). None of them is a dependency of this
project; the project's own environment is never an analysis input.

| File | What |
|---|---|
| `pyproject.toml` | A virtual uv project (`[tool.uv] package = false`) with exactly one pinned requirement and an exact `requires-python`. `[tool.lctx] release` names the first-party distributions whose code is compiled; everything else installed is dependency context. `[tool.lctx.source]` names the upstream tag and commit, and the globs that select its `documents` (less `documents_exclude`), `examples` and `tests`; each module's role (`example`, `test`, or `doc_block` for a document's Python block) comes from the key that selected it, and a file two keys select is refused (ADR-0015) |
| `.python-version` | The exact interpreter |
| `uv.lock` | The acquisition lock: every distribution of the closure with each artifact's sha256. Reviewed and committed |

## Add a library

```sh
lctx library init <name> --requirement '<dist>[extras]==<version>'
```

This writes `libraries/<name>/`, runs `uv lock`, acquires the environment, and proposes `release` as
the requested distribution plus every installed distribution that shares its source repository.
Review the proposal and the lock, then commit the three files.

## Upgrade a library

1. Edit the pin in `pyproject.toml`.
2. `uv lock --project libraries/<name> --upgrade-package <dist>`, and review the lock diff.
3. `lctx compile <name> --store build/store` (acquires, verifies, extracts, derives, validates,
   publishes).
4. For `fastmcp`, move the `fastmcp` skill (the gold reference) to the same version through its own
   `MAINTENANCE.md`; `just gold` (run by `just test-all`) fails until they agree.

## Compile

```sh
lctx acquire <name> [--reinstall]       # uv sync --frozen into build/envs/<name>
lctx compile <name> --store build/store # the snapshot id, per-table rows and versions
just pilot                              # lctx compile fastmcp, release build
```

`lctx acquire` runs `uv sync --frozen --no-install-project --no-config --python <.python-version>
--link-mode copy`: user and system uv configuration is ignored, the interpreter pin is enforced,
and files are copied rather than hard-linked from the uv cache. It removes every `UV_*` variable
and `VIRTUAL_ENV` from uv's environment, and passes an absolute environment path (uv resolves a
relative `UV_PROJECT_ENVIRONMENT` against the project). Stage A then refuses an environment that is
not the lock's or the pin's, a release locked without artifact hashes, and any analyzer-readable
file (`.py`, `.pyi`, `py.typed`) that differs from its `RECORD`; `lctx acquire <name> --reinstall`
repairs it. A declared `[tool.lctx.source]` must pin a 40-hex `commit` whose `tag` names the
locked version.
