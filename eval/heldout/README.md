# FastMCP 4.0.5 held-out evaluation set (sealed)

This set was authored on 2026-09-23, before any capability brief existed. Keep it away from
brief compilation and brief authoring. It is meant for scoring only.

## How it was made

- **Sources.** Only the official FastMCP MDX docs at
  `build/sources/fastmcp/004bf15a2ba99f077160993c00a404e1a9da83ea/docs/`, mainly
  `getting-started/`, `servers/`, `clients/`, `patterns/` and `tutorials/`. No briefs, gold
  files, compiler outputs, repo design docs, library source, web search or Context7 were used.
- **Names.** Every `expected_calls` path and every `expected_controls` keyword was checked by
  importing and running `inspect.signature` against the installed library in
  `build/envs/fastmcp`.
- **Quotes.** Every `limits[].quote` is verbatim from its doc after whitespace normalisation, and
  every cited heading exists in its doc. Both were checked mechanically on 2026-09-23.

## Task schema (`tasks.json`)

`tasks.json` is an array of 20 objects with these fields:

| Field | Meaning |
|---|---|
| `id` | `T01` to `T20` |
| `area` | `register`, `inputs`, `outputs`, `errors`, `resources`, `middleware` or `other` |
| `prompt` | The task in a developer's words. It never names the FastMCP API that solves it |
| `expected_calls` | FastMCP public access paths that a good solution uses |
| `expected_controls` | `{access path: [keyword, ...]}`, listed only where the docs make that control matter |
| `limits` | `[{statement, quote, doc, heading}]`: documented limits or preconditions. `doc` is relative to `docs/`. `heading` is `(page introduction)` for text that sits before the first heading |
| `rebuild_smell` | What a solution that rebuilds library behaviour by hand looks like |
| `sources` | `[{doc, headings}]`: the docs and headings the task was built from |
| `fixture` | The path of the task's pytest file, or `null` when the task has none |

Areas: register 3, inputs 2, outputs 3, errors 2, resources 3, middleware 3, other 4.

## Fixtures

These tasks have fixtures: T01, T03, T04, T06, T09, T11, T14 and T18. Each one has
`fixtures/fixture_Txx.py` and `fixtures/reference_solution_Txx.py`.

- **Candidate contract.** A fixture tests a module named `solution` that exposes a module-level
  `mcp` (a `fastmcp.FastMCP`). T14 also needs a module-level `AUDIT_LOG` list.
- **How the fixtures run.** They use only the in-memory `fastmcp.Client(solution.mcp)`, inside
  `asyncio.run`. There is no network, no credentials, no sleeps and no pytest-asyncio.
- **Checking a candidate.** Run each fixture in a fresh scratch directory:

  ```bash
  cd <scratch-dir>
  cp <candidate>.py solution.py
  cp eval/heldout/fixtures/fixture_Txx.py .
  build/envs/fastmcp/bin/python -m pytest -q fixture_Txx.py     # needs pytest
  build/envs/fastmcp/bin/python fixture_Txx.py                   # fallback: runs every test_*
  build/envs/fastmcp/bin/python -c 'import fixture_Txx as f; f.test_...()'   # one test
  ```

**pytest is not installed in `build/envs/fastmcp`.** That is why the reference solutions were
verified with the fallback: every `test_*` function was called directly with
`python -c 'import fixture_Txx as f; [getattr(f, n)() for n in dir(f) if n.startswith("test_")]'`.

On 2026-09-23 all eight reference solutions `passed` that way. The pytest form was `blocked`,
because `No module named pytest`. Naive variants were also run to check that the fixtures
discriminate. Each variant drops the key control, and each failed its fixture: T01 without
annotations, T04 without strict validation, T06 returning `dict`, T09 without error masking, and
T11 without a MIME type.

Server-side tracebacks printed to stderr while T03, T09 and T14 run are expected. They are the
server logging the tool errors that those tests provoke on purpose.

## Python used

The environment was `build/envs/fastmcp/bin/python`: CPython 3.14.7 with fastmcp 4.0.5. The
in-memory client negotiated MCP protocol `2026-07-28`, the modern era.

## Documentation inconsistencies noticed while authoring

- **The Storing State example.** In `servers/middleware.mdx` ("Storing State"), `set_state` and
  `get_state` are called without `await`. They are coroutines, as both introspection and
  `servers/context.mdx` ("Request State") show.
- **State across mounts.** `servers/middleware.mdx` ("Server Composition") says
  middleware-stored state does not cross mount boundaries. `servers/context.mdx`
  ("Request State") says it is inherited by mounted children within the same request. T16
  avoids mounts for this reason.
- **Rate-limit burst capacity.** `servers/middleware.mdx` ("Rate Limiting") lists
  `burst_capacity` with a default of `20`. The installed signature defaults it to `None`.
- **Typed prompt arguments.** The description suffix that `servers/prompts.mdx`
  ("Argument Types") shows for typed arguments ("Provide as a JSON string matching...") differs
  from the text the library emits ("Provide a value matching the following JSON schema...").

## Integrity

`MANIFEST.sha256` holds the sha256 of every other file here. To check it:
`cd eval/heldout && sha256sum -c MANIFEST.sha256`.
