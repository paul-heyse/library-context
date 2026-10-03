# Code-facts expansion: coordinated implementation plan

**Proposed, 2026-10-03.** This series translates the
[expanded target review](../design_review/reviews/design_review_code-facts-expanded-target_2026-10-03.md)
and its [source review](../design_review/reviews/design_review_code-facts-opportunity_2026-10-03.md)
into implementation work. Inspected baseline: `main` at `3c15bc01`, preserving the separate,
uncommitted serving-qualification runner and source-default repair. Core principles 3.2,
code-intelligence profile 1.3 and the [repository binding](../design_review/design_principles/binding/library-context.md)
apply. Source conclusions are Interface-checked; all implementation and acceptance below are
planned. This document is the **sole mutable execution and finding-disposition owner** for this series.

## 1. Outcome and completion boundary

The target is a richer, correctly attached fact input and an explicit first interpretation for
each selected addition. Canonical latest-Ruff syntax, independent Pyrefly typing and ty flow
observations feed the existing model, normalization, finite behavior, analytics and catalog.
Generated packets retain the same distinctions and evidence. A fact is not finished because it
was stored: its scheduled first consumer and refusal behavior must work.

The model remains the semantic authority, PostgreSQL the single relational store, DataFusion
in-process compute, and synthesis programmatic. Preserve five verdicts, finite models, bounded
BDD conditions, source defaults as Unknown when not proved, public eligibility independent of
ranking, and Catalog Flow as NotRequested. Do not introduce a new heap analysis, universal
provider admission registry, second type authority, alternate store or compatibility readers.

Completion covers pinned migrations, selected payloads C1–C12, first operations, downstream
consumer migrations, fixture journeys and full functional/non-functional gates. Real-library
reconstruction, serving activation, operator registrations, retrieval-quality studies and live
embedding/vector qualification are a **separately authorized follow-up**, as selected by the
operator. The stopped Phase 5 checkpoint stays at [Phase 5 §10](semantic-model-phase5-detailed-plan_2026-10-01.md#10-finding-routes-limits-and-current-state).
PR6 and new features remain separate. Do not use this series to resume the interrupted pilot.

## 2. Reading route and responsibility

| Plan | Owns | Exchanges with the other plans |
|---|---|---|
| [Analyzer migration](python-analyzer-migration-plan_2026-10-03.md) | Pins, fork seams, source identity, canonical syntax and owned provider payloads | Source-correspondence and role-labelled observations; no product/behavioral admission decisions |
| [Facts and normalization](code-facts-normalization-plan_2026-10-03.md) | Formal/slot correspondence, callable roles, generics, metadata, aliases, located/member evidence | One normalized interpretation of identities and roles, with explicit availability |
| [Behavioral analysis](code-facts-behavioral-analysis-plan_2026-10-03.md) | Assumption basis, typed truth use, captures, dispatch, terminal/exit and exception completion | Qualified decisions, witnesses and finite summaries; characterization does not become runtime proof |
| [Analytics and product](code-facts-analytics-product-plan_2026-10-03.md) | Analytic inputs, typed selection/facets, original evidence, synthesis and packets | Role-aware grouping/query results and generation-bound output that preserves basis |

Enduring authority stays at the [architecture owners](../design/README.md), their ADRs and
`lctx-model::domain`. Plans specify target changes; they do not silently amend accepted decisions.
M0 owns the ADR route for independent Ruff and changed parse/pin policy. B0 owns the shared
qualification/basis decision. Signature/selection authority changes are recorded with N1/P2.
Supersede immutable accepted decisions where necessary and amend their owning sections together.

## 3. Foundations and shared decisions

The focused authoring assessment retains generation publication, invariant validation, native
preparation, canonical signatures/parameters and charged bounded kernels. It schedules three
foundational improvements rather than replacing these components:

1. **Source correspondence:** source snapshots/views, occurrences, formal declarations, native
   slots and synthetic variants have distinct identities. One model operation resolves those
   correspondences. Fix the existing parameter join now; do not distort type subjects to repair FCA.
2. **Provider family identity:** independent latest Ruff and ty use one source revision; Pyrefly
   keeps its embedded Ruff family. Cargo nominal source identity matters in addition to versions.
   No AST casts, equal-version mixed-source structs, or artificial embedded-version ceiling.
3. **Qualification basis:** behavioral use of a typing observation introduces typed premises.
   Conjunction combines premises; alternatives retain their own premise sets. A conditional result
   neither deletes the conservative runtime result nor infects its unconditional sibling.

Latest Ruff owns canonical syntax and stable model node kinds. Pyrefly's embedded AST remains
an adapter detail; ty's flow view is explicitly mapped. Matching byte ranges alone is insufficient:
attachment also checks source/view, node kind, role and uniqueness. Synthetic slots use native
variant/ordinal identity and optional source correspondence, never invented source locations.

Question-specific policies share identity and qualification composition. Catalog typing,
heuristic grouping and runtime completion need different admission rules. All keep source status,
coverage, approximation and assumptions as separate dimensions. A new named signature role can
extend catalog/selection without changing declared ParameterType; a new behavioral model extends
existing finite transfer without modifying the extractor. These are extension scenarios, not
extra implementation scope.

Pinned provider facts and interpretation traps come from the
[supply contracts](../design_review/evidence/2026-10-03_code-facts-expanded-target/supply-contracts.md)
and the live `python-analyzers` Rust source index. This task uses that source route, following the
operator's instruction; CLI/Python Context7 material does not establish Rust embedding contracts.
The source inventory is 340 records/137 concepts; only the selected 41 payload routes and
C1–C12 consumers enter this series. A catalogue entry is not an instruction to store every fact.

## 4. Execution dependencies and ownership

Package status and dated scoped receipts are in §7; unlisted packages remain **open**. Companion documents describe delivery, not a second status list.
Prerequisites denote working, tested contract slices, not merely agreed type sketches.

| Package | Working prerequisite | Delivered boundary / implementation responsibility |
|---|---|---|
| M0 | Current tree and reviewed exact supply | Architecture/pin policy; compiling narrow seam prototypes; root owns shared manifests/ADRs |
| N0 | Existing source facts | Model-owned source-formal-slot correspondence plus existing consumer migration with P0 |
| P0 | N0 | Correct Parameter/ParameterType, receiver exclusion and type-layer membership |
| N4a | Existing import observations | Intra-root import repair and per-alias identity where current provider supports it |
| P2a | Current predicate contracts | Unsupported facet admission/refusal before grants/effects |
| B0 | Current qualification and vocabulary epochs | Assumption-set identity, composition and all existing qualification consumers migrated |
| M1 | M0 seam and source-policy controls | Locked Pyrefly/Ruff/ty/CLI migration, native adapter parity and family checks |
| M2 | M1 | Latest-Ruff canonical syntax/populated contextual facts and shared provider attachment |
| M3 | M1, B0 for conditional interpretation | ty narrowing/timing payloads, precision provenance; Pyrefly protocol trace payloads |
| N1 | M2, N0 | Source/effective/synthesized/overload variants and slot types, catalog role consumer |
| N2 | N1 | Generic binders/substitution and role-aware specialized query inputs |
| N3 | M2, N1 for generated constructors | Class/member metadata and one record-option interpretation |
| N4b | M2, N4a | Full per-alias module identity, partial exports and public paths |
| N5 | M2, N1 | Selected located/expected types and field/member identity |
| B1 | B0, M3, N5 operand attachment | Atom decisions actually govern conditional restriction/transfer |
| B2 | B0, M3 capture/timing payload | Capture explanation and bounded stable-capture transfer |
| B3 | B0, N3, N5, M3 protocol/exit payload | Basis-labelled dispatch, protocol phase, terminal/exit completion |
| B4 | Current exact-class/source controls; B0 for typed additions | Ordered typed handlers and bounded exception values |
| P1 | P0; relevant N1–N5/B2 slices | Declared analytic roles, attributes and separate graph/frame inputs |
| P2b | P2a, N1–N5; B4 for behavioral raise predicates | Role-explicit catalog/selection and typed facet values |
| P3 | M2/M3 and normalized target/variant identity | Selected diagnostics/pytest/usage/original evidence consumers |
| P4 | B0; completed output slices P1–P3/B1–B4 | S0 and generated wire preserve role, premises and citation closure |
| Q0 | All functional packages | Assembled disposable-PG fixture journeys, snapshots, test-all and hygiene |

N0/P0, N4a, P2a and exact B4 are useful before analyzer migration; they repair current facts
without interim semantic authorities. M0 and B0 settle the largest uncertainty first. N1 supplies
working role identity before N2/N3 consumers; P2a refusal needs no future facet population.
Implement a producer with its first scheduled consumer as a coherent slice. P4's wire/basis
contract work starts with B0; its final output journey waits for the richer inputs.

Parallel branches are possible after these boundaries exist. Logical independence does not
grant concurrent edits to `domain` declarations, shared qualification, stage inputs, manifests
or generated-wire definitions. The coordinator assigns one integration writer for each shared
surface; delegated executors get bounded file ownership and preserve concurrent edits. Follow
the repository roles/model routing; no global build serialization or cache cleanup is required.

## 5. Coverage and source-review traceability

The [consumer ledger](../design_review/evidence/2026-10-03_code-facts-expanded-target/consumer-ledger.md)
is the retained finite selection rationale, not another execution register. C groups route all
selected payloads; competing/oracle and deferred entries keep their explicit consumer triggers.

| Selected contract | Producer → first operation → output packages |
|---|---|
| C1 Ruff semantic/syntax/context | M2 → correspondence/recognition → N1/N3, P1/P3 |
| C2 signatures/overloads | M2 → N1 → P2b/P4; B3 only with admitted target basis |
| C3 generics/typed slots | M2 → N2 → P1/P2b/P4 |
| C4 metadata/record/closure | M2 → N3 and B3 → P1/P2b/P4 |
| C5 imports/exports | N4a/M2 → N4b → P1/P2b/P3 |
| C6 located/expected/member | M2 → N5 → B1/B3, P1/P2b |
| C7 captures/timing | M3 → B2 → P1/P4 |
| C8 narrowing/actual truth use | M3 → B1 → qualified Summary/P4 |
| C9 implicit/terminal/exit | M3 → B3 → P1/P4 |
| C10 exact exceptions/handlers | Current facts/M2 → B4 → P2b/P4 |
| C11 parameter and richer analytics | N0/N1–N5 → P0/P1 → P4 |
| C12 diagnostics/framework/usage | M2/M3 → P3 → original evidence/P4 |

### Sole current finding disposition

`O-Fxx` means the opportunity source review; `E-Fxx` means the expanded review. The original
source IDs remain unchanged. **Scheduled/open** is not accepted implementation or verified closure.

| Source finding | Scheduled owner / correction | Closure evidence required; current disposition |
|---|---|---|
| O-F01 typing admission | B0/B1/B3/P4; shared premises and question-specific admission | Conditional truth, composition and packet controls; scheduled/open. Unsafe current served refutation is unestablished |
| O-F02 effective signatures | M2/N1/N2/P2b | Wrapper/generated/overload roles and slot-type query journey; scheduled/open |
| O-F03 dispatch closure | N3/B3 | Exact versus typing-final/open target basis, override control; scheduled/open |
| O-F04 typed handlers | B4/P4 | Ordered tuple/subclass handlers, finally/reraise and finite Summary; scheduled/open |
| O-F05 intra-root imports | N4a/N4b/P1/P2b | Per-alias roots/re-export/namespace and partial exports; scheduled/open |
| O-F06 stale extraction claims | M0/M1/N1; architectural owners | Source-current role/pin/coverage documentation and docs-check; scheduled/open |
| O-F07 record options | N3 | Effective options with source disagreement/default limits; replaced interpretation deleted; scheduled/open |
| O-F08 located-type deferrals | N5/B1/B3/P2b | Receiver/member/expected role controls and named consumers; scheduled/open |
| O-F09 erased predicates/AMBIGUOUS | M3/B1/B3 | Distinct reachability/narrowing and completion certificates, limit controls; scheduled/open |
| O-F10 ty/index/one-parse policy | M0/M1/M2 | Independent latest Ruff, role/source family controls, retained ty flow limits; scheduled/open with review refinement |
| O-F11 unsupported facet | P2a/P2b | Honest supported evaluation and unsupported pre-effect refusal; scheduled/open |
| E-F01 populated independent Ruff | M0/M1/M2 | Compiling populated seam, context/attachment/configuration fixtures and ADR/owner pivot; scheduled/open |
| E-F02 dormant predicate truth | B0/B1/P4 | Literal/exact-class truth changes Summary under correct basis; empty/mixed/nonconforming controls; scheduled/open |
| E-F03 parameter identity | N0/P0 | Independently specified parameter/receiver incidences and release-class type-layer test; scheduled/open |

Do not close a shared finding on a provider build alone. Add dated command receipts and remaining
limits here as work completes. A later discovery must link its source and responsible component
rather than renumbering these findings. The source reviews retain dated assessments only.

## 6. Migration, retirement and acceptance

Each declaration change updates generated schema/digest, stage dependencies, invariant inputs,
PostgreSQL store projection and generated wire consumers together. Codes are append-only; inspect
`.snap.new` before `cargo insta accept`. These are schema migrations, not formatting changes.
No old-format reader/defaulted missing premise is allowed. Mid-series checkpoints may compile and
pass their scoped fixtures while the cumulative target is incomplete; do not select them as a
qualified operator generation.

Future execution quiesces affected readers before any store replacement. Q0 uses disposable PG18
and rebuilt fixture generations, without resetting the operator's stopped staging. Activation
follow-up inventories old generations/artifacts, preserves required receipts, then retires
obsolete runtime state and reconstructs from the new pins. Interrupted staging is never selected;
resume/abort uses current generation commands, not an old-schema rollback copy.

For Q0 hygiene, provision a current disposable store through existing PostgreSQL test support
and set `LCTX_DATABASE_CONFIG` to its application configuration before `just hygiene`:
`just store-check` otherwise checks the operator's old live schema. Keep that store alive through
the gate and verify the same-tree model there. Do not reset stopped operator staging to make a
fixture qualification pass. Configure service/migration roles through existing test support;
never substitute superuser credentials in product execution.

During implementation use normalized build environment and targeted release-profile Cargo checks,
tests and provider parity controls. Keep stable caches, jobs16 and default frontend1. After all
functional scope, Q0 runs `just test-all` and `just hygiene` once; repair findings and rerun named
failed checks. Hygiene includes full Clippy, dependency/fork policy, agent/ADR/documentation checks.
Formatting and generators remain end-of-turn-hook work. Generated refresh timing must precede
claiming the final same-tree gates; use a documented scoped exception only if needed and authorized.

Required assembled journeys, independently specified fixture expectations:

- Both profiles compile through Catalog into real disposable PostgreSQL; Catalog Flow remains
  NotRequested, behavioral Partial reasons remain truthful. Publication checks new vocabulary,
  attachments, variant roles, assumptions and same-generation referents.
- Decorated/generic/generated APIs serve invocation roles, slot types, source-default uncertainty,
  aliases, fields and original evidence. Declared versus effective selection gives different,
  correct answers; unsupported predicates fail before effects.
- Literal/exact-class/capture/handler/context-manager examples yield finite expected verdicts,
  with conditional versus unconditional alternatives preserved into Summary, S0 and packet bytes.
  Empty domains, unknown exit, dynamic raise, override and lazy capture remain bounded unknowns.
- Fixed communities/PageRank/FCA/RCA/type/mention fixture policies retain universes and weights,
  expose corrected parameter inputs and provenance, and avoid duplicate votes. Embeddings and kNN
  are off/NotRequested for this acceptance; no quality or speed claim follows from these journeys.
- Rust/generated wire/native/Python controls cover changed required roles/basis and source-default
  behavior, including preserved concurrent serving changes when integrated by their owner.

Follow repository review cadence for changed architecture/behavior before closing their packages;
independent implementation assessment is evidence for its bounded scope, not a replacement for
functional gates. New authoring delegation could not start due to the agent thread limit; the
root performed the focused foundation/source assessment. No second architectural verdict is claimed.

Measurements of extraction cost, memory, conditional-coverage gain and retrieval quality are
deferred, with triggers at representative real-library reconstruction/product evaluation. Their
absence does not block fixture/test/hygiene completion and cannot support a Measured benefit.

## 7. Current checkpoint and next action

Execution is in progress, 2026-10-03. M0 fork seams are implemented and passed their isolated
native controls; workspace migration/consumers remain open. ADR-0117/0118 accept independent
canonical Ruff and scoped nominal source families; acceptance does not establish M1/M2 closure.

| Package / command, 2026-10-03 | Outcome and boundary |
|---|---|
| N0/P0: `cargo check --release -p lctx-model --quiet` | **passed**; shared model-owned container→formal→native-slot attachment; analytic parameter/type/receiver joins use it |
| N0/P0: `cargo nextest run --release -p lctx-model --test parameter_correspondence --test domain_selection --test serving_contracts` | **passed**, 32 controls including all five source parameter kinds, same-range distinct containers, descriptor/context/owner controls, missing/ambiguous/cross-source refusal |
| P0: `cargo nextest run --release -p cpg-core --test analytic -E 'test(selected_algorithms_publish_actual_nominal_results) or test(selected_vector_failures_remain_visible)'` | **passed**, two actual disposable-PG analytic journeys; eight value parameters, defaulted flag, typed incidences and exactly six release-class type-layer pairs per frame |
| P2a same model/wire controls | **passed**; shared predicate validation refuses unsupported facets before classification/request admission. First wire test failed because search_capabilities has no selection input; corrected to compare_operations, named rerun passed. Python adapter control is added, **not_run** until current native refresh/Q0 |
| M0 Pyrefly isolated release native session + paired unmodified-tag parity | **passed**; normal/exceptional exit, per-route applicability/async status, terminal guard controls, invalid locations, no residual Type::Var and identical native traces/hover/diagnostics; current-tree consumer acceptance **not_run** |
| M0 Ruff isolated release harness, no-observer lint parity, ty narrowing controls | **passed**, eight owned-observer controls, one lint parity and six precision/algebra controls; canonical parsed reuse and explicit-src configuration; current-tree consumers **not_run** |

Fork sources/receipts are retained under `/home/paul/.cache/lctx-code-facts/`. Immutable Pyrefly
revision `72bb34d6d67c2bc14720c77e2ad7eff6b89d360f` has parent `80cec3f57364bc11d4a39a419f6894a8eabcaa00`;
Ruff revision `8f01d80020921d3867f255ee5f919dd2d329b730` has parent `3265ed1f944c98bb4c04d632fbefb1257cdb583d`.
These are provider seam receipts, not full-series acceptance. M1 pins/API port are committed at
`1946cff1`; release extraction/flow all-target checks passed, 19 policy controls passed, and the
current-tree `cargo nextest run --release -p cpg-extract --test ruff_context --test typed_types
--test typed_flow --test harness` passed 14 controls. CLI versions were verified as Ruff 0.16.10
and Pyrefly 1.4.0-dev.3 after `uv sync --no-install-workspace`.
N4a is integrated at `ef7a50ed`; its executor release receipt passed 43 controls including actual
per-alias extraction, normalization/projection and PostgreSQL roundtrip. Integration against new
pins is being requalified with M2.
M2 canonical syntax/context is committed at `789cd0ac`: latest Ruff owns parse/traversal,
owned static branch adapters retain Pyrefly decisions, contextual records preserve active-node
versus final-reference phases, and overload recognition consumes actual qualified names. The
initial focused run failed 16 controls on the old single-provider stage coverage assumption.
ADR-0119 accepts finite family/provider coverage grants; frontier expectations and schedule
identity now derive from those exact pairs. `cargo check --release --workspace --all-targets
--locked` passed on 2026-10-03. The first admission rerun failed seven fixture-stage
output declarations; the fixture now declares the actual contextual relations. Native rerun
passed 29/31, with two coverage mismatches caused by augmented, deletion and nonlocal reference
roles. Exact structural reference attachment repaired those mismatches. The final combined
`cargo nextest run --release -p cpg-extract --test typed_lexical --test typed_syntax_shapes
--test typed_documents --test typed_ruff_context --test ruff_context -p lctx-model
--test domain_stages --test domain_admission` **passed**, 34/34, zero skipped. Earlier native
controls for owner/limits/conformance/per-alias/type/parity also passed within the composite
receipt. These are scoped controls, not complete M2 enrichment or Q0 acceptance. ADR-0120
accepts required canonical assumption bases before conditional behavior activation. B0 is
integrated at `23c9dd87`, with source/fixture ports at `e7211c7b`. Its isolated four-crate
check, 16 model controls, 22 existing qualification/native/transfer controls, actual Summary
alternative publication, both premise arms through PostgreSQL packets, transfer-store lifecycle
and Python required-wire controls **passed**. The initial root workspace check found a fixture
catalog constructor and newly appended predicate renderer/default variant; these are ported.
This is a composite scoped receipt, not full gate acceptance.
M3 flow payloads now preserve separate narrowing formulas, typed nonterminal/iterable/exit
predicates and the exact transformed TYPE_CHECKING view identity. `cargo check --release
-p cpg-flow -p cpg-extract --all-targets` **passed**. `cargo nextest run --release -p cpg-flow
--lib -p cpg-extract --test typed_flow -p lctx-model --test domain_admission` **passed**,
134/134, zero skipped, including actual native narrowing/source-view extraction and uncertainty,
polarity and precision-loss controls. First behavioral interpretation and ty capture timing
remain open; this is not M3 closure.
N1 is integrated at `cf0731db`: source/effective/synthesized roles, native slot/return ports,
chosen-overload versus candidate traces and ordered result-value Overloaded terms. Its isolated
three-crate all-target check **passed**; composite native/model/selection/real-PG receipt passed
68/69, then the failed Arguments-range lookup was repaired and the native binary rerun **passed**
2/2. N3/N5 is integrated at `bdddd079`/`b3fbdf87`; final frozen
`cargo test --release -p cpg-extract --test typed_class_metadata -p cpg-core --test catalog_core
-- --nocapture` **passed**, two actual native controls and one real-PG catalog journey.
M3 now emits owned located Pyrefly exit/terminal observations and native capture origins in both
profiles, preserving diagnostic phases, guarded decisions and unknown timing/mutation. The
initial protocol/metadata/B0 native integration **passed** 26/26. After N1/capture integration,
`cargo nextest run --release -p cpg-extract --test typed_protocols --test typed_captures
--test native_callable_variants --test typed_class_metadata --test typed_flow -p lctx-model
--test domain_admission --test domain_assumptions` **passed**, 29/29, zero skipped. No concrete
capture-value bridge, runtime suppression or unconditional divergence follows from these raw rows.
N2 is frozen in executor commit `1b13a5e`: located receiver/binder mappings, declared variance
and bounded specialization overlay are exposed through `Prepared::specialize_port`. Its composite
23-control receipt includes two repaired fixture identity failures and three actual native →
normalized → prepared-query controls; merged model/extract/postgres all-target check **passed**.
Ordinary generic functions lacking a native exported binding mapping remain Unknown. Native
callable deprecation is separately frozen at `3baf387c`: availability, untrimmed optional message
and exact leaf metadata origin are retained independently of the runtime overload identity.
`cargo nextest run --release -p cpg-extract --test native_callable_deprecation --test
native_callable_variants --test native_generics -p lctx-model --test domain_types` with the
executor's named-test filter **passed**, seven selected controls, fifteen outside-filter skipped.
Root integrated these slices as `28e7ba1e` and `92e09afc`; neither establishes a complete
product/series exit.

B1 located atom decisions/restrictions are implemented in the working tree, including a separate
TypeConformance basis, conservative original alternatives and explicit Uninhabited semantics.
Initial combined Local/protocol controls **failed** on duplicate global invariant registration;
registration was repaired. Two subsequent scoped runs **failed**, each nine passed/eight failed,
on existing source entry/field controls before the new transfer oracle. Investigation found
N1 source/effective formal-link ambiguity and an M2 placement proof still requiring Ruff's
provider ID to equal Pyrefly's. `8a31138d` selects source roles and checks canonical placements
against their own attributed run/surface/family in the exact captured frame. Native declaration
checks retain their Pyrefly provenance. The revealing native rerun `cargo nextest run --release -p cpg-extract --test local_semantics
-E 'test(native_local_parameter_receiver_and_class_receiver_return_transfers_are_witnessed)'`
**passed**, one selected control/twelve outside-filter skipped. Wider Local/PG repair rerun
**failed**, fourteen passed/seven failed: one unsupported compound type trace made the whole
module's Types inventory Partial, refusing unrelated available finite domains. A cached native
isolation control **passed** with only that compound case temporarily absent; the case was restored.
The repair publishes selected `TypeQueryObservation` availability and admits an available query
only with its exact observation/subject/role/run/surface when inventory loss is MissingEvidence.
Missing, partial, source/parser/resource and foreign-frame evidence remain refused; partial
inventory never proves absence. The extraction/model all-target release check **passed** before
the final uniqueness/refusal controls. A subsequent compile **failed** on a role tuple cast;
that cast was repaired; the focused Local/protocol/generic/deprecation suite **passed** nineteen
and **failed** two native fixtures on an over-strict availability uniqueness key. Declared versus
inferred returns and individual overload-candidate answers are distinct native results. Query
identity now preserves declaredness and the selected observation, rejecting contradictory
availability for the same result without imposing a single-answer inventory. The affected suite
and new typed-facet controls are running against that refinement.
Canonical Ruff syntax now also survives an unavailable native typing parse, with source-byte
verification and separate contextual coverage. These source/query repairs remain scoped
Implemented checkpoints pending final controls; no B1 acceptance or Summary/packet cascade is claimed. Logs preserve all failures under
`/home/paul/.cache/lctx-code-facts/b1-*.log`.

B4/P1 are integrated with scoped final receipts below. M2 native enrichments, M3 timing,
remaining consumers and Q0/full gates remain open. Real-library/activation remain separate.

N4b integrated at `f4a77641` (executor `77e8ec5d`), 2026-10-03. Native pair 64 preserves
Complete/Partial/Unavailable export enumeration and basis; canonical `__all__` retains its
known source subset independently. PublicExposure separates public-path knowledge from alias
identity; the first model-owned public-path decision retains enumeration/path proof IDs and
permits negative absence only under complete exact unconditional native enumeration/support.
Actual computed/invalid/wildcard/shared/circular/private-positive/namespace/bundled/unresolved
controls **passed** 32/32 with
`cargo nextest run --release -p cpg-extract --test native_exports --test normalized_imports --test typed_symbols -p lctx-model --test domain_syntax --test domain_normalized --test domain_catalog`;
all-target model/extract compile **passed**. Composite failure/repair receipt:
`/home/paul/.cache/lctx-code-facts/n4b-receipt.json`. PG/persisted contract integration remains Q0.

Root atom/query plus first typed-facet batch `b1-query-p2-controls.log` did not establish a pass:
it reported missing FacetValue import (repaired), then its Cargo 723342 was interrupted after
an actual two-process lock cycle with P1 Cargo 792478 was confirmed using owned cwd/cmdline
and `n4b-locks.json`. Only the root participant received SIGINT; peer checks were preserved.
The batch is **failed** (exit 101), rerun pending. Root P2 now also implements a located
SpecializedType query using existing native substitution and the shared structural matcher;
global Specialized variant queries refuse before effects. This is a working-tree checkpoint,
**not_run** for the latest change and not P2 acceptance.

B4 integrated at `c115e623` (executor `d2e234a2`), 2026-10-03. Ordered exact builtin
exception values/handlers, causes/reraising/else/finally, named-handler disposal and finite
Summary exception selection are implemented. Witness 19 and required BehavioralExceptionPacket
fields are append-only/hard wire migrations. Final scoped release compile and five controls
**passed**, including actual disposable-PG body and Summary publication/required stored packet;
the affected Enriched and Summary regressions **passed** from those unchanged final compiled
binaries. Exact commands, binary digests and retained failures:
`/home/paul/.cache/lctx-code-facts/code-facts-b4-receipt.md`. Handler lookup at a conditional
source-call boundary retains actual refusal 48; it does not fabricate a completed exception
body. Integrated Q0/service qualification remains **not_run**.

Root `p2-located-controls.log` release batch ran 51 controls: **48 passed, 3 failed**.
All atom/query, native protocol/deprecation, and located specialization controls passed.
The remaining controls exposed an obsolete invalid type category (34 now means LiteralString),
a packet fixture missing the required explicit claim basis, and a wrapper-kind expectation
that confused a typed callable with a known runtime descriptor. Repairs preserve the actual
contract: category 35 is invalid, the fixture carries canonical empty basis, wrapper Function
kind remains Unresolved, while true/false Module facets are specified independently.
The B4 merge also required preserving both witness 19/20 and removing a duplicated old blanket
facet-refusal arm. All-target model/extract/core integration compile **passed**; its initial
unreachable-pattern warning was repaired and is covered by the next compilation.
`p2-b4-b1-summary-controls.log` ran 84 controls: **82 passed, 2 failed**. All three earlier
repairs and native specialization controls passed. Failures were the old preparation inventory
count and the replay probe missing declared atom inputs. Both are repaired; the probe consumes
shared Summary evidence/projection macros. `b1-summary-repair.log` subsequently failed compilation
during the P4 DTO migration (older compiled model/new consumer). The assembled scoped rerun
`p2-facets-p4-b1-controls.log` **failed**, 41 passed and four failed. The replay probe passed
all twelve controls; remaining failures were the shared decorator projection, duplicate C2 input
registration in two service controls, and the actual conditional atom→Summary journey. No
conditional atom→Summary pass followed in the bounded repaired receipt below.


Narrow M2 correction integrated `fe942fe1` (executor `07aa8570`), 2026-10-03. Populated Ruff
context support uses Lexical independently of structural Syntax coverage. All-target check and
22/22 release controls **passed** with
`cargo nextest run --release -p cpg-extract --test ruff_context --test normalized_callables --test native_exports --test typed_symbols -p lctx-model --test domain_normalized`.
Receipt: `/home/paul/.cache/lctx-code-facts/m2-context-receipt.json`; additional M2/P3 remain open.

P1 integrated `1a7ad33d` (executor `02bbc1b5`), 2026-10-03. Native metadata/decorator/capture/
protocol characterization, explicit role/receiver policy and deterministic attributes are implemented.
Final unchanged disposable-PG Catalog metadata/RCA journey **passed** 1/1, including exact prior
RCA handoff. Four model/three FCA controls passed on the unchanged producer. Composite commands,
failures and receipt: `/home/paul/.cache/lctx-code-facts/p1-receipt.json`. No full-series or
behavioral-PG acceptance follows; later B2/B3 facts require their consumers.

P2 checkpoint `6bf7469f` implements typed facets and located specialization; its controls passed
in the 82/84 batch. Remaining facet consumers share normalized decorator identity with analytics;
qualification pending. P4 checkpoint `9cfd5854` declares packet role/native signature identity,
source-declared versus native typed ports, supports and full bases through one packet lease.
Source-default repair is integrated. Model/store all-target compile **passed**:
`p4-native-ports-check-final.log`; actual service/wire journey pending. Initial compile attempts
failed a duplicate command option and misplaced allowance block; repairs/logs retained. Remaining
facet/P4 model/store/core/CLI all-target check **passed** (`p2-structural-p4-check-repair.log`),
after derive-macro repair. Generated adapter/Q0/full test-all/hygiene remain **not_run**.


2026-10-03 integration repairs (qualification pending): `bc488d1c` declares existing pinned
Ruff codegen/index/database dependencies for P3; `cargo check -p cpg-extract --lib` **passed**
(`p3-manifest-prerequisite-check.log`). C2 now registers shared decorator input once, visits
every declared projection during replay, and persists canonical fact-witness rows before
request-time classification. The evaluator refuses a citation absent from that output. Source
qualified decorator names follow selected normalized binding identity and supported declaration
metadata; source spelling does not select a target. Witness codes 21–26 are appended.

Summary seeds now preserve actual supported Local atom-restriction alternatives alongside
their conservative original seeds. The real-PG control uses `finite_zero → identity` to
exercise interprocedural Summary production; direct Local-only return paths do not independently
produce Summary transfer witnesses. Original failed receipts remain retained.
`cargo check -p lctx-model -p cpg-core -p lctx --all-targets` **passed** after adding the
explicit Summary owned restriction input (`p2-witness-b1-integration-check-repair.log`).
Subsequent shared-projection replay repair and service witness-row assertions are under
`p2-witness-b1-p4-repair-controls.log`, **failed** during test compilation; repairs and reruns are below.
Full `just test-all`, `just hygiene`, adapter readiness and Q0 remain **not_run** while
M2/P3, B2/B3 and the final P4 cascade are being integrated.

P2 native facets **passed**, 2026-10-03, on the freshly compiled repaired release binary:
`/home/paul/.cargo/build/library-context/release/build/cpg-extract/51baf3cb72e8b073/out/native_callable_variants-51baf3cb72e8b073 --nocapture`
ran all three controls (`p2-native-direct-repair-controls.log`). This includes positive/negative
resolved decorator, source async, class metadata, raised-type, declared/effective return and
deprecation role distinctions. The assembled Nextest build stopped on the new service assertion's
ID display formatter; debug formatting is repaired. That aggregate is **failed**, not a PG pass.
The transcript-only Python controls are **blocked** on missing installed `lctx_semantics`;
`uv run --no-sync python -m pytest tests/scripts/test_qualify_serving.py -q` failed collection
(`p4-stdio-transcript-controls.log`). Native readiness precedes their final rerun.

The freshly compiled repaired release `domain_selection_catalog-8c112cc7a0538668 --nocapture`
**passed** all 17 controls (`p2-model-direct-repair-controls.log`), including the exact 78-row
classification preparation inventory and canonical replay. Native three/model seventeen are
a composite scoped receipt, independent of the still-open PG packet/conditional Summary cascade.
`p2-witness-b1-p4-repair-controls.log` drained **failed** (exit101, test assertion compile);
`p2-witness-b1-p4-repair-controls2.log` was **interrupted** (exit130) before tests while its
pre-M2 baseline was queued/rebuilding; only the root Cargo process was stopped. The current
merged output rerun is `p4-m2-merged-output-controls.log`, outcome pending.

B1 repaired atom→Summary publication **passed**, 2026-10-03: freshly compiled release
`summary_publication-151e47f63420a12f behavioral_summaries_publish_finite_proofs_and_validate --exact --nocapture`
ran the actual disposable-PG journey and all twelve replay controls (196.92s), log
`b1-summary-direct-repair-control.log`. Conditional restricted and conservative empty-basis
alternatives remain separate. This does not establish final S0/packet cascade acceptance.

Remaining M2 native lexical metadata integrated `f36441c7` (executor `b8ee625f`), 2026-10-03.
Native bindings/definitions, same-scope resolution and explicit outer/builtin uncertainty feed
normalized reference/declaration characterization. Context52 FinalUnresolved does not fabricate
missing flags. Release controls **passed** 17/17; all-target check **passed** before integration,
then the executor merged main and checked again (51.18s). Receipt:
`/home/paul/.cache/lctx-code-facts/m2-native-lexical-receipt.json`. P3 remains open.

P4 conditional Summary wording and shared fresh-binding admission integrated `b86ef8c7`: S0 and
brief rendering consume the exact qualification and name a nonempty premise basis; an empty
basis remains explicit. SourceCall fresh-binding evidence now refuses nonempty assumptions,
preserving its independent runtime question. `cargo check -p lctx-model -p cpg-core --all-targets`
**passed** (`p4-conditional-synthesis-check-final.log`, 42.10s) after migrating text callers.
The focused model/core/service release rerun is pending; full-series gates remain **not_run**.
