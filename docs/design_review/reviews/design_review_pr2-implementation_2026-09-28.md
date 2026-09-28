# PR2 implementation review

## 1. Scope, outcome and coverage

The source-review checkpoint below precedes final qualification. [§13](#13-qualification-follow-up-from-the-implementation-owner)
records the implementation owner's later outcomes; the original review evidence keeps its scope.

**Decision: Accept scoped source conformance; assembled PR2 qualification remains unresolved,
2026-09-28.** No remaining blocking source defect was identified after the corrections below.
This does **not** accept the pending live publication, new-generation recovery, operator cutover
or complete code gate. The first real-library attempt failed during Delta constraint planning;
the source correction has been inspected, but its successful rerun is not evidence available to
this review.

This is a **design/conformance** review of the assembled PR2 boundaries under ADR-0073/0074,
core 3.0, code-intelligence profile 1.1 and the repository binding. Baseline is
`8334c86e462fe428eaf6ee6c55973e3ebeaf1a18` plus the concurrent dirty implementation. The reviewer
inspected source, test definitions and the named implementation-agent receipts; the reviewer ran
no build, tests, database actions or integrated gates and changed only this review document.

Reviewed: shared wire contracts and all six current MCP routes, the capability resource,
catalog loading/indexing/derivation, surface and field associations, canonical publication
validation, projection4/bundle14 and migration010, typed hydration and recovery compatibility.
PR3 scenario/deployment completeness, PR4 classification semantics, PR5 new tools, comparative
Context7 claims and general behavioral soundness are excluded. No sealed evaluation material
was inspected. The [forward plan §6.2](../../plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition)
owns current finding disposition; this review records dated evidence.

## 2. Responsibilities and authority

**Implemented, inspected 2026-09-28.**

| Owner | Responsibility and adjacent consumer |
|---|---|
| [Schema catalog](../../../crates/cpg-schema/src/catalog.rs) and [wire module](../../../crates/cpg-schema/src/wire/mod.rs) | Canonical rows, finite vocabulary, nominal boundary identities, required/null semantics, requests and nested response contracts; schemas derive from the same Rust owners |
| [Extraction](../../../crates/cpg-extract/src/walk.rs) | Preserve original field annotation/default syntax and spans without executing analyzed source |
| [Catalog](../../../crates/cpg-core/src/catalog.rs) and [surface](../../../crates/cpg-core/src/surface.rs) | Explicit immutable facts, prepared indexes and pure derivation; ordered surface observations and bounded associations |
| [Public relation owner](../../../crates/cpg-schema/src/public.rs) | Shared MRO/shadowing relation, with separate all-observation and preferred-path consumers |
| [PG hydration](../../../crates/lctx-postgres/src/hydration.rs) | Generation-scoped reads, bounded assembly and typed packet validation; no second surface interpreter |
| [Native transport](../../../python/lctx_semantics/src/lib.rs) and [MCP adapter](../../../python/lctx_mcp/src/lctx_mcp/wire.py) | Pure decoding/page semantics, generated tool contracts, bounded asynchronous admission and presentation |

| Fact family | Fidelity, identity and limit |
|---|---|
| Surface aspects | Ordered source observations with resolved identities and source facts; recognized metadata does not establish body admission |
| Configuration fields | Provider-declared records joined to original syntax; default/factory expressions remain unevaluated; unsupported normalization retains source and uncertainty |
| Field links | Declaration associations differ from bounded initialization proofs and exact receiver-field associations; reader links do not prove survival through intervening mutation |
| Catalog evidence | Original bytes, digest, span and source fact survive canonical materialization and packet hydration |

The new rows use compiler109/extractor35/template21 and projection4/bundle14. The authored
behavior-model catalog remains FORMAT7; adding catalog projection rows does not require relabeling
that unrelated model format. Migration010 is additive; earlier deployed migrations are unchanged.

## 3. Contracts and test boundaries

The Rust request boundary owns Unicode-scalar limits, finite variants, strict primitive decoding,
nominal hexadecimal IDs and unknown-field refusal. Required nullable output fields preserve the
difference between omitted malformed data and explicit null. Schemars uses distinct input/output
schemas. JSON Schema's mathematical integer acceptance of `1.0` is explicitly distinguished from
strict Serde token decoding in the fixture corpus.

`CatalogFacts` and `PreparedCatalog` allow local derivation without acquisition, embeddings or a
database. Publication reconstructs expected contracts from fresh canonical inputs; this catches
corruption but is not an independent semantic oracle. Hand-authored fixture expectations supply
the independent controls. Missing observations never become negative claims.

## 4. Composition and execution

| Stage | Model and output | Effects and limits |
|---|---|---|
| Catalog derivation | Indexed finite joins and traversal; retain alternatives, original source and declared unknowns | Explicit facts and public scope; no ambient SQL, files or embedding clients inside pure derivation |
| Surface normalization | Ordered observations; shared strict descriptor admission; supported property/registration metadata | Opaque composition remains unresolved; no decorator execution |
| Field association | Supported construction plus exact receiver, field and formal; separate declaration-only links | Refuse hooks, custom allocation/metaclasses, descriptor fields and unmodeled constructor effects |
| Serving | Generation-pinned projection reads followed by the Rust response contract | Query leases, hydration/packet limits, whole-request timeout and native worker admission |

The behavioral consumer explicitly marks cross-method field propagation unknown where intervening
mutation is unproved. It does not turn the catalog association into a temporal identity proof.
Native wire normalization now uses `py.detach`; canceled callers retain the admitted worker slot
until the computation actually finishes. Final packet limits apply after tagging and normalization.

## 5. Change scenarios

| Scenario | Observed ownership and verification route |
|---|---|
| Add a supported decorator | Add resolution/aspect semantics to the surface owner; catalog, behavioral admission and synthesis consume that owner; qualify alias, shadow and composition cases |
| Add a configuration association | Extend explicit source/field contracts and the bounded proof; PG/JSON shape follows declarations; preserve a negative descriptor/conversion control |
| Change a current tool field | Change its Rust contract and typed assembly; generate both schemas and exercise shared fixtures plus actual FastMCP listing/call |
| Retain an older generation | Preserve exact transport/artifact content while current readers keep strict schema admission; restore an old runtime only against its matching schema |

These routes improve local reasoning without introducing an engine/plugin framework. No source
inspection establishes general performance improvement or completeness outside the supported shapes.

## 6. Gates and profile assessment

| Gate | Judgment at this review boundary |
|---|---|
| G1 Authority | Satisfied for inspected ownership: shared vocabulary, admission and row/wire definitions replace independent classifiers |
| G2 Semantic fidelity | Satisfied for corrected supported shapes; declaration, exact storage, reader association and unknown remain distinct |
| G3 Validity | Focused controls support the changed boundaries; assembled qualification unresolved pending successful Delta publication and complete gates |
| G4 Hidden behavior | Satisfied by source inspection: analyzed defaults/decorators are not executed and pure derivation has explicit inputs |
| G5 Consistency/recovery | Focused native cancellation and retained-baseline recovery have receipts; new-generation mixed recovery and operator cutover remain unresolved |
| G6 Transformation/reuse | Indexed/clean and probe-local Salsa equality have bounded evidence; both-profile live parity and final projection/hydration remain unresolved |
| G7 Truthful capability claims | Satisfied only at the labels and exclusions in this review; no complete PR2 or comparative product qualification is inferred |
| G8 Library leverage | Satisfied for the inspected mechanisms: Schemars/Serde, offline jsonschema, FastMCP, SQLx and Ruff provide their respective generic capabilities |
| CI-G1 Fidelity | No remaining source-level relabeling defect found after corrections; unsupported runtime behavior remains unknown |
| CI-G2 Evidence closure | Canonical attribution and generation-scoped hydration inspected; final live packet/restore qualification pending |
| CI-G3 Evaluation integrity | No sealed data inspected or evaluation inputs introduced in this review; comparative evaluation excluded |

## 7. Findings and dated correction evidence

All findings below route to the [forward-plan disposition owner](../../plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition).
Correction evidence is not a second mutable status register.

<a id="F01"></a>
**F01 — Exact association exceeded its construction/receiver proof.** Earlier code admitted
custom construction, treated ordinal-zero static/class parameters as instance receivers, and
checked only the selected constructor assignment. An unrelated setter or overloaded operator
could invalidate storage identity. An inner property observation also incorrectly established a
later accessor after opaque outer decoration. **FP-05/06, DP-02/08, CI-02/06; G2, CI-G1.**
The corrected surface owner checks every constructor effect, rejects same-name descriptor fields,
requires the bounded construction shape, checks instance binding, and requires one supported
preceding property/accessor binding. The expanded catalog fixture includes custom allocation,
metaclass, replacement, descriptor, setter-mutation, operator-mutation and replaced-property
controls. Source corrections and the supplied passing catalog receipt were inspected.

<a id="F02"></a>
**F02 — Nullable migration weakened required presence.** Plain Serde `Option` accepted omission
where prior Pydantic response fields required a key whose value could be null. **FP-02/05,
DP-02/03/24; G2/G3.** Required-nullable deserialization and its schema adapter now preserve that
contract; explicit old defaults remain defaults. Actual native fixture controls include missing
evidence-key refusal. An earlier Rust unit receipt still contains a stale nonhex-ID fixture failure;
its existence is not relabeled as a passing whole Rust suite.

<a id="F03"></a>
**F03 — Descriptor admission had two resolution policies.** Sharing only a flag predicate left
qualified/imported builtin observations with different catalog and behavioral admission.
**FP-04, DP-01; A2/G1.** Both consumers now use `admitted_descriptor`; richer metadata remains
available without widening body admission. Source-level closure was inspected.

<a id="F04"></a>
**F04 — Wire adaptation escaped bounded execution and final-size enforcement.** Initial request
and final response normalization ran outside the full deadline/worker envelope, native routines
retained the GIL, and normalization could expand unchecked output. **FP-03/05, DP-18/20; G5.**
The full tool path now uses bounded workers and one outer deadline, both native normalization
entrypoints detach, and final normalized packet size is checked. Actual native/FastMCP controls
exercise cancellation ownership and event-loop progress; their six-test receipt was inspected.

<a id="F05"></a>
**F05 — A canonical constraint was not plannable through the real Delta boundary.** The first
pilot stopped with `Boolean = BinaryView` while planning the reader-nullability equivalence.
**DP-15/23; G3/G6.** `CatalogFieldLinks` now expresses the same invariant as explicit AND/OR
branches. The production constraint correction is source-inspected; successful real-library
publication after this correction remains required and was not available at review time.

## 8. Library fit and alternatives

Schemars plus Serde avoids a second Python domain authority while preserving current owner-local
formats. Offline Rust jsonschema supplies independent structural conformance; it does not replace
generation membership, evidence or retrieval-policy checks. SQLx retains transactional ownership;
the custom FastMCP Tool is a transport adapter rather than a new domain interpreter.

Pure indexed functions are simpler for ordered decorators, MRO selection and finite field joins
than a production Salsa or Ascent engine. The separate Salsa experiment retains the pinned 0.28.2
family and checks same-fact root changes. Persistence is an isolated DTO experiment with a producer
envelope, not production catalog or ty serialization. Ascent's genuine recursive-summary candidate
remains in S4. These are bounded library-fit decisions, not claims that the libraries lack other
capabilities. Future adoption requires the existing workload/compatibility trigger.

## 9. Tradeoffs

Keeping Python semantic models would simplify local Python reflection but preserve dual validation
authority. The selected thin packet views remove that duplication while making native boundary
qualification necessary. General decorator execution or heap analysis would broaden semantics and
lifecycle substantially; the selected finite normalizer instead preserves source information under
explicit uncertainty. Retained-runtime recovery is preferable to relaxing current schema checks.

## 10. Verification and qualification limits

**Implementation-agent receipts inspected, 2026-09-28; none of these commands was run by this
reviewer.** Build paths below name ignored local evidence and do not imply durable publication.

| Command or receipt | Observed outcome and limit |
|---|---|
| `cargo test --release -p cpg-core --test catalog` | **passed:** one expanded canonical catalog fixture; `build/pr2-catalog-tests.log` |
| `uv run pytest python/lctx_mcp/tests/test_wire_contract.py -q` | **passed:** six actual native/FastMCP controls; `build/pr2-wire-python.log`; these are transport controls, not live PostgreSQL serving |
| `cargo test --release -p lctx-postgres --lib schema_tests` | **passed:** existing owner contracts and synthetic credentials; `build/pr2-pg-schema.log` |
| Fresh migration probe | Receipt reports **passed:** ten migrations and three new RLS tables; `build/pr2-migration-probe.log`. Invocation is not captured in that JSON receipt; this is not an import/hydration receipt |
| Retained-runtime `postgres_backup.py backup` and `restore-drill` | **passed:** 78 tables, four retained generations/artifacts and prior current-profile serving; `build/pr2-baseline-backup.log` and `build/pr2-baseline-restore.log`; this protects schema009 recovery, not PR2 cutover |
| `cargo test --release -p lctx-postgres --test serving catalog_specificity_import_and_typed_hydration` | **failed** in the inspected receipt with incompatible projection definition/catalog; `build/pr2-pg-specificity.log`; requires a current-artifact rerun |
| `uv run python docs/design_review/evidence/2026-09-28_pr2/pilot_probe.py build/pr2-pilots` | **failed** in its first catalog compile on F05; corrected-source rerun pending |
| `just test-all` | Latest inspected `build/pr2-test-all.log` **failed** on large enum variants; repairs/reruns are implementation work in progress, not an accepted complete gate |
| Final current-artifact profile parity, mixed recovery and operator cutover | **not_run** in completed evidence inspected by this reviewer |

The Salsa receipts establish equal probe-local output for the listed same-fact root cases and a
DTO persistence round trip. They do not establish production cache safety, cross-release reuse or
a performance advantage. Earlier failing snapshot/unit logs are retained failures until a named
successful rerun supersedes them; the passing Python controls do not certify those Rust selections.
The [evidence owner](../evidence/2026-09-28_pr2/README.md) holds experiment and baseline details.

## 11. Authority and disposition

ADR-0073/0074 and [product §14](../../design/sections/api-and-evidence-product.md) remain the
architectural owners. The forward plan owns AP/F02, CLF/F01–F03 and any scheduled findings from
this review. Source conformance supports bounded correction evidence; closing product acceptance
requires the actual successful gates, current-profile import/hydration, live packets and recovery
receipts. Existing PR3–PR6 and retained behavioral obligations are not closed here.

## 12. Architectural judgment

- **A1 satisfied for inspected scenarios:** semantic transforms have explicit inputs and local
  tests; storage, native transport and presentation have separate owners.
- **A2 satisfied after corrections:** required presence, shared admission, finite vocabulary,
  evidence and association fidelity are structurally represented at their owning boundaries.
- **A3 satisfied for inspected composition:** existing profiles and tools consume shared contracts;
  derived projections remain rebuildable and no speculative engine is a production dependency.

FP-01–FP-06 are satisfied for these source-level change scenarios. Applicable correctness and
profile gates remain independent: live Delta publication and assembled qualification are still
unresolved. **Accept only scoped source conformance. Do not report PR2 implementation or deployment
fully qualified from this review.** Reinspection is required if the pending failures need semantic,
ownership or admission changes; a successful rerun alone should update qualification evidence
rather than silently broaden the review's architecture scope.

## 13. Qualification follow-up from the implementation owner

**Tested and Measured, 2026-09-28; recorded after the source review.** The
[PR2 evidence owner](../evidence/2026-09-28_pr2/README.md) now supplies successful corrected-source
publication, both live profiles through PostgreSQL/stdio MCP, twelve-relation catalog parity and
current/mixed-format recovery. These supersede the F05 pilot and stale-artifact hydration failures
listed above. Source corrections also removed a misplaced capability-request check from
`inspect_value_paths`; its actual native/MCP regression passes. The expanded association control
preserves three unknown record-field fates, their scopes and hydrated transfer kinds, while refusing
fabricated fates for an unsupported plain class. The pure-input control preserves canonical row
multisets under input reversal and exercises membership, missing syntax, provider and evidence changes.

The complete `just test-all` passed 458 Rust, 188 Python, 19 real-PG Rust and two PG Python tests,
including the corrected database fixture versions and final SQLx metadata verification. Explicit
operator migration010, both live profile imports/smokes and behavioral selection passed. The populated
restore preserved all 81 tables/six current and retained generations, both current-profile serving
paths and the exact selection in 21.61 s. The evidence owner records the full gate log, the concurrent
environment-policy boundary, cutover receipt and actual deployed runtime checksums. The forward plan
closes AP/F02 at bounded Tested strength and retains the named PR3–PR5 CLF extension obligations.
This follow-up does not broaden the source review into a general semantic,
performance or comparative product certificate. The reviewer did not execute these commands.
