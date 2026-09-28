# PR3 implementation review: original scenarios and deployment evidence

## 1. Scope, outcome and coverage

**Decision: Accept the scoped PR3 architecture and bounded qualification, 2026-09-28.** The eight
findings below have corrected-source, focused and assembled evidence. The complete gate, both
live profiles, fresh deployment observations, original-byte MCP expansion, current/mixed recovery
and operator cutover passed. This qualifies the supported PR3 records and boundaries; it does not
qualify PR4–PR6, general behavioral soundness or comparative Context7 differentiation.

This is a **design/target** review of assembled PR3 under core 3.0, code-intelligence profile 1.1
and the [repository binding](../design_principles/binding/library-context.md). The subject is
`5aebab46e6ec650b5a0351b8ca70b8dc2180af44` plus the concurrent working implementation, including
the corrections, release-subject normalization and bounded diagnostic/determinism follow-up
rechecked on 2026-09-28. The final qualification update inspected the
[evidence owner and receipts](../evidence/2026-09-28_pr3/README.md) after the implementation owner
completed the gate, live runs and cutover. Source links identify owners rather than unstable
working-tree line numbers. The reviewer changed only this document and ran no tests, builds,
formatters, database operations or integrated gates; outcomes below are from inspected receipts.

Reviewed: verified metadata and original document/config acquisition, scenario derivation,
provider-attributed associations, original-byte closure, explicit deployment receipts,
bundle15/projection5/migration011, Binary codecs, generation-pinned evidence reads, shared Rust
wire schemas and the new MCP route. The expected changes are additional selected documents,
large contexts, provider uncertainty, another library release, another explicit deployment policy
and a new evidence rendering. The [product owner §§14.5–14.6](../../design/sections/api-and-evidence-product.md#section-14-5),
[ADR-0076](../../adr/0076-catalog-contextual-evidence.md) and
[forward plan](../../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
define the target and execution boundary.

General evidence search/browse, the PR4 classifier, PR5 journeys, arbitrary execution policies,
minimal-install inference, arbitrary host attestation and comparative Context7 qualification are
excluded. Existing behavioral capabilities remain; their general soundness is not assessed here.
No sealed evaluation material was inspected.

## 2. Responsibilities, dependencies and semantic ownership

**Implemented; source-inspected 2026-09-28.**

| Owner | Responsibility, contract and reason for change |
|---|---|
| [Acquisition](../../../crates/cpg-extract/src/library.rs), [metadata interpretation](../../../crates/cpg-extract/src/metadata.rs) and [capture](../../../crates/cpg-extract/src/lib.rs) | Verify selected inputs, retain bytes and interpret supported metadata syntax. A new metadata dialect belongs here; effective deployment semantics are not inferred from parsing |
| [Receipt admission](../../../crates/cpg-extract/src/observations.rs) and [explicit runner](../../../scripts/deployment_check.py) | Separate execution from compilation. The runner owns disposable processes; admission checks fixed policy, source, runner and environment inputs |
| [Schema evidence](../../../crates/cpg-schema/src/evidence.rs) | Canonical identities, typed details/statuses, original-byte and cross-record invariants; shared publication/projection validation |
| [Pure evidence pass](../../../crates/cpg-core/src/evidence.rs), composed by [catalog](../../../crates/cpg-core/src/catalog.rs) | Load declared facts, prepare immutable indexes, derive scenarios/deployment associations, then materialize through the existing attempt owner |
| [Projection contract](../../../crates/cpg-schema/src/serving_projection.rs), [bundle](../../../crates/cpg-core/src/bundle.rs), [PG import](../../../crates/lctx-postgres/src/import.rs) | Rebuildable physical representations and ready-generation admission; no new canonical authority |
| [PG evidence](../../../crates/lctx-postgres/src/evidence.rs), [wire](../../../crates/cpg-schema/src/wire/evidence.rs), [native storage](../../../python/lctx_storage/src/lib.rs), [MCP](../../../python/lctx_mcp/src/lctx_mcp/server.py) | Generation-qualified reads, admission and pagination; shared Rust meanings reach a thin transport adapter |

The effectful path is acquisition → explicit facts → pure derivation → canonical publication →
rebuildable serving. The separate task runner supplies explicitly admitted observation inputs;
neither derivation nor querying launches it. A module is a sufficient ownership boundary here.

| Fact family | Provider/fidelity, identity, coverage and consumer |
|---|---|
| Original artifacts and spans | Acquired bytes or existing source facts; artifact identity binds release/path/content and span identity binds artifact/range. Digest integrity is distinct from exact or mapped release alignment |
| Scenario context | Derived enclosing source, independent imports and requirement references; synthetic Python coordinates remain distinct from original Markdown fences/passages. Parse coverage and execution status are independent |
| API associations | Existing provider edge/fact IDs, modality, phase, target and site context survive. Candidate targets remain candidates; textual mentions remain textual evidence. Association identity preserves subject kind, site and provider-edge multiplicity |
| Deployment metadata/configuration | RECORD-verified metadata and selected original configuration bytes; package declarations have one release subject, while explicit source/config associations retain member subjects. Parsed declarations do not establish operation-specific requirements, installation sufficiency or effective CLI acceptance |
| Task observations | Explicit fixed-policy receipts with source/runner/lock/runtime/interpreter inputs, command, interaction and outcome. The tested configuration is not proof of minimal dependencies or per-call execution tracing |

Compiler110/extractor36/wire2 and bundle15/projection5 make the changed contracts explicit.
The behavior-model catalog remains7 and brief template21 remains unchanged. Migration011 is
additive; current readers retain strict format checks for older generations.

## 3. Contracts, constraints and test boundaries

`EvidenceInputs` and `PreparedEvidence` expose all inputs to the pure pass, including coverage,
boundaries, provider calls, arguments and analysis-context definitions. No database, embedding
client or source acquisition is needed to call `derive`. Catalog validation reconstructs expected
rows from fresh canonical inputs. That reconstruction checks drift; independent fixture answers
are still needed to challenge a shared semantic mistake.

The late diagnostic repair preserves that successful publication path. When strict typed catalog
construction refuses malformed inputs, the attempt owner runs the same relational rules over a
cached session to recover named invariant diagnostics; it still returns an error. The cache
normalizes only transient table-level Arrow metadata, preserving field metadata, arrays and row
counts. Canonical table metadata and stored bytes remain unchanged. Source-coordinate ordering in
the prepared evidence index makes requirement/import order independent of provider batch order.

The shared validator checks artifact bytes/digests/identity, span bounds and UTF-8 boundaries,
typed references, scenario/deployment identity, option-expression equality with original spans,
association support and original receipt equality. Scenario execution cannot be promoted from a
source heuristic. Missing coverage produces a non-passing parse state; an empty association set
does not establish that an API has no uses.

Association rows require exactly one member or release subject. Release subjects must be
deployment declarations with unknown intent, no site/provider-call support and a matching original
artifact release; `release_distribution` cannot be attached to a member subject. The canonical
validator enforces those meanings, the migration enforces nullable-subject XOR, and the wire
`AssociationSubject` exposes the distinction structurally. Generation-local member foreign keys
continue to validate non-null member subjects.

Wire references distinguish spans, scenarios and deployments with nominal identifiers.
`get_evidence` binds requests/cursors to one generation and target. Metadata too large for the
packet is explicitly omitted while the primary original span remains readable; this is a
transport limitation, not an extraction-completeness claim. Original bodies are read only after
metadata/byte admission. Final Rust serialization enforces the actual response budget.

## 4. Composition and execution

| Question/stage | Universe, method and fidelity | Effects, limits and evidence linkage |
|---|---|---|
| Which original context belongs to a use? | Selected docs/examples/tests and public-catalog targets; indexed joins and syntax ancestry. Source context is retained, not stitched into an invented recipe | Pure derivation. Original ranges, unresolved requirements, per-site intent and parse coverage survive |
| Which API does an observation concern? | Preserve provider edges and contextual target identity; catalog-source closure is separate from callable exposure | No singleton-candidate promotion or inherited-constructor relabeling; deterministic row identities and ordering |
| What deployment information is known? | Parse declared package/extras/entry points and selected JSON fields; associate package metadata once per compiled release, config references by member | Release declarations are returned alongside member evidence with a tagged subject; no package resolver, shell execution or operation-requirement/install-sufficiency inference |
| Did the selected task work? | Fixed FastMCP programmatic and CLI policies; actual MCP listing and `add(2,3)` interaction | Explicit disposable runner, owned process groups, time/output limits; received observations admitted as data |
| What can fit in an API packet? | Ordered references, counts and up to two admitted demonstrations | Default20/max50 references; 32 KiB default/256 KiB expanded final packets; refusal of an optional demonstration does not discard the API contract |
| How is original evidence expanded? | Typed exact ID, metadata admission, one original span chunk and deterministic continuation | SQL reads bounded byte substrings; UTF-8 prefixes or explicit base64 preserve bytes. Cursor scope prevents generation/target/representation reuse |

Canonical artifact pruning follows evidence roots, including original task sources. Scenarios and
deployment evidence do not need a fabricated behavioral assertion to survive. PostgreSQL RLS,
loading/ready lifecycle and immutable-generation triggers remain the existing publication model.
The new Binary path is present in schema grammar, receipts, bundle canonicalization, COPY staging
and PG decode; its real boundary is covered by the focused PG receipt in §10.

## 5. Change and failure scenarios

| Trigger | Owner and observed propagation | Verification/remaining limit |
|---|---|---|
| Add a selected Markdown document or configuration file | Acquisition membership/content changes release inputs; existing capture and pure derivation produce roots. Serving consumes the same declarations | Original CRLF/Unicode/fence context and failed interpretation must remain attributable; no transformed child coordinates are claimed |
| Add an API outside brief seeds | Public catalog membership and provider/document associations drive evidence independently of synthesis | Fixture includes a non-seed API; catalog-source closure does not widen callable identity |
| Add many public APIs without changing package metadata | The evidence owner retains release-scoped package declarations once; only genuinely member-specific associations grow | Fixture expects four metadata declarations regardless of member count; both live profiles contain 85,167 associations, including 88 release subjects. The 200,000-row relation cap is unchanged |
| Provider returns candidate or parallel targets | Existing evidence inputs carry the alternatives; association records keep edge identity, modality and phase | Candidate mutation and reordered-input controls challenge fidelity; no winner is inferred from set size |
| Equivalent provider batches arrive in a different order | Prepared evidence sorts module/child syntax by coordinates and stable node ID, and lexical bindings by coordinates/name | Extended permutation control passes in the complete gate; final both-profile scenario content matches |
| Strict typed decoding rejects malformed raw input | Attempt owner initializes empty analysis relations, then invokes shared relational diagnostics only on the failure path | Repaired invalid-input diagnostics and full publication validation pass in the complete gate; a diagnostic failure still cannot publish an invalid snapshot |
| A context contains negative tests, deferred code or a method override | One intent owner combines exact resolution with execution-region ancestry | Manager-header, nested function, generator eager/deferred/filter and unittest-override controls; unresolved unittest dispatch is withheld |
| A source expression or launch fence becomes very large | Canonical evidence remains available; serving marks omitted metadata and paginates the original primary span | Focused PG test covers both budgets and long typed details. Complete oversized metadata browsing remains outside this route |
| Add another explicit deployment task | Change the versioned runner/admission policy and its independent interaction control | Current policy is intentionally fixed, not a general execution framework. Runner bytes and exact policy bind the two implementations |
| Upgrade the analyzed release or restore an older generation | Release/content identities and projection manifests distinguish inputs; recovery retains older formats while current serving refuses unsupported readers | Current and mixed recovery passed; six legacy generations remain preserved with matching-runtime requirements. Selected-legacy and corrupt-artifact refusals passed |

These are bounded extension routes. They do not justify a new plugin system, production incremental
engine or universal recipe evaluator. PR4 can consume typed contextual details, but must preserve
metadata omission, unknowns and source-vs-runtime distinctions when adding eligibility semantics.

## 6. Correctness and fidelity gates

| Gate | Scoped verdict and evidence |
|---|---|
| G1 Authority | **pass, source scope:** schema-owned records and one pure evidence owner; Delta remains canonical and PG rebuildable |
| G2 Semantic fidelity | **pass for inspected supported shapes:** independent status dimensions, provider support, original/synthetic coordinates, release/member subjects and explicit omission. Corrections F01–F08 below |
| G3 Validity | **pass for supported scope:** shared validators and typed boundaries reject malformed references/bytes; diagnostic, focused and complete-gate controls passed |
| G4 Hidden behavior | **pass, source scope:** acquisition and the explicit task runner own effects; derive/query do not execute analyzed examples |
| G5 Consistency/recovery | **pass for supported scope:** scoped reads/limits, current and mixed recovery, six-generation preservation and populated operator restore passed; legacy serving still requires its matching runtime |
| G6 Transformation/reuse | **pass for supported scope:** independent evidence roots, Binary round trip, provider permutations, both live profile content and original-byte serving/recovery have qualified receipts |
| G7 Truthful capability claims | **pass at the labels in this review:** runtime success is limited to the two recorded policies; no minimal-install sufficiency, arbitrary-platform success or product differentiation is inferred |
| G8 Library leverage | **pass, bounded fit:** existing parser, Arrow/Delta, SQLx, Serde/Schemars and FastMCP capabilities provide generic mechanisms; custom code owns domain associations and admission |
| CI-G1 Fidelity | **pass for inspected supported claims:** expected-failure scope, candidate attribution and execution observations remain distinguishable |
| CI-G2 Evidence closure | **pass for supported scope:** shared closure enforcement, live span/scenario/task-deployment expansion, exact original-byte checks and populated recovery passed |
| CI-G3 Evaluation integrity | **pass within inspected changes:** no gold/heldout input route added; no comparative evaluation was performed or accepted |

Gate judgments are limited to the inspected supported shapes and named qualification runs.
Future semantic, ownership or admission changes require review of their affected boundaries.

## 7. Findings and dated correction evidence

The findings below retain the defects identified during this implementation review and their
inspected corrections. They are not a mutable completion register. Current disposition belongs
to the [forward plan §6.2](../../plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition).

<a id="F01"></a>
**F01 — Large metadata prevented access to original evidence.** The producer can retain a large
argument expression or deployment snippet, while the reader originally refused its entire detail
before exposing the primary span. Even expanded mode could not open a valid record. **FP-02/05,
DP-08/20, CI-08/11; G5/G6, CI-G2.** The PG owner now performs SQL metadata admission and returns
`metadata_omitted=true` plus the primary original span when needed. Canonical detail is unchanged
by that transport decision. The focused PG receipt covers oversized scenarios/deployments in both
packet modes. Complete oversized metadata paging is not claimed.

<a id="F02"></a>
**F02 — TestCase ancestry was mistaken for exact manager identity.** A first-parameter binding
and direct unittest base did not exclude overrides, MRO precedence or replacement. **FP-04/05,
DP-02/08, CI-02/06; G2, CI-G1.** The intent owner removed that ancestry shortcut. Instance forms
now require definite provider support for the actual unittest target; unresolved dispatch remains
ordinary test intent. The override control and the changed unresolved instance expectation are
present in the passing original-context fixture.

<a id="F03"></a>
**F03 — Deferred generator bodies inherited surrounding expected-failure intent.** Function and
lambda boundaries did not cover generator expressions. **FP-05, DP-08, CI-02/06; G2, CI-G1.**
The intent owner now recognizes `ExprGenerator`, allowing only its eager outermost iterable to
inherit the surrounding context. Deferred element/filter calls stop there. All three cases are
asserted independently in the original-context fixture; its focused receipt passed.

<a id="F04"></a>
**F04 — Runtime identity omitted the executed console wrapper.** The earlier CLI task launched
`bin/fastmcp`, but identity hashed site-packages and the interpreter, not that wrapper. **FP-05,
DP-09/21, CI-10; G4/G6.** The fixed policy now invokes `from fastmcp.cli import app; app()` through
the hashed interpreter, with exact command/tool/arguments checked during admission. Fresh
programmatic and CLI receipts bind the final runner SHA `9f5a9d0f…`, record `add(2,3)=5`, and were
admitted into both qualified live profiles.
It does not establish arbitrary host or external-service attestation.

<a id="F05"></a>
**F05 — Typed task observations could disagree with their original receipt artifact.** Source
hash and duplicated environment fields alone did not connect the embedded outcome to the original
JSON bytes. **FP-04/05, DP-01/03, CI-11; G1/G3, CI-G2.** The shared validator now deserializes the
original receipt span as `TaskReceipt` and requires equality before checking target source and
environment links. Original option-expression spans also require byte equality. The shared
validator controls passed, and the final task observations survive live publication and actual
MCP deployment expansion under both profiles.

<a id="F06"></a>
**F06 — Release metadata was expanded into a Cartesian set of member associations.** The first
live catalog run published its canonical snapshot but failed bundle admission with
`catalog_associations: relation budget`; the implementation owner reported 504,595 association
rows. Release-wide metadata had been copied onto every public member. Besides unnecessary
cardinality, that representation obscured whether a declaration concerned a package release or
an operation. **FP-04/05/06, DP-04/05/07/16/20, CI-03/08; A2/A3, G2/G5.**

**Correction Implemented, source-inspected 2026-09-28:** `CatalogAssociations` now has exclusive
member/release subjects, and its identity includes both tagged optional positions. The pure
producer emits one release association per first-party metadata observation. Member-specific
scenario, configuration and task associations retain their original meaning. Serving selects the
generation's release declarations alongside the requested member's associations, with a typed
wire subject; it does not claim those declarations are requirements of that member. The shared
validator rejects wrong release/artifact scope, member relabeling, non-declaration role, non-unknown
intent and provider-call support on a release subject. Migration011 enforces the subject XOR and
adds a release lookup index. No relation cap was raised.

The fixture asserts four package observations produce four release associations independently
of public-member count. The malformed release-as-invocation control and PG regression passed in
the complete gate. Both final live profiles contain 85,167 associations, including 88 release
subjects, and passed bundle/import/serving. This closes the observed cardinality defect at bounded
Tested strength without raising the cap. [ADR-0076](../../adr/0076-catalog-contextual-evidence.md)
and product §14.5 carry the revised subject contract. The forward plan §6.2 owns current
disposition of PR3/F06, together with the earlier correction findings.

<a id="F07"></a>
**F07 — Early typed catalog failure bypassed named relational diagnostics.** Strict decoding
could reject malformed input before publication's shared invariant rules ran. Moving diagnostic
evaluation earlier then exposed conflicting relation-level Arrow metadata when DataFusion planned
UNIONs containing initialized empty relations. **FP-03/06, DP-03/15/21; G3/G7.** This was a
diagnostic/validation-boundary failure, not evidence that invalid input was published.

**Correction Implemented, source-inspected 2026-09-28:** the
[attempt owner](../../../crates/cpg-core/src/attempt.rs) initializes analysis relations before
catalog construction and, on catalog error, calls the shared
[validation owner](../../../crates/cpg-core/src/validate.rs). `relational_costed` contains the
existing ordered, concurrency-bounded rule execution; the successful path still performs full
publication validation once. When diagnostic evaluation completes, the error path returns named
violations when available, otherwise the original catalog error; evaluation failures also remain
errors. It never converts failure into a successful result.

The transient cache removes table-level metadata from its schema and collected batches while
retaining fields, arrays, row count and registered relation name. The first repair used
`RecordBatch::with_schema`, whose inspected pinned contract requires a metadata superset; that cannot remove
existing metadata. The reviewer inspected the pinned implementation and identified this mismatch.
The current repair uses the pinned `schema_metadata_mut().clear()` operation instead. No canonical
schema or published metadata is rewritten. The repaired invalid-input diagnostic controls and
successful publication validation passed in the complete final gate. The earlier
`with_schema` build is not used as correction evidence.

<a id="F08"></a>
**F08 — Provider arrival order changed serialized scenario requirements.** Both live profiles
could compile and serve while emitting different orderings for the same scenario's requirements.
The prepared index retained provider order for module bindings and syntax nodes, which then
became serialized requirement/import order. **FP-04/05/06, DP-08/11, CI-13; G6.** A successful
individual profile therefore did not establish canonical cross-profile equality.

**Correction Implemented, source-inspected 2026-09-28:**
[`PreparedEvidence::new`](../../../crates/cpg-core/src/evidence.rs) sorts module/child syntax by
`(start_byte, end_byte, node_id)` and bindings by `(start_byte, end_byte, name)`. These are the
values that determine original context order; provider fact IDs and delivery order no longer
select it. Equal binding sort keys yield the same requirement representation, so this does not
introduce an arbitrary semantic winner. The
[permutation control](../../../crates/cpg-core/tests/catalog_evidence.rs) now also reverses
`CatalogFacts.lexical_bindings` and `CatalogFacts.nodes` before comparing the complete pure
evidence output. No extraction, serving or independent second sorting policy is added. The
extended control passed in the complete gate, and the final live profile comparison preserves
equal scenario content, original context and ordered requirements. The earlier parity failure is
superseded by that corrected-source evidence.

Earlier review feedback also resulted in explicit provider support/coordinates, coverage-based
parse status, independent catalog-source closure, optional-demo refusal handling and artifact
pruning. Those inspected boundaries are described in §§2–4. No additional open source finding
was identified in this pass.

## 8. Library fit and total complexity

**Implemented fit, inspected 2026-09-28; no new dependency recommendation.** The selected pinned
`pep508_rs` parser replaces ad hoc requirement splitting and uses `Requirement<url::Url>` to avoid
ambient URL expansion. `mailparse` provides header structure while repository policy preserves
raw UTF-8/unfolding semantics. `rust-ini` provides parsing with quote/escape rewriting disabled;
duplicate interpretation is explicit. These decisions keep package grammar out of the catalog
and avoid a resolver or dialect framework.

Ruff syntax and existing provider relations supply bounded intent/context information. The
custom code is a catalog projection over those contracts, not a replacement Python analyzer.
Arrow Binary, existing bundle/projection receipts and SQLx supply original-byte storage and reads.
Serde/Schemars continue to own shared wire structure; offline jsonschema remains conformance
tooling rather than a second runtime semantic validator. FastMCP supplies the actual client/server
interaction in the explicit runner. No production Salsa/Ascent engine is needed for these finite
joins, syntax walks and packet projections; their existing conditional consumers are unchanged.
Release-subject normalization is a relational modeling correction; another graph or incremental
engine would not fix the repeated semantic subject or its Cartesian cardinality.

## 9. Alternatives and tradeoffs

Expanding brief snippets would retain the seed limitation and couple original contexts to
behavioral synthesis. A separate authored recipe store or general executor would add authority,
lifecycle and input-trust problems without a selected consumer. Pure scenario/deployment functions
composed with the catalog are the simpler viable choice and retain original evidence independently.

Always hydrating full metadata is incompatible with fixed packet budgets. Permanently refusing
large records would make useful original content inaccessible. The selected explicit omission plus
primary-span fallback preserves honest bounded access; complete metadata pagination is a distinct
future consumer requirement, not silently implied by this API. Fixed deployment policies are less
general than an execution framework, but their commands and results can be inspected locally.

## 10. Verification and uncertainty

**Tested for the supported PR3 scope; final receipts inspected on 2026-09-28.** The reviewer ran
none of these commands. The [evidence owner](../evidence/2026-09-28_pr3/README.md) records exact
invocations and retains their raw outputs. Test source was also inspected in
[catalog evidence controls](../../../crates/cpg-core/tests/catalog_evidence.rs) and
[process-owner controls](../../../tests/scripts/test_deployment_check.py). Native modules use
the existing editable environment; no wheel build is part of this qualification.

| Command/receipt | Observed outcome and boundary |
|---|---|
| `just test-all`; [complete log](../evidence/2026-09-28_pr3/raw/test-all.log) | **passed:** 465 main Rust tests, 191 Python tests, 20 real-PG Rust tests and two PG Python tests. Formatting, Clippy, Ruff, Pyrefly, rules, ADR/agent checks, 90 fixture parses, dependency policy, gold-reference agreement and SQLx verification passed. Main Rust skips 21 explicit/ignored tests; the PG selection skips three unselected tests; main pytest reports two skips. The frozen-reference comparison is not admitted or run |
| `uv run --no-sync python scripts/deployment_check.py --source <pinned-source> --out build/pr3-qualified-tasks`; [programmatic](../evidence/2026-09-28_pr3/raw/task-programmatic.json) and [CLI](../evidence/2026-09-28_pr3/raw/task-cli.json) receipts | **passed:** both actual stdio policies list `add`/`echo` and return `5` for `add(2,3)`. Their runner SHA `9f5a9d0f78222c3f0fc2263fe2ca40c44a73ceca77041c9c3708354c6d18044e` matches the current script. Source, locked environment, interpreter/runtime and metadata identities agree; the CLI uses the hashed interpreter and fixed entry-point expression |
| `pilot_probe.py build/pr3-qualified-pilots build/pr3-qualified-tasks`; [receipts](../evidence/2026-09-28_pr3/raw/pilots.json) | **passed:** fresh live-vLLM FastMCP4.0.5 catalog and behavioral compilation, final task admission, canonical publication, PG import and actual stdio MCP. Both profiles reach the expected capability flags; behavioral enrichment remains optional with `techniques=none` |
| `compare_profiles.py build/pr3-qualified-pilots`; [result](../evidence/2026-09-28_pr3/raw/profile-parity.json) | **passed:** all seventeen catalog relation multisets match after explicit normalization of run-qualified citations/derived IDs and profile-specific brief status. The strengthened final comparison retains stable association `evidence_id` targets and artifact IDs; this checks evidence attachment as well as target content |
| `evidence_probe.py build/pr3-qualified-pilots`; [result](../evidence/2026-09-28_pr3/raw/live-evidence.json) | **passed:** actual MCP expansion of a span, a paginated scenario and both task deployments matches original bytes under both generations. Both contain 965 artifacts, 32,320 spans, 3,293 scenarios, 1,726 deployments and 85,167 associations, including 88 release subjects. Focused/full-gate controls additionally cover UTF-8 continuation, cursor scope, both packet budgets and oversized typed detail omission |
| Pilot populated backup/restore; [result](../evidence/2026-09-28_pr3/raw/live-restore.json) | **passed:** 86 tables, both current profiles served, all generations and the selected behavioral generation preserved; measured restore 20.23 s |
| `recovery_probe.py build/pr3-mixed-recovery`; [result](../evidence/2026-09-28_pr3/raw/mixed-recovery.json) | **passed:** schema010→011 with six legacy and two current fixture generations; both current profiles served. Selected-legacy and corrupt-artifact refusals passed. Legacy generations are preserved and require their matching runtime; current-runtime serving of them is not claimed. Measured restore 13.35 s |
| `operator_cutover.py build/pr3-qualified-pilots build/pr3-operator-cutover`; [result](../evidence/2026-09-28_pr3/raw/operator-cutover.json) | **passed:** schema011 migration/check, both final live imports, actual MCP/original evidence, exact behavioral selection and populated restore. All 86 tables/eight generations survive, including six legacy generations; both current profiles serve after restore. Measured restore 28.59 s |

The final gate and live receipts supersede the initial failures described in F06–F08 and retained
by the evidence owner. The last runner annotation changed its hash, so both actual task checks
and both live profiles were rerun against that final source. No failure-accepting snapshot
override is used in the complete gate. The parity probe was also tightened during this receipt
review: excluding stable association targets could hide changed attachments; the corrected
comparison passed without a production change.

Equality with reconstruction or profile agreement is not an independent semantic oracle.
Hand-authored intent/override/deferred-execution expectations and original-byte comparisons
remain the independent controls. The live probe expands selected records, not every possible
record or arbitrary API task. Compilation/import/smoke timings and restore durations are measured
local observations; they establish neither a comparative performance advantage nor a general
latency guarantee. Product comparison, complete behavioral soundness, minimal-install sufficiency
and positive ANN admission remain outside this qualification.

## 11. Authority changes and disposition

ADR-0076 and product §§14.5–14.6/14.9 remain the architectural owners. No further ADR or new
document type is needed for the inspected corrections. The forward plan §6.2 closes AP/F03,
CLF/F02 and PR3/F01–F08 at bounded Tested strength from the final receipts; that plan remains the
disposition owner. This review records the supporting architectural assessment.

The dated operator receipt selects schema011 behavioral generation `d874d369…` with exact profile
`ff645e4a…`; catalog generation `0b2fe120…` is ready. The evidence owner's
[recovery instructions](../evidence/2026-09-28_pr3/README.md#operator-state-and-recovery) identify
the current populated recovery set and preserved schema010 runtime/assets. Six legacy generations
remain retained, with strict reader compatibility. That owner records that the temporary vLLM
service stopped after the operator checks; live query embeddings require restarting it.

The accepted contract deliberately exposes metadata omission and withholds unproved instance
dispatch. PR4/PR5 must consume those states honestly. Revisit when a selected task needs complete
oversized metadata browsing, exact transformed-document child coordinates, another package/config
dialect, another execution policy or per-call runtime observations. Existing research and
comparative-evaluation obligations remain open under their current owners.

## 12. Architectural judgment and decision

| Judgment | Verdict and scenario evidence |
|---|---|
| A1 Localize change | **satisfied, source scope:** capture/parsing, pure association, effectful receipt execution, shared diagnostic caching and serving budgets have coherent owners. Source and policy changes have bounded dependency/test routes |
| A2 Encode meaning structurally | **satisfied after corrections:** identities, typed references and release/member subjects, independent statuses, provider support, receipt equality and explicit metadata omission preserve the distinctions agents need |
| A3 Extend through composition | **satisfied, source scope:** the existing catalog, publication, projection and transport contracts compose with a pure evidence pass; no speculative engine or competing store was introduced |

FP-01–FP-06 are satisfied for the inspected change scenarios. The related DP-01–05/07–09/11,
DP-13–24 and CI-01–04/06/08/10–13 are supported at the specific boundaries described above;
general recursive analyses and heuristic ranking changes are outside this review.

**Accept the scoped PR3 architecture and bounded qualification, 2026-09-28.** The complete gate,
fresh policy observations, both live profiles, original-byte serving and populated/mixed recovery
support the implemented boundaries. A1–A3 remain architectural judgments over the stated change
scenarios; passing tests do not certify the enclosing product. PR4–PR6 and comparative Context7
qualification remain open. A future semantic or ownership change requires review of the changed
boundary.
