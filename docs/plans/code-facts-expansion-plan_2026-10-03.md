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
polarity and precision-loss controls. Pyrefly exit/terminal/capture payloads and first behavioral
interpretation remain open; this is not M3 closure.
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
N2, B4 and P1 remain in isolated executor worktrees. Remaining M2 native enrichments, M3 timing
and first behavioral consumers, downstream packages and Q0/full hygiene
remain open/not_run. Real-library reconstruction/activation remain separately authorized follow-up.
