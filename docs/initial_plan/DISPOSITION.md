# Initial_plan disposition ledger

For every level-1 and level-2 heading of `Initial_plan.md`, this records where it landed in
`docs/design/DESIGN.md`, or why it did not. It exists so that condensing the research input can't
silently drop a rule (baseline review finding F2).

**Status key**
- **adopted**: taken as is.
- **adapted**: taken with changes; the ADR says what changed and why.
- **deferred**: listed in DESIGN §13 with a trigger.
- **rejected**: an ADR gives the reason.
- **context**: background or argument; nothing to carry.

This is history, not authority: DESIGN.md wins where they differ. Update a row whenever a
DESIGN section or ADR changes what it points at. Line numbers refer to the 2026-09-22 version
(commit `8bba05c`).

## Part I: analyzer audit and ontology (L1–L917)

| Line | Heading | Status | Where |
|---|---|---|---|
| 1 | Conclusion | context | — |
| 22 | 1. Audit scope and the meaning of "available" | adopted | exposed/internal/derived distinction → DESIGN §4.2 |
| 47 | 2. What Ruff can contribute | adapted | ADR-0006 |
| 49 | 2.1 Source, tokens, syntax, and exact structural relationships | adopted | §4.2, §3.4; exhaustive-exporter contract applies to the `syntax` family |
| 99 | 2.2 Lexical scopes, bindings, references, and execution context | adapted | our own recognizer (§4.2); Ruff semantic-model port deferred (§13) |
| 156 | 2.3 Ruff's semantic extraction is not a standalone public pass | adopted | reason for the §4.2 recognizer (ADR-0006) |
| 168 | 2.4 Module dependencies, diagnostics, and supporting crates | deferred | not needed by v1 passes (§13) |
| 178 | 2.5 The critical CFG limitation | adopted | CFG deferred (§1.3, §13) |
| 192 | 3. What Pyrefly can contribute | adapted | CLI reports, not native state (ADR-0006) |
| 194 | 3.1 There are several distinct extraction surfaces | adapted | Pysa + coverage report in increment 1; Glean on demand (§4.2) |
| 209 | 3.2 Glean: the navigation and source-reference layer | deferred | §13, until a consumer needs cross-references |
| 233 | 3.3 Pysa: call-resolution and object-model export | adopted | §4.2, §3.6 (the `ifCalled` and synthetic-shim rules) |
| 290 | 3.4 CinderX: structured types beyond display strings | deferred | §13 |
| 334 | 3.5 The maximal native type layer | deferred | §13; requires native linking, which ADR-0006 rejects for now |
| 363 | 3.6 Rich class metadata and synthesized behavior | deferred | §13 |
| 384 | 3.7 Binding IR, narrowing, inference dependencies, and answers | deferred | §13; the "not runtime dataflow" rule kept in §B5 |
| 410 | 3.8 Query interfaces: useful, but verify their semantics | adopted | `getDeclaredType` rule in §3.5.1; TSP deferred (§13) |
| 427 | 4. The maximal graph obtained by synthesizing these sources | adapted | target model; demand-driven slice (ADR-0004, ADR-0008) |
| 435 | 4.1 The ontology should distinguish entities that are often incorrectly merged | adopted | §3.1 |
| 469 | 4.2 Exact core schema | adapted | authoritative family tables, one producer each; generic `nodes`/`edges` views deferred until a reader exists (ADR-0008) |
| 576 | 4.3 Type observations must be contextual and multi-valued | adopted | §3.5.1; the `types` family from increment 3 |
| 625 | 4.4 Resolution is a set with uncertainty, not a single edge | adopted | §3.6 |
| 657 | 4.5 Relation families | adapted | fact families (§3.2); the 94-edge registry is not needed yet |
| 678 | 4.6 Preserve unnormalized native detail without making JSON blobs the database | deferred | `record_fields` (§13) |
| 719 | 4.7 Identity and evidence rules | adopted | §3.4, §3.4.1, §3.7 |
| 747 | 5. How this fits your Rust architecture | adapted | one workspace plus a pyrefly subprocess (ADR-0006) |
| 806 | 6. Remaining gaps and whether they justify more Rust libraries | adopted | §B1 |
| 808 | 6.1 Gaps that do not justify another parser or type checker | adopted | §B1 |
| 821 | 6.2 Gaps requiring genuinely new semantic analysis | deferred | §13; native boundary → `boundary_reason` (§3.5) |
| 886 | Recommendation | adapted | §B1 (ADR-0006) |

## Part II: the Arrow / DataFusion / petgraph construction (L918–L1649)

| Line | Heading | Status | Where |
|---|---|---|---|
| 918 | 1. The division of responsibility | adapted | §B2–§B5, §4.1 |
| 955 | Recommended ownership | adapted | §4.1 stage owners |
| 975 | 2. Make Arrow schemas authoritative | adopted | §B2 |
| 977 | 2.1 Separate the logical ontology from physical representation | adopted | §3.3 |
| 1034 | 2.2 Typed Arrow does not eliminate semantic validation | adopted | §8 |
| 1066 | 2.3 Core Arrow tables | adapted | family tables by increment (§3.2, ADR-0008) |
| 1121 | 2.4 Use nested Arrow selectively | adopted | child tables for arguments and parameters (§3.2) |
| 1144 | 3. Construct the canonical graph with Arrow and DataFusion | adopted | §4.1 |
| 1146 | Stage A: Establish the source and analysis universe | adopted | §4.0, §4.1 |
| 1172 | Stage B: Emit typed provider facts | adapted | §4.2 (report decoders instead of native adapters) |
| 1208 | Stage C: Resolve provider-local identities | adopted | §4.1, §3.4 |
| 1257 | Stage D: Construct the semantic relationships relationally | adopted | §4.1, §B3 |
| 1294 | Stage E: Construct the caller-to-callee projection | adopted | §5 |
| 1335 | 4. The Arrow → petgraph boundary | adopted | §5 |
| 1337 | 4.1 Construct a small, explicitly defined projection | adopted | §5 |
| 1396 | 4.2 Use compact graph weights | adapted | `Graph<(), ArcRow, Directed, u32>` (ADR-0011) |
| 1416 | 5. What petgraph should actually compute | adapted | §B4, with named owners (ADR-0011) |
| 1418 | 5.1 Native topology algorithms | adapted | traversal, SCC, `page_rank`; dominators deferred (§13) |
| 1434 | 5.2 SCC condensation is a useful hybrid operation | deferred | §13 |
| 1472 | 5.3 Return graph results to Arrow immediately | adopted | §5, §9 (findings go back to Arrow) |
| 1505 | 6. What requires custom Rust analysis rather than merely petgraph | adopted | §B5 |
| 1507 | 6.1 Full Python control flow | deferred | §13 |
| 1529 | 6.2 Reaching definitions and value flow | deferred | §13; v1 uses the conservative `ambiguous_binding` rule (§4.2) |
| 1565 | 7. Execution, validation, and Delta persistence | adopted | §6, §8 |
| 1567 | 7.1 DataFusion should own substantial relational execution | adopted | §B3 |
| 1581 | 7.2 Validation needs two levels | adopted | §8 |
| 1603 | 7.3 Delta is a separate physical boundary | adapted | §3.3, §6 (publication via a `snapshots` append, ADR-0009) |
| 1624 | 7.4 Pin a coherent Rust dependency family | adopted | §7, ADR-0002 |
| 1634 | Recommended implementation sequence | adapted | replaced by the increments in §1.2 (ADR-0004) |

## Part III: the first product, capability briefs (L1650–L2172)

| Line | Heading | Status | Where |
|---|---|---|---|
| 1658 | 1. The first product: capability discovery and usage briefs | adopted | §1.1 |
| 1695 | 2. Make the capability brief the unit of interpreted output | adopted | §10.3 |
| 1735 | 3. Keep three analytics passes mandatory | adopted | §9.1–§9.3 |
| 1743 | Pass A: Public entry point and delegation analysis | adopted | §9.1 |
| 1785 | Pass B: Configuration and local restriction analysis | adopted | §9.2 |
| 1833 | Pass C: Direct handoff analysis | adopted | §9.3 |
| 1886 | 4. Put an explicit interpretation stage after the analytics | adapted | programmatic synthesis, no LLM (ADR-0005) |
| 1932 | Give the interpreter a constrained job | adapted | assertion structure kept (§10.2); a template instead of an LLM; the kind policy and status propagation are new |
| 1949 | Separate grounding from truth verification | adopted | §10.2 evidence statuses, §10.4, including the one-pass manual review from increment 3 (IP L1965) |
| 1969 | 5. Keep the storage and execution design small | adopted | ADR-0004, ADR-0008 |
| 1971 | Reuse the Arrow foundation, but implement only the required slice | adopted | §3.2 families by increment |
| 1992 | Add six enrichment table families | adapted | the `findings` family (§3.2); embeddings in the global `embedding_cache` Delta table, copied into the bundle (§6.4) |
| 2027 | Execution ownership | adapted | §4.1; the LLM role is removed (ADR-0005) |
| 2037 | 6. Simplify retrieval more aggressively than interpretation | adopted | §11.2 |
| 2073 | What to defer | adapted | community detection and FCA/RCA brought **into** v1 by operator direction (ADR-0005); the rest in §13 |
| 2088 | 7. Make the analytics requirement part of "done" | adopted | §1.5 |
| 2092 | The first deliverable must demonstrate all three derivation families | adopted | §1.5 |
| 2116 | Evaluate the interpretation, not just retrieval | adopted | §12 |
| 2135 | Recommended first implementation | adapted | ADR-0004 (pilot FastMCP, not PyArrow; no LLM; LanceDB deferred) |

## Part IV: implementation handoff (L2173–L3085)

| Line | Heading | Status | Where |
|---|---|---|---|
| 2173 | 1. Architecture and the key integration decision | adapted | §B12, §B13 |
| 2175 | Keep compilation and retrieval separate | adopted | §B12, §6.4 |
| 2217 | A verified compatibility issue: LanceDB is not on the same dependency train | adopted | the reason LanceDB stays out of Rust (ADR-0010) |
| 2233 | 2. Configure petgraph around evidence, not general graph exploration | adopted | §5 |
| 2235 | 2.1 Use a lightweight immutable directed multigraph | adopted | §5 |
| 2266 | 2.2 Build the invocation projection relationally | adopted | §5 |
| 2312 | 2.3 Use bounded traversal with explicit witnesses | adopted | §9.1; sorted adjacency, because petgraph's iteration order is newest-first (ADR-0011) |
| 2366 | 3. Detailed design of the three analytics passes | adopted | §9.1–§9.3 |
| 2368 | 3.1 Pass A: public entry point and delegation | adopted | §9.1 |
| 2420 | 3.2 Pass B: configuration propagation and local restrictions | adapted | §9.2 with the `ambiguous_binding` rule (§4.2) |
| 2514 | 3.3 Pass C: direct producer-to-consumer handoffs | adopted | §9.3, §10.5 |
| 2580 | 4. Make findings, interpretation, and publication distinct stages | adapted | §10 (programmatic; ADR-0005) |
| 2582 | A finding is not a generated claim | adopted | §10.1 |
| 2605 | Constrain the interpreter | adapted | grounding checks and the manual review of the initial corpus kept (§10.4, IP L2634); the LLM is deferred (§B11) |
| 2640 | 5. Embedding design | adopted | §11.1 |
| 2642 | 5.1 Model choice | adopted | Qwen3-Embedding-4B, 2,560 dimensions (§11.1) |
| 2664 | 5.2 Embed the interpreted capability, not the raw graph | adopted | §11.1 document text |
| 2703 | 5.3 Query/document asymmetry | adapted | fixed: no space after `Query:` (§11.1, ADR-0010) |
| 2718 | 5.4 Store an immutable embedding specification | adopted | §B14, §11.1 |
| 2744 | 5.5 vLLM and the Rust client | adapted | vLLM `--runner pooling`, no `dimensions`; Rust and Python clients checked against conformance vectors (ADR-0010) |
| 2766 | 6. Typed Arrow and LanceDB storage | adapted | the bundle holds briefs, vectors (copied from `embedding_cache`) and the symbol map (§6.4); LanceDB deferred |
| 2768 | 6.1 Keep canonical facts separate from search projections | adopted | §B12 |
| 2782 | 6.2 One row per published brief | adopted | bundle `briefs` + `vectors` (§6.4) |
| 2814 | 6.3 Separate lexical text from embedding text | adopted | §11.2 |
| 2841 | 7. Retrieval: use LanceDB's built-in capabilities | deferred | §13; in-process equivalent in §11.2 (ADR-0010) |
| 2843 | 7.1 Exact vector search first | adopted | §11.2 exact cosine |
| 2860 | 7.2 Native full-text search | adapted | in-process BM25 (§11.2) |
| 2878 | 7.3 Native reciprocal-rank fusion | adapted | in-process RRF, K = 60, with a tie-break (§11.2) |
| 2924 | 7.4 Hydration is deterministic | adopted | §11.2 |
| 2945 | 8. How DataFusion participates in retrieval | rejected | no DataFusion in serving (§B13, ADR-0010) |
| 2949 | V1: Arrow result integration | rejected | ADR-0010 |
| 2957 | Later: matched provider/plan integration | deferred | §13 (with LanceDB) |
| 2967 | 9. Publication, reproducibility, and operational behavior | adapted | §6 (ADR-0009) |
| 2969 | Publish immutable generations | adopted | §6.4 |
| 2988 | Make incomplete analysis visible | adopted | §3.7, the `boundary_reason` codebook (§3.5) |
| 3007 | Keep operational dependencies modest | adapted | `reqwest`/`tokio`/petgraph/leiden-rs; no `lancedb` crate (ADR-0010, ADR-0011) |
| 3026 | 10. Tests that establish the analytics are actually working | adopted | §12 |
| 3030 | Graph and recognizer fixtures | adopted | §12 plus added fixtures |
| 3045 | Interpretation checks | adapted | grounding checks (§10.4) applied to templates |
| 3051 | Embedding and database checks | adapted | §11.1 response checks, conformance vectors |
| 3057 | Product checks | adopted | §12 agent evaluation |
| 3073 | Recommended implementation order | adapted | §1.2 (ADR-0004) |
