# L4 — Serving path: FastMCP, MCP SDK, PyO3 bridge

Lane L4 of the 2026-10-04 library-leverage review. Static research from pinned source, the
`fastmcp` and `pyo3` skills and upstream release notes; no probe was run. Baseline `main` at
`948b2a88` (concurrent work landing; paths cited at that tree). Pins checked: `fastmcp==4.0.5`
(fastmcp-slim 4.0.5), `mcp`/`mcp-types` 2.2.0, `httpx2` 2.13.1, `pydantic` 2.13.5
(`python/lctx_mcp/pyproject.toml`, installed `.venv`); `pyo3 =0.29.2`, `pyo3-async-runtimes
=0.29.0` (`Cargo.toml:27,36`). Labels follow the principles' §D scale; everything below is
**Proposed** interpretation over source-reviewed facts unless marked otherwise.

The `fastmcp` skill was used only to understand FastMCP as our serving dependency. Nothing here
is, or may become, a compiler input or evaluation gold content.

## Capability table

| Library · item at pin (source) | Contract / limits | Repo consumer | Fit: replaces/enables · what it would not preserve |
|---|---|---|---|
| FastMCP `Tool` subclass + `output_schema`, `annotations`, `meta` (`fastmcp/tools/base.py` 240–349) | Explicit schema must describe an object; `NotSet` derives, `None` disables. No server-side output validation in 4.0.5 or mcp 2.2.0 (`mcp/server/validation.py` covers sampling only) | `wire.py` `SchemaTool`, `register()` | **Already used as intended** (ADR-0073 option 2). Rust schemas pass through unchanged with `dereference_schemas=False`. Nothing to displace |
| `ToolResult.from_mcp_result` / `to_mcp_result` (`tools/base.py` 158–190) | "Wrap a protocol result while preserving its exact wire representation": keeps `_raw_mcp_result` and returns it verbatim. The constructor still re-serializes `structured_content` (`_serialize_to_jsonable`), which is an extra copy whose output is discarded | `SchemaTool.run`, failure path | **Already used.** This is the library hook that makes byte parity possible. Cost: one redundant Python copy of each packet (not measured) |
| `FastMCP(mask_error_details=True)`; `ToolError`/`ResourceError`/`ValidationError` pass through (`server/server.py` 373–377, 1575ff) | Default False; deliberate FastMCP errors are public; other exceptions are masked. A masked parent does not sanitize a mounted child (skill fm.errors.r04.1) | `server.py`, `wire.py` | **Already used.** Public text comes from Rust `wire_failure` and `StorageError.kind`, not driver text. Does not bound early framework errors (§11.3 says so) |
| Error result `ToolResult(is_error=True, meta=…)` vs JSON-RPC `MCPError(code, message, data)` | Tool error results carry metadata outside the success schema; resource failures need JSON-RPC errors | `admitted_failure`, `CapabilityResource.read` | **Already used** in exactly the split §11.3 requires |
| `ResourceTemplate` + `match_uri_template` (`resources/template.py` 83–101, `unquote`s params) | Template matching and percent-decoding owned by FastMCP; 4.0.6 changes literal/percent-encoding matching | `CapabilityTemplate` | **Already used.** Small residue: the template's parameter schema (`{"capability": string}`) is hand-written in `wire.py` while Rust owns `lctx://capability/{capability}` (`dispatch.rs:157`). Rust `wire_resources()` could emit it (ADR-0073 ownership), no library change |
| SDK writers: `mcp_types.methods.serialize_server_result`, `ServerRunner._stamp_server_info`, `resultType` default (`mcp/server/runner.py` 355–418); stdio `model_dump_json(by_alias, exclude_unset)` (`mcp/server/stdio.py:203`); modern HTTP compact `json.dumps` (`mcp/server/_streamable_http_modern.py` 163–198) | Public: `serialize_server_result`, `SERVER_INFO_META_KEY`, protocol-version sets. Private: the runner's stamping/`resultType` pipeline. `LowLevelServer.server_info_stamp` is public on the low-level server but reached only via FastMCP's private `_mcp_server` | `wire.response_encodings`, `negotiated_encodings` | **Deliberate mirror, not displaceable at the pin.** It pre-computes the exact final stdio and HTTP bytes under the original Rust grant so `encode_envelope` can admit them against `ResourceLimits` *before* the SDK writes. No FastMCP middleware or SDK hook sees final framed bytes (middleware runs before the runner serializes). The skill's own guidance says a hard byte bound needs "an exact serialized-envelope oracle" (fm.outputs.j01.1). Drift guard: `current_transport.py` asserts actual stdio/HTTP bytes equal the mirror (lines 160–212, 285–295, 650–834). One drift risk: `negotiated_encodings` rebuilds `Implementation(name, version, website_url, icons)` while the SDK uses `server_info` with `title`/`description` too; equal today because FastMCP 4.0.5 sets neither |
| `ResponseLimitingMiddleware` (`server/middleware/response_limiting.py`) | Lossy text replacement, hides output schemas, drops structured output; `max_size` is not a final-bytes bound (skill fm.outputs.j01.1) | none | **Rejected by contract** (§11.3: "cannot drop semantic structured output"). Correctly absent |
| `ResponseCachingMiddleware` (`server/middleware/caching.py`) | When installed, caches successful tool calls ~1 h regardless of `readOnlyHint`; keys omit session and domain state | none | **Rejected by contract** (§11.3: caching "cannot conceal degraded availability"). Correctly absent |
| `FastMCP(cache_ttl=, cache_scope=)` → SEP-2549 hints (`server/caching.py`) | Uniform server-wide `ttlMs`/`cacheScope` on `tools/list`, `resources/list`, `resources/templates/list`, `resources/read`, `server/discover`; inert unless the client opts in on `2026-07-28`; error results not hinted | none | **Possible enhancement, no consumer.** One generation per process makes list results immutable for the process lifetime. It would *not* fit `resources/read` if a degraded/refused read could be cached (it would add `ttlMs` to the byte-mirrored envelope as well), and it is uniform, not per-method. Only worth it with a named client-side consumer |
| `list_page_size` (`server/server.py` 380–383) | MCP list pagination (`cursor`/`nextCursor`) for list methods only | none | **No overlap**: 10 tools and 1 template. MCP has no protocol pagination for `tools/call` results |
| Lifespan: plain `@asynccontextmanager`; `fastmcp.server.lifespan` composition (`lhs | rhs`), `Context.lifespan_context` (`server/context.py:418`), `Shared`/`Depends` | Composition merges yielded dicts; `Shared` follows the root lifespan; custom `Tool.run` does not use dependency injection | `server.build_server`, `generation.open_generation` | **Already the idiomatic minimum.** One resource, one context manager, shutdown in `finally`. Composition/`Shared` would add nothing until a second lifespan resource exists |
| Middleware: `ErrorHandlingMiddleware`, `TimingMiddleware`, `LoggingMiddleware`, `RateLimitingMiddleware`; native OTel spans (`server/telemetry.py`; `opentelemetry-api` comes with mcp) | Interception before serialization; rate limiting is per server, not per grant | none | **Optional, no consumer.** Native grant admission/deadline already bounds concurrency; rate limiting would duplicate it. OTel spans are an enhancement only if a tracing consumer appears |
| `httpx2.AsyncClient` (via mcp; `AsyncHTTPTransport(retries=)` retries connect only) | Connection pooling requires a long-lived client | `embedder.HttpEmbedder.embed` creates a client per call | **Minor**: hold one client for the lifespan (pooling). Retries are correctly absent under a request deadline |
| pydantic strict models | Strict JSON validation | `embedder._Embeddings/_Datum` | Already used; mirrors Rust serde DTO. See Q2 for the bigger duplication |
| pyo3 `create_exception!` (`serving.rs:19`) + `setattr("kind")` under `Python::attach` (`serving.rs:32–38`) | `create_exception!` defines a class deriving only from the base; no fields. `#[pyclass(extends=PyException)]` with `#[pyo3(get)] kind` is the guide's route for structured exceptions (`pyo3/guide/exception.md` 131–170) | `StorageError`; read by `wire.public_failure` via `getattr(exc, "kind")` | **Small optional tidy-up.** A typed `kind` getter would replace the attach-and-setattr step and `getattr` probing. It would need a constructor and a test that `StorageError` still subclasses `RuntimeError`; behavior otherwise equal |
| `pyo3_async_runtimes::tokio::future_into_py` + `init(builder)` once (`lib.rs` 7–14) | Cancelling the asyncio future drops the Rust future (tokio.rs 280–382); init after first use is ignored (skill B037) | every `Service` method | **Already used correctly.** Cancellation-retains-slot comes from `RequestExecution::cpu`/`query` spawning owned tasks (`crates/lctx-postgres/src/generations/runtime.rs` 227–270), which is the right owner. `into_future` (Python awaitable → Rust) is unused because callbacks are synchronous; correct |
| `Python::attach`/`detach`, `Py<T>`/`Bound` | 0.29 names; `Bound` is `!Send` | `serving.rs`, `lctx_semantics/src/lib.rs` | Current API throughout; no obsolete names seen |
| pythonize 0.29.0 (crates.io 2026-08-28, pyo3 0.29 line) — **absent** from `Cargo.lock` | serde ↔ Python objects without a JSON text step | would touch `encode_request`, `preflight_json` (`serving.rs` 135–290), and the JSON-string returns of `lctx_semantics` | **Low fit.** `preflight_json` is a resource-admission lower bound (cycle detection, budgeted traversal) over the request's JSON byte size, which is the declared limit's unit. pythonize materializes without a budget and would lose the byte measurement. String returns from `lctx_semantics` are one `json.loads` each at startup |
| `hex` 0.4.3 / `base64` 0.22.1 (workspace pin) / `data-encoding` 2.11.1 — present in `Cargo.lock` | Standard encoders | `cursor.rs` 62–93 hand-rolls hex encode/decode | **Trivial displacement** (~25 lines). Not a semantics change: the token stays opaque and binding-checked |

## Q1 — What `wire.py` re-implements, and what is deliberate

What it re-implements: the SDK's final-result shaping (`serialize_server_result` is called, but
`serverInfo` stamping and `JSONRPCResponse`/`JSONRPCError` framing for stdio and modern HTTP are
re-done) and the server identity dump. Everything else is FastMCP surface used as designed:
custom `Tool`/`Resource`/`ResourceTemplate` subclasses, Rust-supplied `parameters`/`output_schema`,
`ToolAnnotations`, `ToolResult.from_mcp_result`, `mask_error_details`, `ToolError`/`MCPError`.

Deliberate contracts, with sources:
- Rust owns request/response schemas and dispatch; Python is a thin adapter, "custom Tool
  validation is explicit" (ADR-0073 Decision; §11.3 first paragraph).
- "Request and final SDK codecs execute with precharged scratch under the original Rust
  execution grant and cumulative deadline" (§11.3). This is why the encodings are computed
  inside `service.encode_envelope` and why they must be byte-exact: admission is against the
  bytes the SDK will actually write. Tests pin both encodings to observed transport bytes.
- "Response limiting middleware cannot drop semantic structured output, and response caching
  cannot conceal degraded availability" (§11.3): excludes the two FastMCP middlewares that would
  otherwise look like built-ins for this job.
- Failure envelopes: one attempt on the original grant; tool failure in `_meta.lctx_failure`
  outside the success schema; resource failure as JSON-RPC error data (§11.3 last paragraph).
- ADR-0068 does not exist at this baseline (no `docs/adr/0068-*`); the relevant records are
  ADR-0073, ADR-0025, ADR-0049 and ADR-0114/0116 as listed under §11.3.

No library facility at the pin offers the final-bytes measurement: FastMCP middleware sees
`ToolResult` before the runner serializes; the SDK runner and transports have no pre-write hook.
**Interpretation:** the mirror is the minimum bespoke code for the stated contract. The residual
risks are drift on SDK upgrades (guarded by the byte-equality tests, which need a live fixture)
and the hand-built `Implementation`. A slightly tighter source is
`server._mcp_server.server_info_stamp` (exact SDK value, private attribute path) — a trade of
private access for exactness; the coordinator decides.

Duplication outside libraries noticed in passing (DP-13 ownership, not library leverage):
`lctx_storage.Service.tools/resources/schema` (`serving.rs` 999–1008) duplicate
`lctx_semantics.wire_tools/wire_resources/wire_schema`; no Python consumer of the storage copies
was found (`grep` over `python/`, `scripts/`, `crates/`).

## Q2 — `embedder.py`, `retrieval.py` versus `lctx-embed`

`embedder.py` reproduces `lctx-embed` (`crates/lctx-embed/src/lib.rs`): `request_body` (lib.rs:43),
`parse_embeddings` (lib.rs:71), `check_vector` (cpg-core), the Qwen/fake spec loading and the
splitmix64 fake embedder, and is held to shared corpora under `specs/embedding/`
(`request_bodies.json`, `responses.json`, `fake_vectors.json`). The module docstrings state the
parity intent. Differences: Rust uses `reqwest` with connect 10 s / request 300 s timeouts and has
`count_tokens` via `/tokenize`; Python uses a fresh `httpx2.AsyncClient` per call, a 10 s
timeout plus `asyncio.timeout(grant.remaining_seconds())`, and no token admission or cache.

Library view: nothing in FastMCP/pydantic/httpx2 removes this duplication; it is an ownership
question. The process already hosts a tokio runtime in `lctx_storage`, and `lctx-embed` already
implements the same client in Rust. Running the query embedding inside the Rust service (under
the grant deadline, `tokio::time::timeout_at`) would remove the Python client, its parity
corpora's second consumer and the fake twin. Costs to weigh: `lctx-embed` depends on `cpg-core`
and `reqwest`, pulling them into the `lctx_storage` cdylib closure (AGENTS.md keeps Python
bindings outside the Hakari closure, so this is a build-closure decision, not a blocker), and
Rust's current timeouts are compile-time values (300 s), not request-deadline-bound.

Uncertainty to check, not a finding: §11.1 says "token admission precedes service/cache effects".
The query path does no `/tokenize` call; the 500-Unicode-scalar text bound (ADR-0073) may be the
intended admission, but I found no statement equating them.

`retrieval.py` is a thin adapter over bm25s 0.3.11 with Rust owning tokenization and fusion; no
FastMCP/pydantic overlap. bm25s itself is outside this lane's libraries.

## Q3 — Generation pinning, lifespan and shutdown

`server.build_server` uses a single FastMCP lifespan that opens the Rust service, yields
`{"served": Generation}` and awaits `service.shutdown()` in `finally`; tools read
`ctx.lifespan_context["served"]` (public `Context.lifespan_context`). `open_generation` verifies
the embedding spec and initializes the numerical index under a grant, and shuts the service
down on any failure. FastMCP 4.0.5 offers lifespan composition (`fastmcp.server.lifespan`,
`|` operator), `Shared` dependencies and session state (`ctx.get_state/set_state`), none of which
fit better: there is one server-owned resource, and session state is read-modify-write without
transactional guarantees (skill). The generation pin itself is Rust's (startup selection plus
lease); FastMCP has no notion of it. **No displacement.**

## Q4 — PyO3 bridge

Current API throughout (`attach`/`detach`, `Bound`, `cast`, `future_into_py`, `create_exception!`,
`#[pyclass(module=…, frozen)]`, `#[pyo3(signature=…)]`); no obsolete names. Hand-written parts:
- Exception `kind` via `setattr` after construction — `#[pyclass(extends=PyRuntimeError)]` with a
  getter is the library route (table row). Optional.
- `preflight_json` (≈150 lines) — a bounded, budget-charged lower bound on the JSON size of the
  incoming arguments with cycle detection. No pyo3 or pythonize facility provides budgeted
  traversal; keep.
- Envelope/encoder callbacks take Python callables and check exact builtin types — deliberate
  (untrusted values inside a CPU grant).
- JSON strings across the boundary instead of `IntoPyObject` structs — deliberate canonical
  bytes (e.g. `wire_tool_result`, `canonical_embedding_spec`); pythonize is absent and low fit.
- `pyo3_async_runtimes::tokio::init` in a `Once` — matches the skill's "configure once before first
  use" rule.

## Q5 — Pagination and cursors

`cursor.rs` continuations are tool-result continuations bound to generation, canonical request,
policy, wire identity, channels, group, section, member and ordering; decoding refuses any change.
MCP/FastMCP pagination (`cursor`/`nextCursor`, `list_page_size`) applies only to list methods and
is opaque, unbound and not available for `tools/call` results. **No overlap; the difference is
deliberate** (§11.3: "Cursors bind generation, canonical request, section/group, policy and
representation"). Only the hex codec is replaceable by `hex`/`base64` already in the lock.

## Q6 — Upgrade deltas (findings only; checked 2026-10-04)

| Package | Pin | Latest (date) | Relevant to these consumers |
|---|---|---|---|
| fastmcp / fastmcp-slim | 4.0.5 | 4.0.11 (2026-10-04) | 4.0.6–4.0.9: URI templates match raw or percent-encoded literals, compiled-pattern caching (affects `CapabilityTemplate` matching; re-check `lctx://capability/{capability}` with encoded names). 4.0.11: security fixes (schema-adapter keying, hashed tool lookups apply transforms/auth, Host/Origin on SSE), "keep $ref when inlining would be oversized" (inert with `dereference_schemas=False`), client decodes empty structured content. Upstream: "all users are encouraged to upgrade". mcp range `>=2.0,<3.0` unchanged |
| mcp / mcp-types | 2.2.0 | 2.3.0 (2026-10-02) | Behavior changes that could move bytes: empty `_meta`/`params` no longer sent on legacy requests (request side); `initialize` omits empty `experimental`; explicit `structuredContent: null` checked against output schema. Requires `httpx2>=2.10` (pinned 2.13.1 satisfies). Any upgrade must re-run the byte-equality transport tests because `wire.py` mirrors the runner |
| httpx2 | 2.13.1 | 2.13.1 | current |
| pyo3 | 0.29.2 | 0.29.3 (2026-09-30) | Patch: Windows linking, GraalPy, perf, `experimental-inspect` stubs, FFI/crash fixes; 0.30 announced "within a few weeks" |
| pyo3-async-runtimes | 0.29.0 | 0.29.0 | current |
| pythonize | absent | 0.29.0 (2026-08-28) | Only if Q4's low-fit option is pursued |

Versions from PyPI JSON and crates.io API; notes from GitHub releases (`gh release view`).

## Uncertainties, absences and search coverage

- Absence of a pre-write serialization hook: searched `mcp/server/{runner,stdio,streamable_http,
  _streamable_http_modern,lowlevel/server}.py` and `fastmcp/server/{server,low_level}.py`,
  `server/middleware/*` at the pinned install. Not searched: every private SDK class. A hook could
  exist undocumented; none is public.
- Absence of server-side output-schema validation: `mcp/server/validation.py`, grep for
  `output_schema|outputSchema` in `mcp/server/lowlevel` and `fastmcp/server/server.py`. mcp 2.3.0
  notes mention output-schema checking of explicit `null` (appears client-side; not verified).
- Unused `Service.tools/resources/schema`: grep over `python/`, `scripts/`, `crates/` for
  `service.tools|service.resources|service.schema`; dynamic access not excluded.
- The redundant `structured_content` copy in `ToolResult.__init__` is source-observed; its cost
  is not measured (performance is not a design gate).
- Token admission on the query path (Q2) is an open reading of §11.1, not a verified gap.
- Not covered: bm25s, numpy, pyarrow (outside FastMCP/MCP/PyO3); the HTTP transport is exercised
  only by tests (production is stdio, `__main__.py`).
