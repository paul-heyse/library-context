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

Execution is in progress, 2026-10-03. The table below owns the current checkpoint;
the detailed bounded receipts that follow retain their original baselines and failures.
ADR-0117/0118/0119/0120 govern independent canonical Ruff, scoped nominal source families,
finite family/provider grants and required assumption bases. Accepted decisions do not establish
assembled runtime acceptance.

| Current boundary | Implemented state and remaining acceptance |
|---|---|
| M0–M3, N0–N5 | Migration and selected normalization are integrated, with scoped native/model/store receipts below. Genuine native ClassTrait support is integrated at `603d1a63`; its six-class/provenance and disagreement controls and unchanged three-frontier PG acceptance passed within the scoped composite below. No report support is relabelled. Current-pin assembled qualification remains open. |
| B0/B1/B2/B4 | Required bases, conditional atom restrictions, bounded direct captures and exact finite exceptions are integrated. Their scoped producer controls passed; the completed packet cascade still requires assembled acceptance. |
| B3 | Producer integrated at `603d1a63`: bounded dispatch/protocol/terminal questions reach Summary, with 18 distinct scoped controls passed. Original Candidate/open runtime alternatives remain unchanged. Purpose-specific declaration inspection preserves report fidelity; executable reads remain native structural. Current-pin assembled qualification and P4 terminal serving remain open. |
| P0/P1/P2 | Parameter analytics, native characterization/roles, typed facets and stored witnesses are integrated. Scoped analytic/native/model/service controls passed; generated-role control and same-tree assembled journeys remain open. |
| Raised-type review correction | Independent assembled review found that retained nonmatching raises could incorrectly close an unavailable native raise trace. P2 now requires existing complete Types coverage at the same source/context for a negative, with append-only witness27; matching positives remain valid under partial coverage. Targeted model/service checks and two units plus the classification inventory control passed; actual mixed/complete native service controls and final independent review remain open. |
| Corpus mention review correction | The selected-analytics Q0 fixture exposed an Installed/Corpus input mismatch that silently discarded valid official co-mentions. P1 now consumes existing `CorpusLibrary` and follows only exact declared corpus→library links, retaining context, Document role, resolved universe and deduplication. Targeted check passed; actual split-input and unlinked-corpus controls remain open. |
| P3 | Diagnostics/pytest and usage are integrated. Native ten-control composite passed. Actual PG diagnostics and repaired usage both passed as a two-control composite; the repair retains opaque module/local assignment negatives alongside direct import/re-export positives. Current-tree assembled qualification remains open. |
| P4 | Native roles/ports, conditional Summary/brief basis and exception packet controls are integrated. Capture proof/source-correspondence is integrated (`16827a31`/`3b8a4a69`) after actual PG and required-wire controls passed. Same-tree assembled check remains open. Terminal output is active against integrated B3. Conditional/generated service controls require matching assembled CLI and runtime reruns. |
| Q0 | **not_run**: all functional packages, independent assembled review, current adapter, fixture journeys, snapshots, `just test-all` and full `just hygiene` are prerequisites. The both-profile stdio fixture is being extended to explicitly select fixed communities/PageRank/FCA/RCA/type/mention policies; kNN/embeddings remain off. Old operator staging and real-library/activation remain excluded. |

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


Merged P4/M2 output batch `p4-m2-merged-output-controls.log` **failed**, 2026-10-03:
31 controls ran, 24 model/S0 controls passed and all seven actual-PG service/refutation controls
failed at the same C2 `selection_witnesses` replay closure before serving. Later S0/Analytic
qualification vocabulary expanded the replay's blanket witness universe. `94d9f92a` bounds
qualification witnesses to actual immutable native signature/specialization input references;
an unreferenced later qualification cannot expand C2. All-target model/core/CLI compile
**passed** (70s, `p2-witness-epoch-repair-check.log`). The affected seven PG controls plus the
18 declaration controls are being rerun in `p2-witness-epoch-p4-service-repair-controls.log`.
The new service control independently compares declared int/effective bytes, true/false/unknown
facets and every returned witness's persisted row. Its focused compile **passed** after fixing
the comparison selector helper (`p4-role-selection-service-check-repair.log`); runtime pending.

Ruff explicit-root constructor pin integrated `273955e9` (executor `c61f3d42`), 2026-10-03.
The aggregate fork `f7bdff69e1fb94ab0ed5b340e977aac0d26e9301` has the same upstream parent
`3265ed1f944c98bb4c04d632fbefb1257cdb583d`; its delta from `8f01d800` is solely exposing
existing `new_with_src` publicly. Native separate-process explicit-root/CWD controls **passed**
2/2, exact fork policy **passed**, and locked extraction compile **passed** (60s).
Aggregate patch sha256 `b7154806d8d5106f02c225d35c4cb8802c869d551af4ce1dd2d391178deaa6f8`;
receipt `/home/paul/.cache/lctx-code-facts/p3-ruff-fork-receipt.json`. Prior parity receipts keep
their original revision; P3 consumer and assembled current-pin gates remain open.

P4 output-control checkpoint `7d7a0ba4`, 2026-10-03: actual service controls independently
specify source/effective typing differences and true/false/unknown structural facets, stored
witness identity, conditional versus empty-basis Summary/brief packets, and exact/dynamic
exception packets under body entry. `cargo check --release -p lctx --test conditional_output_packets`
**passed**, final 0.47s (`p4-conditional-exception-output-check.log`); role service control compile
receipt is above. Release PG runs remain pending in `p2-witness-epoch-p4-service-repair-controls.log`
and `p4-conditional-exception-output-controls.log`. A compiled control does not establish served
runtime acceptance. Full-series gates remain **not_run**.

B2 integrated `0a29d7cc` (executor `ee8749de`), 2026-10-03: native timing65 remains
characterization, while the checked direct synchronous caller-frame certificate supplies
formal-entry or primitive-literal value authority. Canonical Parameter and native Identifier
identities stay distinct through their actual supported child edge. Actual PostgreSQL control
**passed**, formal/literal finite Summary and eight refusal shapes (`b2-controls12.log`, 88.93s);
native/API/adapter/inventory **passed** four controls; Catalog and exact-exception regressions
**passed** two. Behavioral replay hydration repair's rerun14 remains pending. Root merged
all-target `cargo check --release -p lctx-model -p cpg-core -p cpg-extract --all-targets`
**passed** (79s, `b2-merged-all-target-check.log`). Shared Local restricted seeds, owned inputs
and empty-basis fresh-binding guard are retained. Capture packet cascade remains open.

The witness-epoch PG rerun **failed**, 23/25 passed (`p2-witness-epoch-p4-service-repair-controls.log`,
119.15s). C2 publication/replay and all five prior service/refutation controls passed; the two
new/changed controls failed after reaching actual service. Their independent oracles are repaired:
`AllApplicable` rejects an actual negative counterexample, while `AnyApplicable` absence remains
Unresolved under Partial coverage with its negative evidence retained. No domain completeness is
invented. The large-signature fixture is recalibrated to twenty complete long-name parameters,
with explicit serialized-byte checks between the unchanged default32KiB/expanded256KiB limits;
required native roles/proof dictionaries made the old seventy-parameter packet exceed both.
The named two-control rerun is pending (`p4-service-oracle-repair-controls.log`); existing 23 passes
retain their exact baseline. Full-series acceptance remains **not_run**.

B2 regression14 **passed**, 2026-10-03: actual PostgreSQL Behavioral Summary control plus
all twelve replay-corruption controls, 111.80s (`b2-summary-regression14.log`); prior failure13
remains recorded. Scoped producer composite totals eight passing controls across native/model,
formal/literal capture PG and Catalog/Behavioral/exact-exception regressions. This closes that
producer boundary, not the new P4 capture packet journey or the full Q0 gates.

B3 persisted RCA: nineteen actual LexicalResolution assertions have native Ruff support and
retained Pyrefly recognizer support with different origin/mode/fidelity. Rejecting the second
support conflated those authorities. The approved native-read eligibility repair requires actual
AnalyzerAssertion/NativeTraversal/NativeStructural attribution and matching native qualification
fidelity; recognizers remain retained characterization. Native/recognizer twin, recognizer-only,
mismatched-fidelity and actual B3 PG controls remain pending. Evidence:
`/home/paul/.cache/lctx-code-facts/b3-read-provenance.txt`; this finding's repair is not yet passed.

P4 conditional/exception service run **failed**, 2026-10-03, before packet assertions
(`p4-conditional-exception-output-controls.log`, 63.37s): the fixture-installed model and its
standalone CLI had different contracts after B2 integration. `cargo nextest` does not replace
the explicit `test-cli-build` prerequisite. A stable-main `cargo build --release -p lctx` is
queued (`p4-main-cli-matching-build.log`); the conditional and named role/budget controls will
be rerun against that matching CLI. This is not packet acceptance or a store reset; only
disposable fixture databases are involved.

The named P4 service rerun **failed**, 2026-10-03: one passed/one failed, 76.56s
(`p4-service-oracle-repair-controls.log`). The unchanged default/expanded budget control
passed. The role/selection questions passed before the generated-constructor control rejected
its normalized parameter identities. That control now separately requires native host typing
and resolves each returned identity to the actual `ParameterEntity::NativeSlot` with matching
signature/parameter; a `Source` entity remains forbidden. A native slot identity is not an
invented source formal. Focused compile and named rerun are pending; no production mapping
is relaxed. P3 diagnostics/pytest native rerun passed two controls; its bounded usage-evidence
cascade is also being implemented rather than closing P3 on diagnostics alone.

Scoped build recovery, 2026-10-03: an actual `/proc/locks` audit found an idle
worktree lock cycle between B3 and the new capture packet build: capture held model reads
while waiting for allocative, and B3 held allocative reads while waiting for the model.
The capture executor interrupted only its own audited Cargo process (1987503; run1
`p4-capture-pg1.log`, **interrupted**, not a PG result). Rustc progress resumed after
that process released its reads. Caches, profiles, stable paths and jobs16 are preserved;
this recovery does not establish a standing build serialization policy.

P3 diagnostics/pytest checkpoint integrated `9996f81f` (executor `fe6da6ad`),
2026-10-03: native pairs66/67/69, source characterization, same-range original-evidence
packets and selected suppression/location/framework roles. Native repaired controls **passed**
two/zero skipped; all-target compile **passed**. Actual PG remains pending. Duplicate pytest
fixture names follow the native last-retained definition identity, not invented ambiguity.
Receipt `/home/paul/.cache/lctx-code-facts/p3-source-characterization-receipt.json`.
This is a schema migration; snapshots/current Python adapter remain Q0 obligations.

The actual shared macro projection mismatch is resolved in source by `fbbb58ae`
(executor `13e4e1ba`): `SupportAttribution` exposes stored origin/mode and its generated
constructor supplies those fields. It adds no stored column or new admission policy.
Root's idle matching-CLI and test-check builds were audited at their exact main cwd with
no children, then interrupted (exit130); both receipts are **interrupted**, not passed.
Aligned main checks are running (`p4-p3-aligned-main-check.log`), followed by a fresh
standalone CLI and actual packet controls. B3 read eligibility remains separately pending.

Aligned main `cargo check --release -p lctx --test conditional_output_packets
--test serving_packets --test source_characterization` **passed**, 2026-10-03,
7m42s (`p4-p3-aligned-main-check.log`). This includes the two-field attribution
projection, P3 diagnostic checkpoint and the generated NativeSlot identity control.
Main's standalone CLI and actual conditional/generated/diagnostic packet reruns wait for
the assembled remaining P4 wire fields, avoiding an immediately obsolete binary.
They remain **not_run** at this baseline; no full gate or packet runtime pass is claimed.

B3 PG3 **failed**, 2026-10-03, 337.25s (`b3-pg-3.log`): the native/recognizer
read ambiguity is resolved and declared terminal/native exit controls passed, but the two
conditional final-class/member frontiers remain absent. Actual source attempts are now
Bound/SourceInspection/DispatchOpen. Their original Overrides qualification is Candidate1/Exact0
(conceptually a may-target), so `typing_target` refuses at its definite-only runtime-style guard
(Approximation48). The bounded repair retains the original Candidate target/open runtime
alternative. Only that checked Overrides route can anchor a separate TypingConditional question:
independent definite receiver typing, unique checked member identity, complete MRO and final
metadata with receiver-conformance/no-extra-overrides premises must establish the conclusion.
Candidate is never relabelled or admitted as a definite premise; non-Overrides Candidate,
Potential, approximate, property, unknown and competing/wrong-member inputs remain refused. Pure qualification controls and the unchanged three-frontier PG oracle
are pending. No full B3 or series acceptance is claimed.

The MroGap negative fixture is being corrected to an actual contradictory-C3 recovery-prefix
case. A missing-import base yields native Complete/empty recovery, so its spelling was not
evidence of a partial MRO. The expected negative operation remains unchanged; no source
heuristic relabels the upstream status. Decisive candidate/receiver/member provenance is retained
in `b3-target-refusal.txt`. P4 capture standalone CLI **passed**, 12m46s
(`p4-capture-cli1.log`); actual PG2 and required-field wire controls are pending. That one build
used a recorded `-j6` override; subsequent builds retain configured jobs16/frontend1. No
profile, cache or standing job-policy change was made.

B3 bounded qualification repair `cargo check -p lctx-model -p cpg-core --tests`
**passed**, 1m09s (`b3-check-6.log`), 2026-10-03. Release model controls **passed** six:
closure2 (original Candidate retained; separate receiver-based Definite question; competing
identity/property/incomplete-MRO refusal), read-authority2 (native/recognizer/fidelity and raw
canonical syntax), protocol/terminal2 (all24 origins/phases and negative terminal twins).
Receipts `b3-closure-1.log`, `b3-read-1.log`, `b3-model-3.log` and `b3-current-batch.json`
retain exact commands. Source-binding, actual unchanged three-frontier PG and terminal packets
remain pending; this is not a B3 package or full-series pass.


Remaining packet qualification, 2026-10-03: capture PG2 **failed** after actual fixture
publication, 187.99s (`p4-capture-pg2.log`), at operation admission before packet assertions.
Its declaration correspondence was checked through a native-value proof helper despite the
actual `ParameterDeclarationSupport` carrying ReportProjection fidelity. The located repair
will preserve that attribution for the source link while retaining strict native checks for
capture/timing/origin value proofs; actual support inspection and rerun remain pending.

B3 source-binding runtime **failed** (`b3-binding-2.log`) at default-model validation:
`SummarySupportSource::TerminalFrontier` targets `SummaryTerminalWitness`, omitted from the
cumulative `summary_replay::relations` owner list. The custom stage fixture declared it;
the production cumulative model must also declare it. The bounded owner-list repair and
membership regression are in progress; no global registry or compatibility path is added.

P3 final usage/native batch **failed**, nine passed/one failed, zero skipped. Aliased,
re-exported, rebound, receiver, chosen-negative and higher-order Potential controls passed;
an overloaded local alias remains native UnresolvedTarget/UnexpectedDefiningClass rather
than an invented association. The C2 failure is an overstrict support-location check:
actual ProviderCallSite support is Invocation evidence. Admission will retain that exact
support only when its invocation run equals the support run and native/context/input checks
hold; canonical site and CallEventSource supply location. Named rerun and actual PG remain
pending. The Q0 stdio runner now addresses only a unique complete Source-role formal for
scalar inputs; NativeSlot identities are not source-body input substitutes. Its fresh runtime
qualification remains pending the assembled model and Python adapter.


P3 repaired usage/native batch **passed**, 2026-10-03: ten controls across four native
binaries, zero skipped, run `f9079739-1441-45c1-9ad7-d0b75a92d7b7`
(`p3-usage-native-repaired2-controls.log`). Exact Invocation support is retained with its run
and native attribution, and source location remains the actual canonical site/event. Earlier
failed batches remain recorded; this is a composite scoped native receipt. The source producer
is frozen for matching-CLI and real-PG original-evidence qualification, which are pending.

Capture PG3 diagnostic **failed**, 194.34s (`p4-capture-pg3-diagnostic.log`), preserving
actual two bindings, two value sources and two witnesses. It confirms the declaration support
is AnalyzerAssertion/NativeTraversal/ReportProjection. The bounded packet repair exposes that
source-correspondence proof with its actual qualification/basis/attribution and explicit lack
of value authority, rather than admitting it through the NativeStructural value-proof helper.
The native capture/timing/origin checks remain unchanged. Affected actual PG rerun is pending.


Frozen P3 usage checkpoint integrated `a862fc29` (executor `ff96c8cc`), 2026-10-03:
C2 SourceUsage anchors, native alias/re-export/rebinding identity, actual argument/receiver/
applicability/association evidence, separate chosen/candidate trace availability and explicitly
potential higher-order channels. Full target qualification/basis survives original-source packet
hydration; unresolved associations remain visible and grants do not expand to foreign/outside
spans. Native composite ten controls passed; matching CLI and actual diagnostic/usage PG remain
pending. This adds required typed schema and no compatibility reader. Aggregate patch SHA-256
`e38bd426f79170975252036a2d95de1602f5c6221b1bb6b7cc38f0dd0daf0100`;
receipt `p3-source-usage-receipt.json`, schema diff SHA-256
`a9d6342c59d2a9ca54b01a3173504bcab7d3c39d69b543522557b5785becfc17`.


B3 PG4 **failed**, 2026-10-03, 362.56s (`b3-pg-4.log`); the unchanged three-frontier
oracle still sees only declared_use. Candidate routing now clears its old approximation refusal
but stops at MissingEvidence. Decisive retained actual rows in `b3-target-refusal-2.txt` show
selected source signature, original Overrides destination and native member Callable.function
have identical symbols. Native metadata/member/receiver evidence is present. The class-trait
assertion has only ReportProjection support (actual fidelity3), while NoExtraOverrides correctly
requires NativeStructural evidence. No policy is weakened and no report is relabelled.

The bounded producer repair will add genuine native ClassTrait support from exact
Bindings KeyClass/BindingClass index and Solutions KeyClassMetadata APIs. Full fields must
match the existing assertion under its qualification/symbol key; only an additional native
support is emitted, retaining report provenance. Missing/ambiguous/disagreeing correspondence
remains an explicit boundary/Partial outcome. Charged native controls will cover ordinary,
dataclass, NamedTuple, TypedDict and functional synthesized classes plus disagreement refusal,
then rerun the unchanged PG oracle. This is an implementation prerequisite of C4/B3's accepted
native NoExtraOverrides contract, not a new admission framework or replacement basis.


Frozen P3 usage matching standalone CLI **passed**, 2026-10-03: `cargo build --locked
--release -p lctx`, 20m05s (`p3-source-usage-explicit-cli-build.log`); actual original-evidence
PG is queued on the unchanged ff96 source baseline. Capture correspondence scoped release
check **passed**, 42.03s (`p4-capture-check5.log`), after a retained move/clone check failure.
Its matching standalone CLI **passed**, configured jobs16/default frontend1, 30m26s including
normal shared-artifact waits (`p4-capture-cli2.log`). Actual capture PG4 and required-field
wire controls remain pending. Native class-support producer check **passed**, 54.29s
(`b3-check-7.log`); native flag/disagreement/provenance controls and unchanged frontier PG5
remain pending. These results do not establish packet or full-series acceptance.


Native ClassTrait support controls **passed**, 2026-10-03, on the frozen B3 executor tree:
`cargo test --release -p cpg-extract --lib native_class_trait_support_requires_identical_unique_report_payload`
and `cargo test --release -p cpg-extract --test native_class_traits -- --nocapture`.
The former checks equality/disagreement refusal; the latter checks six ordinary/record/functional
class forms and actual native/report attribution with one assertion row. Logs
`b3-trait-equality.log`, `b3-native-traits.log` and `b3-final-batch.json` retain commands and
outcomes. Unchanged actual three-frontier PG acceptance and main integration remain pending.

Capture packet PG4 **passed**, 2026-10-03: `cargo +nightly-2026-09-29 test -p lctx
--release --locked --test capture_question_packets -- --nocapture`, one actual disposable-PG
journey, 207.33s after 27m30s build (`p4-capture-pg4.log`). The served entry proof preserves
AnalyzerAssertion/NativeTraversal/ReportProjection source correspondence; capture/timing/origin
value proofs remain native structural. Literal/formal sources, direct frames, qualifications
and characterization are checked. Earlier failures/interruption remain a composite scoped
receipt; required-wire control and main integration remain pending. This is not Q0 acceptance.


The preserved stdio qualification runner and its real both-profile fixture/control tests are
integrated for Q0, 2026-10-03. Scalar inputs now require one complete Source-role formal from
Rust's emitted schema; NativeSlot identities remain outside source-body input mapping. The
runner preserves actual protocol bytes, continuation, generation/source/default/channel checks
and negative fault controls. Earlier scoped Phase 5 receipts retain their original baseline;
current `uv run --no-sync python -m pytest tests/scripts/test_qualify_serving.py -q` failed
collection on the missing installed `lctx_semantics` (`p4-stdio-transcript-controls.log`). Fresh
adapter and both-profile runtime qualification remain pending the final assembled model.


P3 actual original-evidence composite **passed**, 2026-10-03: diagnostics in
`p3-source-characterization-pg-controls.log` (72.365s), then the affected usage control with
`cargo nextest run --release -p lctx --test source_characterization
-E 'test(actual_usage_aliases_targets_overloads_and_locations_reach_original_evidence)'`
(80.530s, one passed/one outside-filter skipped, run
`460f0260-4b73-4724-92f8-76809dff0b20`; `p3-source-usage-pg-repaired2-controls.log`).
The initial two-control run failed only usage because it treated an opaque module assignment
as a resolved import alias. `553d200f` integrates executor `8f16edb2`: direct import/re-export
positives and both module/local opaque-assignment negatives are now independently asserted,
with no production or schema relaxation. Receiver/applicability/chosen/candidate/full-basis,
higher-order Potential and source-grant controls passed in that journey. This closes the
bounded P3 consumer receipt, not assembled Q0 or full-series gates.

P3 final bounded-text control **passed**, 2026-10-03: `cargo nextest run --locked --release
-p lctx-postgres --lib -E 'test(native_source_text_preserves_literal_bytes_and_refuses_indivisible_overflow)'`
(one selected control, three outside-filter; run `b7103db6-2a15-412e-bd8d-8be5fde82f96`,
`p3-source-characterization-text-controls.log`). This scoped receipt does not establish a
fresh native adapter or current-tree assembled qualification.

Raised-type closure correction is integrated at `c8d0bf4b`, 2026-10-03. Independent static
review confirmed the bounded use of existing artifact/context native Types coverage; complete
Signatures coverage alone cannot close omitted native raise traces. Append-only selection
witness27 retains the closure row. `cargo check --release -p lctx-model --tests` **passed**
(43.17s, `f01-raised-types-check-repaired.log`), after the retained initial test-import failure
(`f01-raised-types-check.log`). `cargo check --release -p lctx --test raised_type_selection`
**passed** (25.29s, `f01-raised-types-service-check.log`). `cargo test --release -p lctx-model
--lib raised_coverage_tests -- --nocapture` **passed**, two controls (`f01-raised-types-unit.log`):
partial/missing/foreign-context/foreign-artifact/wrong-family inventories preserve positives and
refuse negatives; a complete set retains the exact closure witness and rejects foreign witness
replay. Actual mixed/complete native inventory serving controls remain pending; actual runtime
needs the final matching CLI.

The F01 classification projection inventory control also **passed**, 2026-10-03:
`cargo test --release -p lctx-model --test domain_selection_catalog
local_preparation_inventory_and_missing_membership_refusal -- --exact --nocapture`
(one selected control, `f01-selection-inventory.log`). Classification declares all 79 required
inputs, including the existing native coverage relation; missing membership stays refused.

Selected-analytics Q0 fixture implementation is integrated at `c79ccf19`/`13e125cd`, 2026-10-03.
The existing both-profile stdio/fault-control journey now selects communities/PageRank/FCA/RCA
and type/mention layers explicitly, with kNN/embeddings off, fixed seeds and an official Markdown
corpus acquired through production code. Its independently specified type/mention pair weights,
source-default/receiver incidence, public universe, method policy and actual receipt lineage
remain assertions, not inferred acceptance. Static review confirmed a newly discovered input
association defect: Installed and Corpus retain separate revisions, so input equality alone
silently discarded official mentions. `32bcc5a6` consumes existing `CorpusLibrary` and permits
only its exact directed link; context, Document role, resolved entity scope and deduplication
remain unchanged. The shared charged mention operation is exposed narrowly for stored-input
controls. Those controls replay actual persisted rows and reject missing/wrong corpus or library
links, without alternate interpretation or fabricated source facts. Targeted checks **passed**:
`cargo check --release -p lctx-model --tests -p cpg-core --test analytic -p lctx
--test serving_qualification` (35.28s, `f02-corpus-mentions-check.log`, before narrow re-export),
then `cargo check --release -p lctx --test serving_qualification` (13.49s, executor session78418,
on `32bcc5a6` plus frozen stored-input controls). Actual both-profile runtime, current adapter
and the final independent review remain **not_run** until the final assembled CLI.


Frozen direct capture output integrated as `16827a31` (executor `c5a26787`), with
`3b8a4a69` sharing the already registered P3 Modality/Approximation schema implementations.
No numeric codes are changed. Required-wire control **passed**, 2026-10-03:
`cargo +nightly-2026-09-29 test -p lctx-model --release --locked --test serving_contracts behavior_requires_explicit_direct_capture_provenance_even_when_empty -- --exact --nocapture`
(one selected control, eighteen outside-filter controls; `p4-capture-wire1.log`), in addition to
the actual PG4 and matching CLI/check receipts above. Exact executor receipt:
`/home/paul/.cache/lctx-code-facts/code-facts-p4-capture-receipt.md`; aggregate patch SHA-256
`91ad5d44deec1bd3f28fa8834d135305ccd1535ab14aadb6d58c2f12015e52ab`.
P3 Evidence fields and source-formal documentation are preserved. Assembled compile, snapshots,
current adapter, Q0 and full gates remain pending; this is an additive hard wire/schema migration.


B3 producer integrated at `603d1a63` (executor `fae167bc`), 2026-10-03. Summary conflict
resolution retains both CaptureWitness12 and TerminalFrontier13, all capture/terminal inputs
and outputs. The scoped composite contains **18 distinct controls passed**; earlier failures
remain in `/home/paul/.cache/lctx-code-facts/b3-receipt.json` and its named logs. Actual
`cargo test --release -p cpg-core --test behavioral_frontiers -- --nocapture` **passed**
(`b3-pg-6.log`, 963.38s total): exactly declared_use/final_use/final_method_use, with
GivenInvocationEntered, full bases, real authored-universe support and unknown effects,
exceptions and cleanup. Original source inspection/open dispatch and contradictory-C3
prefix exclusions survive. Native exit characterization supplies no runtime completion.

`cargo test --release -p lctx-model --test terminal_model_membership` **passed**; native
MRO and protocol/exception identity, context composition and exact handler controls also
**passed**, with exact commands in that receipt. The later actual read regression initially
**failed** because declaration inspection legitimately uses report-projected FunctionTrait,
SymbolDeclaration and ParameterDeclaration. The narrow DeclaredClassInspection purpose accepts
only those three exact native question/support pairs with original fidelity; ordinary
ExecutableRead eligibility remains NativeStructural. Four selector controls **passed**
(`b3-inspection-selectors.log`), then the actual native read inventory **passed** with nine
dynamic observations and two declared-class inspections (`b3-read-regression-2.log`). No
runtime class/allocation/state/member-universe authority follows from inspection.

The receipt is composite: frontier PG6 preceded that inspection-only repair; affected selector
and actual read controls were rerun. Its private Ruff revision `8f01d800` differs from main's
current canonical fork. Current-main terminal, assembled Q0 and full gates must re-exercise
these contracts on the final union. `cargo check -p lctx-model -p cpg-core --tests` **passed**
(`b3-check-9.log`, 55.46s, dev profile); main assembled release check is pending. The additive
Model/Summary relations and append-only claim3/support13 are a schema migration; terminal
packets, snapshots, adapter and full gate acceptance remain open.
