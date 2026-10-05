# A7: structural shape of the declared relation model (static evidence, 2026-10-05)

Lane A7 of the SurrealDB pivot review. This lane establishes facts and gives no verdict.

## Baseline and method

- Tree: `main` at `f66eb15a` (lctx-model clean in `git status`).
- Model: `build/review-probes/surrealdb-native-realization/model-describe.json`, digest
  `0ea88254…c096f`, with 878 relations, 231 invariants and 39 publication checks.
- `describe` omits layer, owner, projection roles and derivation (`crates/lctx/src/model.rs:29-58`).
  To get them, a scratch binary (`scratchpad/a7dump`, isolated target dir, no repo writes) linked
  `lctx-model` and dumped four things:
  - `Frontier::{Facts,Normalized,Analysis,Catalog}.descriptor().relations()`;
  - `Relation::family()`;
  - `projection_roles()`;
  - `derivation()`.

  Its `domain::model()` digest equals the exported JSON's digest, so both describe the same model.
- Layer means the first frontier that contains the relation (`admission.rs:52-91`; `domain/mod.rs:217-236`):
  - L0 is facts: 245 relations;
  - L1 is normalized minus facts: 96;
  - L2 is the analysis frontier minus normalized: 445;
  - L3 is catalog minus analysis: 92.

  L2 and L3 are the cumulative `pre_catalog_analysis_relations` and `analysis_relations`.
- The classifier (`scratchpad/a7classify.py`, output `a7classes.json`) is rule-based. The class
  rules are applied in order:
  1. A blob list is matched by name.
  2. Generated per-owner lifecycle families are the 10 `owner_table!` suffixes × 15 owners
     (`analysis/family/invocation.rs:3`).
  3. Attribution:
     - `*_supports`;
     - L0 `*_observations`;
     - `assertion_qualifications`, `evidence`, `provider_surfaces`, `run_families`;
     - `*_transfer_alternatives`, the qualification wrappers over transfer keys.
  4. Analytics, projection or derived, decided by declaring module: analytics, structural,
     projection, retrieval, embedding.
  5. Lifecycle, by suffix: receipts, admissions, coverage, outcomes, invocations, requirements,
     boundaries, witnesses, premises, inputs, assessments, attempts, obligations, derivations,
     sources, evidence, runs and similar.
  6. Vocabulary or value: value suffixes, no reference, or a tagged reference union (every sum
     arm holds at most one id and no id sits outside the arms).
  7. Everything else is classified by **endpoint arity**. This counts id fields other than
     provenance-flagged ones and other than qualifier or scoping targets:
     - assertion_qualifications, analysis_contexts, coverage_scopes, evidence;
     - provider_runs, surfaces and providers;
     - input_revisions, assumption_sets, conditions, provider_coverage;
     - `*_analysis_invocations/coverage/outcomes`, `*_source_receipts`, `*_runs`.

     Sum arms count only their largest arm. An arity of 1 or less is entity, 2 is binary, 3 or
     more is n-ary.

  An explicit node list of 45 names takes §15.4 vocabulary entities as nodes even when their
  identity is composite: places, access_paths, provider_symbols, type_terms, normalized_call_events,
  evaluation_atoms, condition_nodes, catalog_members and others.

## Classification table

Per layer, with the reference (id) fields each class carries. The model has 3,309 id fields in
total and 5,359 fields.

| # | Class | L0 | L1 | L2 | L3 | Total | id fields (key) | All fields | Derivation-declared |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Entity / node | 43 | 8 | 19 | 11 | **81** | 341 (325) | 599 | 8 |
| 2 | Binary relationship | 18 | 15 | 40 | 10 | **83** | 199 (146) | 299 | 28 |
| 3 | N-ary / hyperedge (≥3 endpoints) | 7 | 20 | 46 | 6 | **79** | 444 (290) | 594 | 34 |
| 4 | Observation/support/attribution | 142 | 0 | 9 | 2 | **153** | 568 (543) | 1,036 | 8 |
| 5 | Vocabulary / value / tagged-union | 29 | 4 | 23 | 3 | **59** | 129 (128) | 309 | 3 |
| 6 | Lifecycle / proof / receipt / coverage | 5 | 43 | 236 | 51 | **335** | 1,287 (1,164) | 1,954 | 138 |
| 7 | Artifact / blob / text-vector bodies | 1 | 1 | 2 | 4 | **8** | 12 (9) | 45 | 0 |
| 8 | Projection / analytics / derived | 0 | 5 | 70 | 5 | **80** | 329 (273) | 523 | 2 |
| 9 | Other | 0 | 0 | 0 | 0 | 0 | — | — | — |
| | **Total** | 245 | 96 | 445 | 92 | 878 | 3,309 | 5,359 | 221 |

### Examples

1. **Entity / node.**
   - L0: occurrences, source_artifacts, modules, places, access_paths, provider_symbols,
     type_terms, condition_nodes, evaluation_atoms.
   - L1: callable_entities, class_entities, parameter_entities, normalized_call_events.
   - L2: catalog_callables, catalog_classes, catalog_members.
   - L3: findings, retrieval_units, synthesis_briefs.
2. **Binary relationship.**
   - L0: call_arguments, declaration_decorators, syntax_placements, type_sequence_members,
     symbol_declarations.
   - L1: field_entity_links, place_entity_links, symbol_entity_resolutions,
     effective_decorator_members.
   - L2: catalog_exposures, catalog_document_associations, `*_members` of execution and summary.
   - L3: finding_members, synthesis_brief_assertions.
3. **N-ary / hyperedge.**
   - L0: call_resolutions, flow_call_steps.
   - L1: normalized_call_alternatives, occurrence_ownership, call_bindings,
     import_module_candidates, reference_entity_candidates, public_exposure_candidates.
   - L2: `local/model/summary_transfer_keys`, local_control_influences, guard_substitutions,
     catalog_paths, context_entry_bindings.
   - L3: synthesis_selected_seeds.
4. **Observation / support / attribution.** It holds 63 L0 `*_observations` and 83 `*_supports`,
   75 of which share one identical 7-field shape: assertion, run, surface, evidence, origin,
   mode and fidelity. The rest are `assertion_qualifications`, `evidence`, `provider_surfaces`,
   `run_families` and the 3 `*_transfer_alternatives`.
5. **Vocabulary / value / tagged union.**
   - Values and headers: literal_values, literal_sets, record_options, call_origins, type_sequences.
   - Configuration: analysis_definitions, embedding_specifications.
   - Tagged reference unions (18): entity_refs, coverage_scopes, call_destinations, lexical_targets,
     `*_obligation_subjects`, signature_type_subjects.
6. **Lifecycle / proof.**
   - 150 generated owner families: `{analytic, analytic_embedding, base_completion, base_evaluation,
     catalog_core, catalog_evidence, enriched_execution, local, model, retrieval, selection,
     source_call, structural, summary, synthesis}` × `{analysis_invocations, analysis_inputs,
     analysis_outcomes, analysis_coverage_premises, coverage_requirements, coverage_sources,
     coverage_required_sources, invocation_sources, projection_inputs, source_receipts}`.
   - Others by suffix: 35 assessments, 30 sources (premise sums), 20 coverage, 15 boundaries,
     15 premises, 12 witnesses, 11 `*_evidence`, 7 runs, 6 outcomes, 6 invocations, and so on.
7. **Blob.** artifact_chunks (1 MiB bytes), projection_snapshot_chunks (Postcard petgraph BYTEA),
   embedding uses (vector bytes), retrieval_corpus_texts, retrieval_fragments,
   analytic_text_windows, synthesis_brief_documents.
8. **Projection / analytics / derived.**
   - Projection: the projection_* stores.
   - Analytics: neighbours, communities, concepts, rank scores, `analytic_graph_arcs`.
   - 30 structural traversal outputs: paths, reaches, steps, frames.
   - Retrieval subjects and anchors.

## Edge-shaped relations: identity and discriminators (CI-03)

Classes 2 and 3 together hold 162 relations.

| Shape | Binary | N-ary | Meaning |
|---|---|---|---|
| Pure endpoint-set key (no other key field) | 33 | 16 | Set-of-pairs or tuples; no parallel arcs possible |
| Extra key discriminator (role, phase, ordinal, qualification, kind, context…) | 14 | 20 | Edge with its own identity; parallel arcs distinguished |
| Functional (at least one endpoint non-key, i.e. an attribute of the keyed side) | 36 | 43 | One target per keyed subject (or per subject × invocation/ordinal) |

Examples of each shape:

- **Discriminated.** `syntax_placements{qualification,field,ordinal}`,
  `declaration_decorators{qualification,ordinal}`, `call_event_phase_targets{phase}`,
  `flow_call_steps{ordinal,role}`.
- **Transfer keys.** The transfer keys are discriminated too:
  `local_transfer_keys(owner, input→places, output→places, context, scope, modality, approximation,
  kind, call_site?, provenance)`. All are key fields, so parallel transfers between the same two
  places stay distinct. Each alternative qualification is a separate `*_transfer_alternatives`
  row (§15.6, `semantic-model.md:486-497,556-558`).
- **Functional.**
  - `occurrence_ownership` is keyed only by `occurrence`; `owner` and `entity` are payload.
  - `normalized_call_events.owner` and `call_bindings(attempt, ordinal) → slot/source` are also
    functional.
  - Many L2 execution rows are keyed `(subject, invocation)` with payload pointers.

Every relation has a generated typed content key hashed with BLAKE3 (§15.3,
`semantic-model.md:161-163`). So every edge row already has its own identity. CI-03 parallel-arc
separation is carried by key discriminators (§15.3, `semantic-model.md:200-201`: "Parallel
relationships include the discriminator that separates them").

## Cycles, self-references and hubs

- **Self-references.** There are 15 self-referencing id fields in 3 relations:
  - `condition_nodes.branch_low/high`, the BDD DAG;
  - `type_terms`, with 12 arm fields such as callable_returns, generic_body and typealias_target,
    making a structural type DAG;
  - `summary_path_routes.escapingraise_route`.

  All are content-addressed, so they are acyclic by construction (Merkle ids, §15.7
  `semantic-model.md:615-616`).
- **Cycles.** The relation-level reference graph has 4 non-trivial SCCs:
  - `{evaluation_atoms, predicates}`;
  - `{catalog_candidates, catalog_paths}`;
  - `{local_atom_restrictions, local_support_sources, local_transfer_supports}`;
  - `{summary_path_witnesses, summary_transfer_premises, summary_transfer_witnesses}`.

  The other 870 relations sit in a reference DAG.
- **Collections.** There are no list-typed id fields: 0 of 3,309. Reference collections are always
  member relations (§15.2, `semantic-model.md:145-146`). 1,239 id fields are nullable, mostly
  inactive sum arms; 138 relations are sums and 28 id fields carry subtype constraints.
- **Hubs**, counted as distinct referencing relations:

  | Relation | Referencing relations | Class |
  |---|---|---|
  | assertion_qualifications | 157 | 4 |
  | occurrences | 129 | 1 |
  | entity_refs | 105 | 5, the polymorphic entity union |
  | evidence | 87 | 4 |
  | analysis_contexts | 81 | 1 |
  | provider_runs | 79 | 6 |
  | provider_surfaces | 75 | 4 |
  | coverage_scopes | 51 | 5 |
  | local_analysis_invocations | 37 | 6 |
  | input_revisions | 36 | 1 |
  | provider_symbols | 35 | 1 |
  | summary_analysis_invocations | 30 | 6 |
  | provider_coverage | 29 | 6 |
  | native_analysis_premises | 28 | 6 |
  | normalized_call_events | 24 | 1 |
  | source_artifacts | 23 | 1 |

  53 relations are referenced by 10 or more others, 143 by 5 or more, and 269 by none.
- **Qualifier references.** 1,097 of 3,309 id fields (33%) point at qualifier, scoping or
  lifecycle targets as defined above. 168 point at `assertion_qualifications` alone, from 157
  relations: 76 L0, 4 L1, 68 L2, 9 L3.

## Mapping to graph-native storage (structural facts, no verdict)

**Natural RELATE edges.** These are binary relations with both endpoints keyed and at most a few
discriminators: 33 pure-set plus 14 discriminated, so 47 at most. Most are membership, link or
association tables:

- `*_members`;
- `field_entity_links`, `place_entity_links`, `type_entity_links`;
- `catalog_document_associations`, `catalog_exposures`.

**Record links rather than edges.** These are the 36 binary and 43 n-ary functional relations.
The non-key endpoint is a function of the keyed side, so it maps to a `record<…>` field on the
keyed record, for example `occurrence_ownership.owner/entity` or `normalized_call_events.owner`.

**Edge-with-identity tables in any store.** These are the 79 n-ary relations, the discriminated
binaries, and transfers (one key with 10 fields, 4 of them references, plus alternative and
support rows). An n-ary hyperedge has no single in→out pair. A graph-native store would need
either:

- an edge record with extra reference fields, which is how `gen_schema.py` already treats
  everything; or
- reification as a node.

The design's chosen transfer, binding and call-alternative shapes are of this kind (§15.5
`semantic-model.md:427-431`: bindings keyed by site × alternative × variant × actual → formal).

**Plain tables in any store.** This covers classes 4, 5, 6 and 7 (555 of 878 relations, 63%;
3,344 of 5,359 fields, 62%) plus most of class 8. Their references point at the edge or node they
qualify, not between domain nodes. One example: a `*_supports` row points at its observation
plus run, surface and evidence.

**Prior worker's mapping.** `gen_schema.py` emits only `TYPE NORMAL` tables, 1,199 of them: 878
identity tables plus 321 `__p` payload tables. References become `record<…> REFERENCE` fields.
`schema.surql` has 0 `TYPE RELATION` tables and no RELATE (`gen_schema.py:107-116`).

## Where the model already defines the graph view (§15.10)

- **Projections.** The model declares 4 projections, each over typed endpoint roles
  (`projection.rs:47-57, 127-139`):
  - CallableInvocation;
  - DefinitionContainment;
  - ImportReference;
  - PublicExposure.
- **Arc sources.** The 7 endpoint roles draw arcs from exactly 5 relations (`EndpointRole::relation`,
  `projection.rs:28-36`; confirmed by `projection_roles()` in the dump):

  | Arc source | Roles |
  |---|---|
  | `normalized_call_alternatives` | Invocation, Definition |
  | `occurrence_ownership` | Containment, SourceDefinition |
  | `import_module_candidates` | Import |
  | `reference_entity_candidates` | Reference |
  | `public_exposure_candidates` | PublicExposure |

  Arc identity is the typed source-row id (`ArcId`, `projection.rs:~150-160`).
- **Arc endpoints are computed, not stored columns of the arc-source row.**
  - Import arcs take their source from the assessment's observation alias occurrence. Their target
    comes only when the candidate's provider module is the `Acquired` arm; other cases become a
    gap such as `ExternalModule` or `OutsideUniverse` (`projection/normalization.rs:582-608`).
  - Containment and SourceDefinition use the ownership row plus a definitions index
    (`normalization.rs:556-577`).
  - Policy (`CallPolicy::Invocation`), qualification, universe and coverage filters apply first.
- **Universe.** The vertex universe is `entity_refs` filtered by `ProjectionSpec::accepts`
  (module, callable, class; occurrences only for containment and import; parameters and fields
  only for import and exposure). It is built before arc selection and keeps isolates and
  resolved foreign destinations (`normalization.rs:355-445`; `projection.rs:179-194`). The fixed
  policies are `InputEntitiesAndContextTargets`, `PreserveTypedParallelArcs` and
  `RetainEveryRequiredScope` (`projection.rs:58-69`).
- **Unresolved side records.** These are persisted relations: `projection_gaps` (11 reasons) plus
  `projection_gap_subjects` (an 8-arm sum over events, alternatives, assessments, exposures and
  coverage). With them are `projection_source_assessments` (vertex, arc and gap counts) and
  `projection_source_coverage`.
- **The graph itself is persisted only as Postcard petgraph bytes.** These live in
  `projection_snapshots` plus `projection_snapshot_chunks` (§15.10 `semantic-model.md:732-741`:
  "No separate graph schema or materialized path closure becomes semantic authority").
  `analytic_graph_arcs` (frame, alternative, source, target) is the one per-analysis persisted
  arc list.
- **Other graphs, kept separate by design** (§15.1 `semantic-model.md:111-114`):
  - The derivation index is 221 derivation-declared relations, 762 premise columns and 84
    conclusion columns, with generated `derivations`/`derivation_premises` views (§15.9
    `semantic-model.md:673-683`). This is a provenance DAG, mostly over class 6: 138 of the 221.
  - The stage table.
- **Implicit graphs** that are not declared projections:
  - transfers (place → place, §15.6);
  - control influences (place → atom);
  - the BDD node DAG and the type-term DAG;
  - class ancestry members;
  - the summary SCC schedule, which uses the invocation projection's `kosaraju_scc`
    (`semantic-model.md:727-729`).

## Graph-relevant versus bookkeeping (counts)

| Category | Classes | Count |
|---|---|---|
| Domain-graph core | 1 + 2 + 3 | 243 relations (28%), 1,492 fields (28%) |
| Graph-native-edge candidates | — | ≤47 binary (5%) |
| Declared projection arc sources | — | 5 relations (0.6%) |
| Analytics / derived outputs | 8 | 80 (9%) |
| Attribution, provenance, lifecycle, vocabulary and blob bookkeeping | 4 + 5 + 6 + 7 | 555 (63%), 3,344 fields (62%) |

The domain-graph core, classes 1–3, is itself partly reified proof structure: 70 of its 243
relations are derivation-declared.

## Uncertainties and limits

- **The classification is heuristic.** Boundaries move by tens with reasonable rule changes
  (the first pass, below, is the only sensitivity point measured).
  - Name-suffix lifecycle rules put L1 `*_assessments` (35, for example
    `effective_callable_assessments`, a total outcome per entity) and `*_sources` premise sums
    (30) into class 6. Some readers would call these semantic.
  - The 45-name node override and the qualifier-target list (references to `analysis_contexts`
    and `input_revisions` do not count as endpoints) are judgments. A first pass without them
    gave 27 entities, 93 binary and 107 n-ary.
  - Catalog, synthesis and selection product relations (L2/L3) were classified by shape, not
    as class 8.
- **Endpoint arity counts only the largest sum arm.** Polymorphic arms such as
  `callable_aspect_sources` are therefore under-counted relative to the physical field count.
- **"Functional" assumes key/non-key implies a functional dependency.** That follows from
  `Same-key conflicting payload is an error` (§15.2 `semantic-model.md:148`), but a model merge
  owner can alter it.
- **Layer is the frontier contract** (ADR-0101 descriptor), not the §15.1 table's prose. The
  frontier was taken under the current tree; the describe JSON's `facts` flag uses
  `Profile::Catalog`.
- **Row-volume weighting was not measured.** These are declaration counts, not row counts, and
  no generation or store was read. The running compile was not touched.
- **The relation `owner` and `contract` strings are private** (`model.rs:17-18`, no getters).
  Owners were not used.
