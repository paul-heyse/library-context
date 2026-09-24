# Structured evaluation — Stage 1 (the whole public surface)

**Set:** `eval/behavior/fastmcp-4.0.5.toml`, pre-registered in `2fb10b8`, before any Stage 1 output
of FastMCP was read. Its Stage 1 questions are Q13–Q20: controls and handoffs of operations that
have **no brief**.

**Served generation:** `0ed0c3b183080b50`, snapshot `4dcb40b00a9550ccb88a53a60f9a82a4`, pilot
2026-09-24, fake embedder. The tools read are `get_operation`'s records. A class carries its
constructor.

**Packet:** `just structured-eval build/generations/0ed0c3b183080b50 1`, which writes
`build/structured/stage1.md`.

**Assessor:** the author (Claude), rating each item against the packet. The operator reviews this
assessment (ADR-0021). No API agent was used.

**Rubric.** Positive items are rated `present` / `partial` / `absent` / `incorrect` /
`misleading`. For a negative item, the question is whether the served answer claims it.

**Baseline.** None of these operations has a brief. The capability compiler's default answers
none of these questions, so every present or partial rating below is a gain over briefs.

## Ratings

| Item | Rating | What the served answer shows |
|---|---|---|
| Q13.a `arguments` `or {}` → `call_tool_mcp` → `ClientSession.call_tool` | partial | `unfollowed` (computed) into `call_tool_mcp`, verdict `unknown`, with the `arguments or {}` site. The SDK hop is past the release and the depth |
| Q13.b `version` merged into `meta['fastmcp']` | absent | No fate; a dict merge is Stage 2's value flow |
| Q13.c `timeout` normalized, then `read_timeout_seconds` | partial | Forwards to `normalize_timeout_to_seconds` and to `call_tool_mcp.timeout`; the SDK kwarg is not reached |
| Q13.d `progress_handler`, falling back to the constructor's | partial | Forwards to `call_tool_mcp.progress_handler`; the field fallback is Stage 2 |
| Q13.e `meta` through `inject_trace_context` | absent | No fate |
| Q13.f `raise_on_error` used only in parsing | partial | Forwards only to `_parse_call_tool_result`, twice; "never sent to the server" and the `ToolError` are not stated |
| Q13.g (negative) `raise_on_error=False` suppresses transport errors | not claimed ✓ | — |
| Q14.a host/port fall back to settings, then `uvicorn.Config` | absent | `host` `unfollowed` (rebound, `unknown`), `port` no fate. This is the fallback pattern |
| Q14.b path, transport, middleware, json_response, allowed_origins → `http_app` | partial | Four established forwards to `http_app`. `path` is `unknown` (rebound): the name is bound again later in the body, a conservative flow-insensitive miss |
| Q14.c `stateless_http` resolution | absent | `unfollowed` (rebound, `unknown`) |
| Q14.d `host_origin_protection` and `allowed_hosts` | partial | `allowed_hosts` → `_resolve_allowed_hosts_for_run`; `host_origin_protection` no fate |
| Q14.e `uvicorn_config` merged over defaults | absent | No fate |
| Q14.f `log_level` | partial | → `temporary_log_level`; the uvicorn fallback is not shown |
| Q14.g `show_banner` only gates the banner | absent | No fate. The `log_server_banner` delegation is listed but not tied to it |
| Q15.a `transport` via `infer_transport` | partial | → `infer_transport` (established), with depth-2 flows into the transport constructors, some conditional, some `unknown` |
| Q15.b `timeout` → session `read_timeout_seconds` | partial | → `normalize_timeout_to_seconds`; the session kwarg (a field) is Stage 2 |
| Q15.c `init_timeout` falls back to settings | absent | `unfollowed` (rebound) |
| Q15.d `log_handler` → `create_log_callback` | partial | `unfollowed` (rebound) into `create_log_callback`: the callee is right, the verdict `unknown` |
| Q15.e `verify` assigned to the transport or `ValueError` | absent | No fate |
| Q15.f `progress_handler` stored on the client | absent | No fate. No false claim either |
| Q15.g `name` only in log messages | absent | No fate |
| Q16.a eight parameters → `OpenAPIProvider(...)` unchanged | **present** | All eight established forwards to `OpenAPIProvider.__init__`, at `server.py:2441` |
| Q16.b `name` → the FastMCP constructor | **present** | Established forward to `FastMCP.__init__.name` |
| Q16.c `providers=[provider]` | partial | Both constructors listed as delegations; the list appears only in the site text |
| Q16.d `**settings` → the FastMCP constructor | partial | `unfollowed` (unmapped) into `FastMCP.__init__`, `unknown`; the `TypeError` is not stated |
| Q16.e an auto-created client when `client=None` | partial | A conditional forward of `openapi_spec` to `_create_default_client` |
| Q16.f only an auto-created client is closed | absent | — |
| Q16.g (negative) `**settings` → `OpenAPIProvider` | not claimed ✓ | The served callee is `FastMCP.__init__` |
| Q17.a–f `read_resource`'s URI, version and meta, its return, the not-found error | absent (6) | `uri` `unfollowed` (rebound) into `read_resource_mcp`; `version` and `meta` no fate. Returns and exceptions are Stage 2–3 |
| Q17.g (negative) `read_resource` takes `timeout` / `raise_on_error` | not claimed ✓ | The parameter list shows `uri`, `version`, `meta` only |
| Q18.a `show_banner` falls back to settings | absent | `unfollowed` (rebound) |
| Q18.b `transport` falls back to settings | absent | `unfollowed` (rebound) |
| Q18.c an unknown transport raises `ValueError` | absent | The guard tests a rebound name, so it is declined |
| Q18.d stdio → `run_stdio_async(...)` | partial | `show_banner` and `transport_kwargs` into `run_stdio_async`, `unknown`; a candidate delegation |
| Q18.e http/sse → `run_http_async(...)` | partial | Likewise into `run_http_async` |
| Q18.f (negative) unaccepted kwargs silently dropped | not claimed ✓ | `transport_kwargs` is `unknown` (unmapped), not "dropped" |
| Q19.a docs pass `FastMCP(...)` into `Client(...)` | **present** | `Client.__init__` `takes_from` `FastMCP.__init__`, formal `transport`, ×49, cited to `docs/clients/client-only-package.mdx`, code block 7: `Client(server)` |
| Q19.b `infer_transport` wraps it in `FastMCPTransport` | partial | → `infer_transport`, then `unknown` into `FastMCPTransport.__init__` at `inference.py:147` |
| Q19.c `FastMCPTransport` enters the server's lifespan, in memory | absent | — |
| Q19.d (negative) `Client(mcp)` starts HTTP or a subprocess | not claimed ✓ | — |
| Q20.a docs pass `@lifespan`'s `Lifespan` into `FastMCP(lifespan=...)` | absent | The value comes from a decorator, which Pass C does not read as a producer. `lifespan` does show `fn` → `Lifespan(fn)` |
| Q20.b a `ComposedLifespan` via `|` | absent | — |
| Q20.c FastMCP stores and calls it | absent | `lifespan` no fate (a field store) |
| Q20.d a plain context manager, and wrapping for `|` | absent | — |
| Q20.e (negative) `|` without wrapping | not claimed ✓ | — |

## Tally

| Items | Present | Partial | Absent | Incorrect | Misleading |
|---|---|---|---|---|---|
| 44 positive | 3 | 16 | 25 | **0** | **0** |
| 6 negative | 6 not claimed | | | 0 claimed | |

## What the evaluation shows

**Stage 1's exit criterion is met in part.** Controls and handoffs are answered for operations with
no brief where the analysis reaches:
- unchanged forwarding (Q16.a, Q16.b present);
- official-usage handoffs from a call (Q19.a present);
- 16 items in part.

Nothing is incorrect, and no negative item is claimed. ADR-0021's revisit trigger ("the
whole-surface tools answer no question the briefs could not") **did not fire**.

**Why items are absent, by cause** (this orders Stage 2):

| Cause | Items | Examples | Owner |
|---|---|---|---|
| A parameter rebound by a fallback (`x = x if x is not None else settings.x`), which Stage 1 declines | ~10 | Q14.a, Q14.c, Q15.c, Q17.a, Q18.a–c | Stage 2's flow-sensitive def-use (reaching definitions and the settings places) |
| A value stored in a field (`self._x = x`), or merged into a dict | ~9 | Q13.b, Q13.e, Q15.e–g, Q20.c | Stage 2's field and value flows |
| An exception or return behavior | ~4 | Q16.f, Q17.e, Q17.f, Q19.c | Stage 2–3 (exits, handlers, summaries) |
| A value produced by a decorator, a handoff Pass C does not recognize | 1–2 | Q20.a–b | Stage 5 (protocols and usage patterns), or a Pass C extension |
| The SDK past the release boundary | several partials | Q13.a, Q13.c | Stage 3's models for the MCP SDK |

**Precision notes**
- `path` in `run_http_async` is `unknown` (rebound). The name is rebound later in the body; the
  forward happens earlier, unchanged. A conservative miss, not an error; Stage 2 fixes it.
- `self.method(...)` delegations are `unknown` (candidate), because override dispatch is open
  (§3.6). This is correct under the model, but it is common, and it lowers what `delegates_to`
  can say.

**Recommendation for the operator's review.** Take Stage 2's order from the cause table:
fallback def-use and settings places first, then fields and dict values. This agrees with the
review's Appendix B, where branch predicates, field flow and summaries dominate.
