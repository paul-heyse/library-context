# Serving discovery, packet bindings and safe failure metadata

**Proposed implementation plan · 2026-10-02.** This companion develops F07 and
opportunities §9.3/§9.5 of the [incremental review](../design_review/reviews/design_review_semantic-model-incremental-alignment_2026-10-02.md).
The [coordinator](semantic-model-incremental-alignment-plan_2026-10-02.md) owns current
disposition, shared identities and combined acceptance. No serving remediation is implemented
by publishing this document.

## 1. Baseline and intended result

Static inspection used main `42551010` plus the preserved working tree on 2026-10-02.
The model owns serving DTOs, finite routes, packet mappings, ranking and native responses.
The PostgreSQL service owns generation-bound hydration and admission. The Python bridge
translates validated objects; FastMCP supplies MCP transport. Preserve these boundaries,
the original generation guard, finite supported protocols, actual request IDs, all existing
success-envelope accounting and explicit uncertainty.

Authorities are [§15.12](../design/sections/semantic-model.md#section-15-12),
[§11](../design/sections/synthesis-and-serving.md#section-11),
[product target](../design/sections/api-and-evidence-product.md) and ADR-0114.
The target makes supported choices understandable through discovery, makes each packet's
dependencies visible at its typed binding, and adds safe programmatic metadata to failures
that have an admitted request and response envelope. It adds no packet query language,
transport implementation, retrieval feature or new protocol version.

Source routes are [schema.rs](../../crates/lctx-model/src/domain/serving/schema.rs),
[mappings.rs](../../crates/lctx-model/src/domain/serving/mappings.rs),
[catalog_service.rs](../../crates/lctx-postgres/src/generations/catalog_service.rs),
[bridge serving.rs](../../python/lctx_storage/src/serving.rs) and
[Python server](../../python/lctx_mcp/src/lctx_mcp/server.py) and
[wire adapter](../../python/lctx_mcp/src/lctx_mcp/wire.py).
Current packet mappings name an output type as a string separately from hydrators.
No concrete missing packet dependency or current wire identity collision was established.

## 2. Meaningful generated schemas — F07

Keep numeric codebooks and existing JSON Schema integer enums. Generate a description
containing every `code = variant label` pair from the existing FieldValue codebook operation.
Give model-owned DTO fields and finite route declarations ordinary documentation annotations
for their intended use, defaults, uncertainty, cursor constraints and support limits.
Codebook labels explain which choice exists; annotations explain when to choose it.
For example, Discovery preserves unsupported/unresolved results whereas Strict applies its
declared admission requirements. Do not infer that distinction from a variant name alone.

Carry those annotations mechanically into input/output schemas and tool descriptions.
Inventory every serving codebook-bearing field and every current tool/resource route; the
completion boundary is the finite serving inventory, rather than one illustrative tool.
Python registers the generated schemas directly and keeps schema dereferencing disabled.
It contains no independently maintained code-to-label dictionary or semantic help table.

Generated `description` plus the existing `enum` is selected over replacing enums with
`oneOf`/`title`: it preserves the current validation shape and is directly visible in the
discovery response without assuming how an agent renders alternative schema branches.
JSON Schema annotations permit this design; no dependency upgrade is needed. Actual client
rendering remains a client property, so acceptance inspects actual tools/list content and
correct semantic explanations, not a claim about every UI.

Keep closed fields, unknown-code rejection and integer-only encoding. String labels do not
become accepted wire values. Generated schema changes intentionally rotate WireIdentity;
model-owned source changes also rotate the captured model contract. Update source-written
schema expectations after inspecting changes, with no old-schema compatibility reader.

## 3. Typed packet bindings and attempted-read coverage — opportunity §9.3

This opportunity is included because output type strings, declared sources and actual hydration
have separate edit paths. Use one closed PacketBinding per existing packet output. Associate
the actual output type, stable packet identity, typed relation sources, explicit child bindings
and prepared-selection dependencies. Derive existing PacketMapping identity metadata from it.
Mapping declarations remain model-owned; executable SQL, canonical read checks and lifecycle
remain store-owned. Use ordinary typed declarations and functions, not a projection DSL.

Migrate the whole finite packet inventory, including composed packets and evidence children.
The ordinary store read helper receives the active binding and checks attempted typed relation
reads against its permitted sources. Record/check the attempt before iteration, so an empty
relation does not hide an undeclared dependency. A composed child contributes its declared
sources mechanically. Extra unused permitted rows are not proof of actual consumption.

Prepared catalog selection reads occur before an individual packet request. Declare that
prepared dependency explicitly and check its construction read scope against the declared
union when preparing the generation-bound selection owner. Request hydration uses the same
prepared owner and guard; it does not pretend those startup reads happened during the request.
Keep actual receipt and row-membership admission independent of binding agreement.

Use the current canonical typed read boundary for coverage rather than a second raw-SQL
interceptor. An empty attempted read, a direct read, a child packet and a prepared dependency
must each be visible. If a hydrator needs a source outside its binding, it refuses before
producing a packet. Declaring a source does not authorize an otherwise invalid epoch or lease.

Remove output-type string authorship and duplicated packet-source lists when their typed
replacement is integrated. Preserve stable packet IDs and all current output shapes. Source
dependency changes rotate their mapping identity; replacement of metadata with equivalent
typed lowering does not justify keeping an independently mutable duplicate. No omission is
claimed as a historical defect and no hydration speedup is claimed.

## 4. Safe structured failures within admitted envelopes — opportunity §9.5

This opportunity is included at a bounded scope. Introduce model-owned PublicFailure with
the bridge's existing coarse kinds: resource_refused, incompatible, corrupt and unavailable.
Each kind has a fixed, bounded public message. Preserve kind meaning; do not infer retryability
or disclose SQL, driver text, credentials, configuration, paths or raw exceptions.
Unexpected exceptions remain masked by the existing transport behavior.

For a recognized StorageError after acquiring the original request grant and admitting the
actual request ID, prepare the PublicFailure and use these existing protocol mechanisms:

| Route | Public transport result |
|---|---|
| Tool | CallToolResult with is_error=true, safe text and `_meta.lctx_failure` containing the typed payload |
| Resource | MCPError with the existing public error code/message semantics and ErrorData.data containing the typed payload |

Tool error metadata is not a success-schema value. Do not put it in structuredContent subject
to the tool's success output schema or return a resource-success object for a failure.
FastMCP clients that raise on tool errors still see safe text; programmatic clients using
raise_on_error=false can inspect metadata. Resource clients receive an ordinary JSON-RPC error.
No SDK internal monkeypatch or hand-written replacement transport is planned.

Before returning/raising this structured failure, use the original grant to admit the complete
error envelopes under both pinned writers/protocols, including actual ID, server information,
aliases and stdio/HTTP framing. Extend the existing envelope representation to distinguish
JSONRPCError from success responses; shared accounting must cover text and metadata once.
Include the failure schema/meaning in WireIdentity. Release all query, CPU and memory permits
through the current request lifecycle on success and failure.

Admission refusal may occur before a grant exists. ID admission or the failure envelope itself
may also refuse. Those cases use the existing safe ToolError/ResourceError path once, without
acquiring another grant or recursively trying to encode structured refusal metadata. They do
not acquire a new retry contract. Metadata on every early error and a universal byte bound on
SDK-generated early errors are **outside this increment's guarantee**. This is an explicit
existing qualification gap, not a reason to weaken admitted success-envelope accounting.

The scope follows pinned FastMCP 4.0.5/MCP 2.2.0 inspection, 2026-10-02. HTTP has a public ASGI
middleware seam and a body-size limiter; neither automatically bounds an echoed request ID.
Stdio exposes no small public per-request suppression/termination hook shared with HTTP.
An unbounded ID cannot be echoed unchanged inside a finite response bound. A transport-wide
early-error guarantee therefore needs a separate transport-owned request-limit/termination
design, triggered before advertising that guarantee or expanding supported transports.
Do not truncate IDs, force process exit or claim this plan settles that wider problem.

## 5. Packages and meaningful controls

| Package | Prerequisite | Responsibility and resulting capability |
|---|---|---|
| D0 discovery semantics | Existing codebooks/DTOs | Model/macros generate meaningful descriptions; bridge and actual discovery expose them; remove Python semantic duplicates |
| D1 packet bindings | Existing packet/selection owners | Model owns finite typed bindings; store migrates every hydrator and prepared read scope; metadata derives mechanically |
| D2 safe failure metadata | Existing admitted envelope/bridge contract | Model owns bounded payload; store/bridge account complete error envelopes; Python uses pinned tool/resource mechanisms |

D0 and D1 are logically independent. D2 shares schema/WireIdentity and bridge files with D0;
one integrator owns those surfaces and lands coordinated changes. Native request semantics
come from N0/N1, not from descriptions authored independently in this plan. Packet preparation
may share selection surfaces with N1/N2; preserve one writer and borrowed-data ownership.

During authorized implementation, run controls that can expose a wrong design:

- Compare generated numeric domains against append-only source-written code expectations;
  inspect meaningful field/route descriptions through actual tools/list. Unknown integer and
  string-label input still refuse. Schema descriptions and WireIdentity change together.
- For every packet, compare binding-declared sources with actual attempted reads through
  ordinary hydration. Include empty undeclared reads, composed children and startup prepared
  reads. Existing original-byte, tampered-row, wrong-epoch and guard-loss controls still apply.
- Through both protocols and stdio/HTTP, exercise all four recognized admitted failure kinds.
  Tool metadata and resource ErrorData.data retain kind; safe text remains usable by raising
  clients. Complete envelope bytes and actual IDs are admitted before emission.
- Exercise early capacity refusal, oversize ID refusal and failure-envelope refusal. Each
  retains the current safe path once, does not recurse, and drains the original grant if any.
  Secret-marked fake driver text and unexpected exceptions never appear in public payloads.
- Assert a payload that fits alone but exceeds the complete envelope bound refuses; schema
  agreement must not substitute for actual writer bytes or lifecycle qualification.

Reuse model serving-contract controls, real disposable PostgreSQL 18 services and actual MCP
transport fixtures. Mocked transports are not passing transport qualification. Targeted checks
run during packages; combined gates follow the coordinator. No checks in this section were
run during authoring. Performance, arbitrary client rendering and universal early-error bounds
are not claimed by these controls.

## 6. Library fit and completion boundary

Pinned installed sources settle API details; current Context7 documentation for FastMCP
corroborates available tool/resource error mechanisms but does not certify this exact pin.
JSON Schema description annotations and existing codebook methods avoid a new schema library.
Existing ToolResult/MCPError mechanisms avoid a parallel MCP transport. Typed packet bindings
reuse current canonical readers rather than adding a generic dependency framework.

Acceptance requires the complete finite migration and the independent cases above. D0 closes
F07 with actual meaningful discovery, not merely an annotated enum helper. D1 establishes an
extension seam without claiming a historical missing source. D2 establishes only its admitted
failure scope; retain the early-error limit at the coordinator and enduring serving owner.
Real-library journeys, retrieval quality and activation remain separately stopped.
