# Graph-native pivot — implementation completeness audit

**Revise: the runtime migration is substantially present, but the complete plan is not fully realized.**
Independent source inspection identifies missing semantic admission guarantees, incomplete executable
identity and cold auditing, lifecycle and serving defects, and retained execution structures that
contradict the pivot's performance-oriented design. These are implementation obligations within the
selected graph-native architecture. Native SurrealDB querying remains selected.

This is a dated implementation audit, not another store-choice review or release qualification.
Current finding disposition belongs exclusively to the [coordinator §7](../../plans/graph-native-pivot-plan_2026-10-05.md#7-sole-finding-disposition-and-optional-capabilities).
No remediation was performed during the audit.

## Baseline, contract and independent method

| Field | Audited boundary |
|---|---|
| Revision | Clean `main`, `b90cb611b6b11ff3c01fdadc7e199c765d28a8b6`, inspected 2026-10-06. Subsequent changes are audit documentation and diagnostic sources only. |
| Contract | [Coordinator](../../plans/graph-native-pivot-plan_2026-10-05.md) and its model/compiler, realization, projections and serving plans; G0, M1, C1, C2-N/C2-U, A1/A2, S1, P1/P2, S2/S3, I1 and Q0. Q1 obligations assessed without activation. |
| Architectural intent | Only the nominated [target review](design_review_graph-native-target_2026-10-05.md) and [capabilities review](design_review_surrealdb-capabilities_2026-10-05.md). Other reviews/receipts supplied historical leads, not additional architectural grounds. |
| Assessment lenses | [Core 3.3](../design_principles/core/design-principles.md), [efficient architecture heuristics](../design_principles/core/efficient-architecture-heuristics.md), [CI 1.4](../design_principles/profiles/code-intelligence/principles.md) and [binding](../design_principles/binding/library-context.md). Implementation review against approved obligations; no new target architecture certification. |
| Independence | Three fresh implementation reviewers examined compiler/projections, native realization/lifecycle, and serving/retirement respectively. The native reviewer independently reconciled decisive cross-component findings. Root coordinated probes and authored the report; implementation completion messages did not determine the verdict. |
| Method | Current definitions, executing source and adjacent consumers; finite dispatch/inventory coverage; dependency/import/configuration/recipe inspection; narrowly targeted Git comparison to `6f1a7e98` for retirement and prior consumer behavior; focused diagnostic probes. |
| Workload | One operator/local persistent server, pinned inputs with many attributed observations, skewed graph degrees, ordinary small evidence/API answers and larger analytical views. No convenient fixture-size restriction substitutes for this premise. |

The compiler's user-accepted completion and stopped testing remain historical facts. They do not
exempt its source from this audit or establish that every admission guarantee is enforced.
Historical Catalog/native/MCP successes remain valid for their exercised cases and revisions.

## Completeness and pathway coverage

**Migration:** no reachable PostgreSQL runtime, old generation reader/command, Python product
retrieval engine, compatibility reader or dual canonical store was found in the inspected runtime
closure. Development BM25 evidence oracles, semantic provider-run records and retained historical
evidence are not runtime migrations merely because their names match a search. Current documentation
retirement is incomplete (F12).

**Meaning:** typed identities, attributed payloads, nominal reference/participant roles, originals,
consumed vectors and declared projections are present. Physical preservation does not compensate
for omitted source-owned semantic admission checks (F01), and a pinned handle does not compensate
for an incomplete executable dependency identity (F06).

**Execution:** native roots/search and scoped coarse hydration replace whole-store serving startup.
The spillable compiler workspace does not make every consuming kernel spillable. Whole-resident
normalization, repeated upstream replay and expansion before native batching remain (F02/F03/F11).
These are structural findings; no latency, throughput or capacity claim is made.

| Obligation / supported pathway | Current-source coverage and disposition |
|---|---|
| G0 architectural adoption | ADR-0128 and current native architecture/runbook/routes exist. Current assurance routes still contradict the pivot: F12. |
| M1 graph / semantic identities | Finite typed entities/assertions, nominal targets, attributed and ordered roles, content/collision checks, separate semantic/producer/physical identities inspected. Semantic support enforcement incomplete: F01. |
| C1 capture / providers / workspace | Six production provider bindings; immutable completed IPC inputs, deterministic ordering/conflict refusal, original capture, cancellation and atomic output completion inspected. Compiler has no database credentials/dependency. |
| Public compilation frontiers/profiles | Facts, Normalized, Analysis and Catalog; Catalog and Behavioral dispatch inspected. Internal Conformance is rejected by artifact format validation. Ordinary compile, artifact-only and publish-existing-artifact routes are present. |
| C2-N finite normalization | Entities, relations, callables, receivers, callable aspects, events, bindings, projections and coverage all have current bindings. Their resident preparation route is F02. |
| C2-U finite dispatch | All 21 declarations have executing routes: Configuration, Native, EmbeddingConfiguration, Text, Embedding, CatalogCore, CatalogEvidence, Local, Base, Completion, SourceCalls, Enriched, Models, Summary, Structural, Analytic, Selection, Synthesis, Retrieval, AnalysisFrontier and CatalogFrontier. Dependencies consume completed inputs; broad replay remains in F03. |
| Optional analysis/embedding | Five analytic methods and three optional community layers inspected; disabled methods retain NotRequested. Selected partial/convergence/vector failures retain outcomes. Embedding absence avoids opening effects; request/spec/token/winner bytes remain checked. Positive optional branches were not newly exercised. |
| Upper completion obligations | Final frontier derivation independently checks captured frames, required invocations, outcomes and coverage. Invocation-derived manifest outcomes alone are not an additional omission finding. |
| A1 named projections | CallableInvocation, DefinitionContainment, ImportReference and PublicExposure declarations, universe/direction/role/multiplicity policy, isolates, parallel/self arcs, external/unresolved gaps and semantic lineage inspected across workspace and native export. |
| A2 consumers | Summary SCC scheduling, Structural, PageRank, communities, FCA/RCA and exact-neighbor adapters use declared inputs/topology. Parameters, qualifications and semantic result IDs are retained. Individual algorithms were not exhaustively re-proved. |
| Compiler → publisher transport | Typed payload/participant reconstruction, original byte length/digests, raw catalog/selection/packet inputs, derivations and consumed vector specifications/bytes inspected. Transport verification is not semantic admission. |
| P1 native layout / codecs | Generated closed compact bodies, string record IDs, native relation insertion, endpoint-before-role ordering, sparse scope indexes and exact bytes inspected. SDK resolves to 3.3.0; production uses remote gRPC/HTTP without embedded primary storage. |
| P1 cache / originals | Complete spec/input key, retained definition/token/vector winner checks, original range/chunk verification and binary preservation inspected. Cache writes remain scalar (F11); actual same-key concurrent conflict behavior remains unresolved. |
| P2 load / seal / visibility | Checked statements, canonical/role reconciliation, synchronous index definitions, supervised sequential loader, invalidation before returning handle and database VIEWER inspected. Cold audit omits derived row integrity (F04). |
| P2 executable integrity | Installed effective definition inventory and altered-definition refusal present. Rust helper dependency omission is F06. Users/access/live subscriptions/STRICT inventory exclusions are documented trust limits, not newly alleged defects. |
| P2 lifecycle | Dedicated pinned sessions, publication distinct from selection, canonical backup, fresh restore/rebuild and explicit reader-stop retirement present. Selection atomicity and HTTP backup finality fail their contracts: F05/F07. |
| S1/S2 selection/search | Discovery/Strict groups, requirement witnesses/joint algebra, indexed roots, eligible context/member pairs before channel nomination, exact-symbol priority, family/member collapse, canonical ties and RRF inspected. Fixed native candidate caps are declared bounded discovery, not exact global top-k. |
| S3 tools/resources | Every tool and its applicable sections traced below. Packet/original/resource handles pin semantic content, realization and exact database. Scoped vocabulary, nested continuation and public failure defects: F08–F10. |
| S3 lifecycle/wire | Overall Python clock includes decoding/embedding/slot wait; Rust receives remaining deadline. Shielded workers retain admission until drain; PyO3 drains before invalidation. Final stdio byte admission includes envelope/newline/request IDs. Synchronous-kernel interruption and native-store cancellation were not newly exercised. |
| I1 entrypoints/build/readiness | CLI native configuration before acquisition; native tools/query/lifecycle; wheel input/membership keys; owned native fixture recipes; manifests/lockfiles/imports/configuration inspected. Runtime retirement observed; current documentation remains F12. |
| Q0 / Q1 | Prior focused fixture receipts inspected at their actual revisions. This audit adds diagnostic evidence, not a complete final-tree runtime campaign. Real-library/live-Qwen reconstruction, operator adoption and performance remain not_run. |

The ten-tool coverage includes: `search_operations` native nomination/fusion; `find_operations`
classification and witnesses; `get_operation` ambiguity/core and all optional sections;
`browse_library` members/modules/classes/vocabulary scopes; `get_evidence` original resolution,
literal/body/flow/characterization/derivation paths; `search_evidence` families/original anchors;
`compare_operations` independent selectors and joint algebra; `search_capabilities` authored brief
origins; `get_capability` attributed packet and MCP resource; and `inspect_value_paths` exact formal
bindings, native inventory, conditional preparation and unavailable/partial states.
Static coverage is not a newly passed functional test of every branch.

## Findings

Severity reflects the supported consequence: **high** for admitted semantic validity or false
recovery success; **medium** for executable compatibility, constrained consumer/lifecycle defects
and structural execution gaps; **low** for misleading current documentation routes. Findings remain source-supported unless explicitly reproduced.

<a id="F01"></a>
### F01 — Admission omits source-owned semantic support invariants

**High · M1/C1/C2 admission · FP-04/FP-05, CI-01/CI-11.**
[admit](../../../crates/cpg-core/src/artifact.rs):1095–1127 checks compilation completion and facts
availability, then graph shape/reference closure and derivation acyclicity. Production compilation
does not invoke completed-input semantic invariant checks. The `workspace.validate()` call in
[facts](../../../crates/cpg-core/src/facts.rs):121 belongs to `inspect_with_budget`, not production
`compile_facts`.

An observation can have qualifications, subjects and closed references while its declared support
relation is complete-empty. Support points toward the observation, so graph reference closure
cannot require that support exists. The model's [SupportCheck](../../../crates/lctx-model/src/domain/assertion.rs):990–1012,1309–1313 explicitly rejects unsupported assertions and wrong-context/provider/family/ownership lineage. Row shape and target existence do not establish those conditions.
Publisher and restore reconciliation preserve the invalid semantic content rather than supplying
these omitted checks. No naturally occurring erroneous producer output was demonstrated.

**Correction:** enforce the necessary source-owned support/qualification/ownership obligations at
the actual completed admission boundary, or consume their acknowledged immutable result. Do not
enable every retained full replay invariant indiscriminately. **Closure:** independently authored
unsupported observation and wrong-context support refuse through production admission; valid
attributed observations continue to admit.

<a id="F02"></a>
### F02 — Spillable storage feeds whole-resident normalization

**Medium · C2-N / affected C2-U preparation · FP-07/A4.**
[normalize::load](../../../crates/cpg-core/src/normalize/mod.rs):16–26 collects every `SELECT *`
batch into [Rows](../../../crates/lctx-model/src/domain/normalized/rows.rs):8. Entities retain
their full fact input; relations simultaneously retain fact, entity and relation inputs (:65–87);
bindings follow the same route (:180–192). The pure kernels produce additional resident inventories.

Transfer batches, charging and disk-backed IPC do not make these retained inventories spillable.
An input exceeding their combined resident envelope has no promised bulk/spill normalization route;
safe refusal alone does not realize the plan's execution fit. No measured ordinary-input failure
or capacity threshold is claimed.

**Correction:** use source-partitioned or bulk spill-capable preparation for the large set/join work,
retaining resident state only where the specific kernel needs it. **Closure:** inspect the actual
consumer route and compare canonical output over independently authored synthetic inputs under a
smaller memory envelope; a renamed collector or larger limit is insufficient.

<a id="F03"></a>
### F03 — Upper consumers retain broad upstream producer replay

**Medium · C2-U preparation · FP-03/FP-07, A3/A4.**
[SourceCalls](../../../crates/lctx-model/src/domain/execution/source_call_records.rs):318,
[Models](../../../crates/lctx-model/src/domain/execution/model_production.rs):538 and
[Summary](../../../crates/lctx-model/src/domain/execution/summary_production.rs):1618 call
[binding verification](../../../crates/lctx-model/src/domain/normalized/binding_normalization.rs):904.
It reruns binding normalization; `verify_upstream` (:1589–1634) copies and recomputes entity,
relation, callable and event normalization. Ordinary behavioral compilation repeats completed
producer work and adds resident copies at several semantic consumers.

**Correction:** establish necessary binding/composition assurances once for immutable completed
inputs and share the prepared result. Retain independent canonical replay where a targeted control
needs it. **Closure:** inspect all three consumers and their preparation lifetime; broad replay is
absent while required semantic assurances remain. F01 and F03 require compatible corrections.

<a id="F04"></a>
### F04 — Cold audit does not reconcile the whole served representation

**Medium · P2 audit / realization · FP-05, CI-13.**
[inspection::audit](../../../crates/lctx-publisher/src/inspection.rs):181 checks canonical
reconciliation and definition identity. [reconciliation](../../../crates/lctx-surrealdb/src/reconciliation.rs):9–19,176 inventories canonical records/roles/external endpoints/originals, but not search documents,
vectors or lexical/vector occurrences; its `Node` omits query-visible scope fields.

Altered search text/eligibility/witnesses/numeric vectors or stale scope values can leave canonical
bytes and definitions intact and escape explicit cold audit. Immediate publication readback is
useful but cannot detect later drift. This concerns the declared anomaly/recovery audit, not an
adversarial-root security guarantee.

**Correction:** read-only verification of canonical-to-query representation mappings, including
scope keys and derived search witnesses/vector lowering. **Closure:** altered/deleted/extra derived
records and altered scope/eligible/context/member/witness/vector fields cause audit refusal without
replaying producers or making ordinary requests rehash the database. See diagnostic evidence below.

<a id="F05"></a>
### F05 — Selection publishes two authorities in separate renames

**Medium · P2 selection / S3 launch configuration · FP-04, G5.**
[RuntimeConfig::select](../../../crates/lctx-surrealdb/src/config.rs):98–99 publishes serving JSON
before selected JSON. A crash/error between renames can make new MCP launches pin B while CLI
selection remains A; a failed selection can already expose B's serving configuration.
[retirement](../../../crates/lctx-publisher/src/backup.rs):324–327 checks only selected JSON.
Existing readers retain their original pins; the defect is disagreement among new-process/lifecycle
consumers, not silent retargeting of an active session. Explicit stopped-reader attestation remains
a separate safeguard.

**Correction:** one atomic authoritative selection consumed coherently by CLI, MCP launch and
retirement; credential presentation can remain derived. **Closure:** failure at selection publication
leaves all consumers on one state, and a failed operation does not publish a new launch pin. See
the deterministic rename-failure diagnostic below.

<a id="F06"></a>
### F06 — Executable identity omits an answer-affecting helper dependency

**Medium · P2/S3 executable identity · FP-02, CI-13.**
[operation_definition](../../../crates/lctx-serving/src/lib.rs):41–97 hashes a manual source list
that omits [serving/packets.rs](../../../crates/lctx-model/src/domain/serving/packets.rs).
[core::packet](../../../crates/lctx-serving/src/core.rs):403 uses its `DefaultValue::from_canonical`
(:55). A schema-preserving change to that helper's Literal/Unknown mapping changes packet behavior
without changing the source guard, wire schema identity or mapping identity. An older sealed
realization can therefore be served by changed executable semantics without refusal.

The complete model implementation digest exists but is not consumed by this guard. Native request
evaluation is covered indirectly by its own definition; it is not a second claimed omission.
**Correction:** dependency-complete identity for answer-affecting implementations, using the existing
complete digest or a complete declared closure. **Closure:** a schema-preserving helper change
changes identity and refuses the earlier realization; current unchanged helpers remain compatible.
No production source was changed to perform that experiment in this audit.

<a id="F07"></a>
### F07 — HTTP backup EOF does not establish terminal engine success

**High · P2 backup / library transport · FP-02/FP-05, G5/G7.**
[backup](../../../crates/lctx-publisher/src/backup.rs):83–106 trusts HTTP SDK export completion,
syncs and publishes the destination. In resolved SurrealDB 3.3.0, the
[SDK HTTP exporter](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/src/engine/remote/http/mod.rs#L468-L493)
checks initial status and copies to EOF. The
[server export handler](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/server/src/ntw/export.rs#L100-L112)
logs a late export task error/panic and then drops the output channel normally. A partial dump can
therefore end with successful HTTP EOF and be reported as completed backup. Restore reconciliation
later rejects missing content, but does not make the earlier backup success truthful.

**Correction:** a transport carrying checked terminal engine success, or independent completed-dump
validation before final publication. The selected SDK's
[gRPC export](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/src/engine/remote/grpc.rs#L1566-L1643)
checks a terminal trailer/length; inspect its composed backup route before adoption.
**Closure:** a late export failure yields failure and no completed destination; normal canonical-only
backup/fresh restore still works. Actual engine failure injection was not_run; diagnosis is source-supported.

<a id="F08"></a>
### F08 — Scoped vocabulary is rebuilt from all eligible library members

**Medium · S3 browsing · FP-04, CI-05.**
[browse vocabulary](../../../crates/lctx-serving/src/operations.rs):651–685 loops over
`d.selected.eligible()` independently of the preceding module/class scope filter. A vocabulary
request for a module/class can return other scopes' values and member counts while echoing the
narrow requested scope. The [product contract](../../design/sections/api-and-evidence-product.md)
promises scoped vocabulary/counts; the prior consumer used scoped members.

**Correction:** one scoped eligible-member set governs vocabulary and counts as well as other views.
**Closure:** two modules/classes with disjoint predicates; actual native scoped vocabulary excludes
foreign values/counts and whole-library browsing retains them.

<a id="F09"></a>
### F09 — A nested diagnostic cursor loses the parent scenario page

**Medium · S3 continuation composition · FP-03/FP-05, CI-08.**
[operation packets](../../../crates/lctx-serving/src/operation_packet.rs):295–319 page each
`scenario_diagnostics_<association>` section before separately paging parent scenarios.
[pagination](../../../crates/lctx-serving/src/pagination.rs):53–65 applies a cursor only to its
matching section; the parent restarts for a nested cursor.

With page size one, at least two scenarios, and multiple diagnostics on the second scenario,
following the parent cursor then the emitted nested cursor computes the remaining diagnostics but
drops their parent from the restarted scenario page. The continuation cannot retrieve its promised
remaining content. **Correction:** retain/address the selected parent when following its child
continuation, or expose an independently addressed child section. **Closure:** the exact native
two-scenario trigger returns the second scenario's remaining diagnostics, preserving handle/request bindings.

<a id="F10"></a>
### F10 — Native errors bypass the declared safe structured failure contract

**Medium · S3 Rust/PyO3/MCP failure boundary · FP-02/FP-05, G7.**
[service::failure](../../../crates/lctx-serving/src/service.rs):210 maps non-resource model errors
to generic invalid strings; [NativeSession](../../../python/lctx_semantics/src/session.rs):112
converts them to `PyValueError` text; [MCP adapters](../../../python/lctx_mcp/src/lctx_mcp/wire.py):272,294
forward text as tool/resource errors. The model's fixed-message `PublicFailure` and exported
`wire_failure` are not consumed by the product adapter, although wire identity still advertises
`lctx_failure` and current serving documentation promises safe structured failures.

Unknown library, incompatible cursor/realization, unavailable reads and corrupt evidence lose their
recognized cause/metadata; internal diagnostics can become public text. Credential leakage was not
demonstrated. **Correction:** preserve typed causes through Rust/PyO3 and emit the owned tool/resource
failure envelopes, without parsing diagnostic strings. **Closure:** actual MCP recognized failures
and an unexpected diagnostic have fixed safe messages and appropriate `_meta.lctx_failure` /
resource error data, within complete wire-byte admission.

<a id="F11"></a>
### F11 — Native batch interfaces retain unbounded expansion and scalar crossings

**Medium · P1 cache / P2 search materialization · FP-07/A4.**
[search materialization](../../../crates/lctx-publisher/src/search.rs):62–214 fetches whole corpus
fragment/anchor/use inventories for a page of units, accumulates expanded unit × fragment × member ×
anchor occurrences and vector objects, then chunks inserts (:328). Its expected-ID map grows over
the whole load. A 128-unit page does not bound corpus size, fanout, response bytes or expanded live
objects. Separately, [cache admission](../../../crates/lctx-surrealdb/src/cache.rs):120–148 accepts
a slice but awaits one gRPC INSERT per vector before its batch winner lookup. Both leave material
work outside the promised bulk/bounded boundary; they have distinct closure obligations.

**Correction:** stream/page lowering into byte-bounded batches and a suitable streamed reconciliation;
batch cache inserts through bound values with exact winner reconciliation. Preserve complete witness
semantics and shared-vector identity. **Closure:** inspect bounded construction/reconciliation for
skewed corpora and concurrent same-key batch admission with exact winners. No speed estimate is
claimed. A universal accounting service or per-query work counter is unnecessary.

<a id="F12"></a>
### F12 — Current assurance routes still prescribe the removed runtime

**Low · G0/I1 documentation retirement · FP-02/FP-06, G7.**
[docs task routing](../../README.md):17 calls the assurance plans current/implemented.
[validation execution](../../plans/validation-execution-pivot-plan_2026-10-04.md):16–31 assigns
authority to deleted `lctx-postgres` validation sessions and retains PostgreSQL as its target.
[analytical enrichment](../../plans/code-facts-analytical-enrichment-plan_2026-10-04.md):25,221
also gives current PostgreSQL mechanisms/acceptance instructions. These are reachable present-tense
instructions, distinct from correctly dated historical receipts.

**Correction:** carry enduring obligations into their current native owners, retire obsolete current
prescriptions/plans once their last consumer moves, and preserve historical evidence through Git.
**Closure:** current task routes have one consistent native owner and runnable native acceptance
instructions. A global ban on PostgreSQL words would wrongly remove historical/evaluation context.

## Integrated assessment, evidence and remaining uncertainty

The principal reviewer inspected decisive sources and reconciled the semantic-admission/physical-
preservation boundary, the executable-identity/cold-audit distinction, and continuation/selection
composition. Broad replay is not the appropriate substitute for F01's necessary semantic checks.
F05 affects new-process selection and recovery consumers; active readers retain their pins.
F11's two manifestations share a bulk-boundary problem but require separate cache and publisher fixes.

| Judgment | Audit conclusion |
|---|---|
| Completeness | Incomplete: F01–F12 describe unmet approved guarantees or material retained structures. Presence of all dispatch branches does not settle their correctness. |
| Migration | Runtime retirement observed in inspected closure; G0/I1 current documentation retirement incomplete. No restoration of historical artifacts or compatibility path is recommended. |
| A1 / FP-01, FP-06 | Coherent compiler/model/native/transport owners and coarse inputs are present. Manual executable dependency knowledge and stale routes weaken local reasoning (F06/F12). |
| A2 / FP-04, FP-05 | Violated for audited obligations: semantic support checks, selection authority and scoped browsing/failure contracts do not fully govern actual behavior. |
| A3 / FP-02, FP-03 | Composition incomplete at shared predecessor assurance, nested continuation and error transport (F03/F09/F10). Complete public handle bindings are otherwise present. |
| A4 / FP-07 | Violated structurally for normalization/replay/native bulk boundaries (F02/F03/F11). Native querying remains the chosen execution basis; no measured performance conclusion. |
| G1/G2/G3; CI-G1 | Semantic validity/admission and authoritative selection gaps prevent a whole-scope pass. Typed graph/codec/reference checks are implemented, but are insufficient for F01. |
| G5/G6/G7 | Recovery/identity/audit and scoped response/continuation/failure gaps prevent a whole-scope pass. Existing successful fixture cases do not refute these triggers. |
| G4/G8; CI-G2/CI-G3 | No competing Python semantic engine or evaluation-input leak found in inspected routes. Composed evidence closure remains limited by admission/identity gaps; no whole-system gate certification is made. Existing bulk/library capabilities inform F11's correction. |

Diagnostic results and exact invocation are recorded in the [evidence README](../evidence/2026-10-06_graph-native-pivot-audit/README.md).
The root release build checks current code; diagnostics test specific adverse conditions, not broad
product acceptance. On 2026-10-06:

| Command / adverse condition | Outcome and limit |
|---|---|
| `python3 scripts/build_environment.py -- cargo build --release --locked -p lctx-publisher -p lctx-serving --message-format=json` | **passed** on audited production sources; release compilation, not compiler-suite execution. |
| `UV_NO_SYNC=1 uv run --no-sync python docs/design_review/evidence/2026-10-06_graph-native-pivot-audit/run_probes.py` | **passed** as diagnostic reproduction: final selection rename failed while the new serving pin was readable; cold audit accepted after seven actual Catalog `search_api_options` texts were changed. Both intended product guarantees **failed** in these cases. |
| Setup attempts for that runner | **failed** before execution: the pinned nightly's separate Rust metadata/code artifacts require both paths. Corrected diagnostic runner supplies both; no product source or acceptance expectation changed. |

The native diagnostic used an owned disposable persistent RocksDB 3.3 server, actual compiler/
admission/publication, and the installed first-party synthesis-sources fixture without live
embedding. It did not invoke the stopped compiler test suite. Search text was the executed drift
case; other drift fields follow the inspected missing inventories. Selection failure was a local
filesystem collision; a process crash and retirement DROP were not executed. Owned server,
credentials, binaries and scratch were removed. Raw run log: `/tmp/graph-native-pivot-audit-probes.log`.

Publication checks on 2026-10-06: `UV_NO_SYNC=1 just docs-check` **passed**, 321 canonical
pages and zero link errors. `UV_NO_SYNC=1 uv run --no-sync ruff check --config 'force-exclude = false'
docs/design_review/evidence/2026-10-06_graph-native-pivot-audit/run_probes.py --output-format concise`
**passed** after import/style repairs. Evidence is normally excluded by Ruff; the initial no-files
selection was not counted as a pass. `UV_NO_SYNC=1 just turn-end` **passed**, with no generated or production-source changes.
Those checks establish publication/probe hygiene only.

Additional coverage limits remain explicit:

- Same-key cache concurrency: the existing native control performs sequential winner reuse, not
  simultaneous admissions. Transaction/transport errors return before the winner read. Actual
  concurrent conflict behavior remains unresolved; no race failure is asserted without execution.
- Historical ten-tool runtime coverage is Catalog, with optional NotRequested/Unavailable states.
  Positive behavioral/optional-method serving, synchronous-kernel interruption, actual native-store
  cancellation and unknown class/multiple-context browsing require targeted follow-up when relevant.
- The earlier direct-original context concern was not confirmed: current providers share the same
  analysis context, and genuine ambiguity is refused. Broader supported multi-context originals
  would require a separate explicit interface investigation.
- Synchronous native index creation is not missing readiness: current definitions omit concurrent/
  deferred creation, and pinned source waits for creation. Pre-collapse candidate caps are not a
  global-top-k defect under the stated bounded search contract. Administrative/STRICT exclusions
  and physical subchunk granularity are not automatically violations.
- Final-source wheel build/import is not a final-source runtime journey. Raw source-byte identity
  legitimately changes after formatting; earlier fixture receipts remain attributed to their own
  baselines. This is an evidence limit, not a recommendation to normalize source or restore an old realization.
- Stopped compiler testing, broad `just qualify`, legacy snapshot/parity work, real-library/live-Qwen
  journeys, operator activation and performance measurement: **not_run**, excluded by the agreed scope.

The SurrealDB reviewer read the capability skill's selected schema, graph, bulk/value/query, error,
concurrency, search, connection/security, feature and frontend routes against resolved 3.3.0,
then used Context7 and official GitHub sources for concrete readiness/export questions. Optional
custom APIs, modules, buckets, external frontends and distributed deployment retain their actual
consumer-triggered status. No query-adoption gate, cost proof, dependency upgrade or skill refresh
was introduced.

## Correction order and rule impacts

1. Close semantic admission and executable dependency integrity (F01/F06), with focused adverse
   semantic inputs and an identity-change control. Establish necessary invariants without adding replay.
2. Repair backup finality, coherent selection and explicit cold verification (F07/F05/F04), using
   owned disposable state and deterministic failure/drift cases.
3. Repair scoped vocabulary, parent/child continuations and typed safe failures (F08–F10), through
   targeted native/MCP journeys with independently specified expected results.
4. Complete normalization/preparation/bulk execution migration (F02/F03/F11) and current documentation
   retirement (F12). Validate canonical results and meaningful working-set/failure cases; measure only
   if a quantitative claim or unresolved choice needs it.

These correction directions fit the approved hard pivot and preserve the exact existing embedding
specification. No new store-selection decision, universal transpiler, rollback reader, historical
archive or accounting machinery is needed. Any future correction that changes an architectural
decision should use the existing ADR/design route. This audit authorizes no production remediation
or Q1 adoption; it records the work that prevents a completeness claim.
