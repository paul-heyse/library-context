# Stage 3 condition kernel and semantic serving — standard design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Proposed [ADR-0024](../../adr/0024-condition-kernel.md), [ADR-0025](../../adr/0025-bounded-semantic-serving.md), DESIGN §§B10/B13/B14/3.9/9.9/11.3 and the Stage 3 forward plan; current fact and serving contracts read as feasibility evidence |
| Standard | Core 2.0, code-intelligence profile 1.0, `library-context` binding in `standard.toml` |
| Tier · purpose | Design · target (proposed §B pivots) |
| Reviewer · date | Independent design-review agent · 2026-09-24 |
| Decision | **Accept the corrected proposed ADR pair**; Stage 3 implementation remains unverified (final disposition in §12) |

**Outcome sought.** Lossless, bounded Boolean conditions and meaningful user-scenario queries over a single pinned serving generation, without giving Python a second condition evaluator.

**Baseline.** Stage 2 persists capped DNF (`conditions`, `condition_literals`) with per-site evaluation identities; the 2026-09-24 pilot recorded 66 `budget_reached` claims. Stage 1 `find_operations` exhaustively scans materialized facets and distinguishes matches, hidden possible matches and completeness. The current server is a pure-Python `uv_build` package over bundle format 5.

**Supported scope and non-goals.** The decisions claim propositional implication/compatibility, typed exclusion only for proven stable primitive places, bounded on-demand semantic filtering, one generation, and same-generation evidence. They exclude general SMT, arbitrary Python expressions, network/Delta/compiler access in the executor, and a hard wall-clock guarantee.

**Method and coverage.** Read both proposed ADRs, the named DESIGN and plan sections, current schemas and translator, bundle reader, package manifests and `find_operations`. Inspected pinned `biodivine-lib-bdd` 0.6.3 API/source for fixed variable order, transfer and bounded binary apply; inspected [PyO3 0.29.2](https://docs.rs/pyo3/0.29.2/pyo3/) and [maturin's mixed-project layout](https://www.maturin.rs/project_layout). No new product tests or pilot were run for this document-stage review; the author is running the Stage 3.0 spike concurrently. Query-time effect/role/witness implementations do not yet exist, so their exact runtime behavior is unverified. Claims below are **Proposed**, except the existing code and library-interface observations explicitly marked **Implemented** or **Interface-checked**. F01–F04 record defects in the first proposal read; the corrected-proposal disposition at the end is the current gate assessment.

## 2. Authority and identity map

| Fact or concept | Semantic type and identity | Authority / owner | Revision boundary | Update path | Derived representations |
|---|---|---|---|---|---|
| Evaluation atom | Test meaning, value place, source evaluation site or provider predicate identity | `cpg-flow` translator and `cpg-schema::condition` | Source/module digest and analyzer/model revision | Re-extract release | Encoded atom, display label |
| Condition function | Boolean function over an intrinsic ordered support of atom IDs; structural root/node IDs | Proposed Rust kernel plus `cpg-schema` flow contract | Condition encoding/kernel version and snapshot | Recompile and schema migration | Bounded DNF display, serving projection |
| Type observation and stability witness | Pyrefly's attributed observation at a use; separate proof linking value identities across effects | Pyrefly extraction plus proposed typed-theory bridge | Analyzer revision, source site and effect model | Re-extract; preserve proof provenance | Theory clauses; compatibility verdict |
| User scenario | Constraint on an operation entry value, with formal/place identity | **Not yet declared** (F01) | Query and operation identity | Per request | Query BDD and filter result |
| Serving generation | Immutable manifest-keyed projection of one Delta snapshot | `cpg-core` bundle writer and server startup validator | Bundle format, schema/kernel version, snapshot | Rebuild from published snapshot | PyArrow lookup tables; proposed native projection |
| Query result | Match, open/unknown, excluded under model, truncated and cursor | Proposed Rust executor; FastMCP transport | Pinned generation + canonical query + budgets | Per request | MCP object |

**Fact and fidelity table (CI).**

| Family or relation | Provider/revision | Fidelity | Coverage and unknowns | Identity | Consumers |
|---|---|---|---|---|---|
| `flow_tests`, `conditions`, `condition_literals` | ty through `cpg-flow`; pinned source | Native/normalized structural; DNF is currently budgeted | Module flow coverage; `budget_reached` is unknown | Snapshot, source site, condition ID | Stage 2 behaviors; proposed BDD migration |
| `type_observations` | Pyrefly fork | Pyrefly type assertion, not runtime observation | Present roles are parameter, return, call result, argument, raised; **no arbitrary test-use role** | Subject node, role, fact ID | Proposed typed theory needs a new exact-use bridge (F02) |
| Effect-stability witness | Proposed specialized analysis | Derived proof under stated effect model | Missing proof must preserve independent atoms | Sites, place and path/region | Typed theory and query alignment |
| Semantic query evidence | Proposed bundle rows and BDD nodes | Derived over one generation | Budget and incomplete source facts must remain open | Generation, row/node IDs | `find_operations`, `get_operation` |

**Opaque behavior.** Python `__eq__`, descriptor/property reads, global and closure mutation, user classes and dynamic dispatch are outside any automatic scalar-equality theory. Their tests remain site-specific unless a proof covers the value and operator at both evaluations. The existing Stage 2 may-model also admits ty's ambiguous terminal and specified runtime assumptions.

**Identity behavior.** Stable condition IDs must be built from intrinsic atom identities and reduced structural nodes, never library-local variable indices. A comparison's union vocabulary is an ephemeral execution context. Source or kernel-format revisions require new IDs and a bundle migration; a renamed label must not silently preserve meaning.

## 3. Contracts and invariants

| Invariant or contract | Enforcement point | Failure behavior | Evidence |
|---|---|---|---|
| Each BDD node has a known atom, ordered support, two valid children, no cycle, and is reduced/canonical | Proposed `cpg-schema` validator and bundle loader | Reject invalid native generation; never evaluate it | **Gap F03:** ADR-0024 says “lossless diagram-node relation” but does not declare these rejection rules |
| A cross-site relation proves the same runtime value under the effect model | Typed-theory proof producer and checker | Independent atoms; no negative conclusion | **Proposed** ADR-0024 Decision; proof schema/consumer missing (F02) |
| Preflight work and result-node caps cover every kernel operation | Kernel adapter before variable-set creation, transfer, apply, rendering | Typed preflight refusal or node-limit hit; no false verdict | **Proposed**, amended ADR-0024 Decision; pinned API inspected |
| `given` only factors after bounded equivalence check | Rust kernel | Return original condition with `not_factored`, preserving truth | **Proposed** ADR-0024 Decision |
| A query scenario refers to an operation entry value and can be transferred to source sites only by proven relation | Query parser, summary/flow mapping | Unknown where alignment is unproved | **Gap F01:** `compatible_with` lacks this identity contract |
| One generation and kernel format per server | Manifest/schema/kernel handshake at startup | Refuse load | **Proposed** ADR-0025; bundle hash/schema checks are **Implemented**; native build/format route lacks detail (F04) |
| A budget-stopped scan never claims complete or negative for unvisited candidates | Query executor and `OperationSet` construction | `complete=false`, `truncated=true`, explicit unvisited/open count and cursor | **Proposed**; operation partition semantics are not specified across row/depth budgets (F03) |

**Absence and outcomes.** The design distinguishes false/refuted under model from unknown and `not_analyzed`, and distinguishes an operationally truncated page from a complete universe. The new executor still needs a per-operation partition of match, excluded, source-unknown and unexamined, so an operational limit cannot be interpreted as absent. Invalid query syntax and invalid native generations should reject, not return unknown.

**Equivalence.** The BDD is intended to preserve the Stage 2 propositional function over its per-site atoms, not to establish concrete Python executions. Typed theory may only strengthen contradictions justified by its explicit runtime model; bounded display is intentionally non-equivalent as a complete enumeration. “Compatible” needs a declared may-model meaning (F01).

## 4. Derivation and execution

| Stage | Semantic output and equality | Mechanism | Inputs and dependencies | Structural vs value | Reuse boundary | Termination / exactness / determinism | Effects, ownership, publication | Expected size and cost |
|---|---|---|---|---|---|---|---|---|
| Lower ty diagrams | Per-site Boolean conditions | `cpg-flow` translator | ty predicates, source sites, runtime bindings, coverage | Diagram topology plus literal values | Source/module and analyzer version | Conservative may-lowering; Stage 2 DNF cap currently loses claims | Pure extraction into fact run | Pilot 13,776 conditions; 66 budget hits on corrected Stage 2 |
| Canonicalize/persist BDD | Function over intrinsic support; structural ID | `biodivine-lib-bdd` + specialized serializer | Atom IDs, support, child nodes, version | Both | Snapshot and condition-format version | Fixed order; bounded nodes; shuffled-input determinism required | Flow facts in Delta, later immutable bundle | Spike/pilot measurement pending |
| Add typed exclusions | Theory constraints for one stable primitive value | Specialized proof and BDD conjunction | Exact-use Pyrefly type, source-site value/effect proof, operator semantics | Both | Type/effect model revision | Conservative; missing proof leaves unknown | Facts/proof rows required in same snapshot | Cardinality pending; F02 |
| Compose summaries | Condition-bearing flow/effect result | SCC fixed point + BDD kernel | Calls, handlers, effects, negative premises | Both | Full model and graph revision | Bounded k and node limit; widen to unknown | New summary facts published as one snapshot | Stage 3 implementation pending |
| Build semantic projection | Snapshot-bound rows and BDD nodes | `cpg-core` bundle | Published snapshot, schemas, node closure, kernel version | Structural | Generation manifest digest | Deterministic, rebuildable | Immutable generation publication | Bundle size pending; F03/F04 |
| Execute `find_operations` | Match/open/excluded partition and witnesses | Proposed native Rust/PyO3 executor | Canonical query, entry-value mapping, source coverage, generation and budgets | Both | Per request; cursor binds query/generation/order | Bounded rows/nodes/depth; partial never complete | Read-only single-generation process | Pilot query latency/memory pending; F01/F03 |

**CI analysis record.**

| Question | Projection: universe, selector, relations, direction, multiplicity, scope | Method/settings | Exactness/model | Budgets/partial | Output/evidence |
|---|---|---|---|---|---|
| Which branch/fate condition holds? | All relevant flow predicates and per-site atoms for one operation; source tests keep separate identities | Ordered reduced BDD; ty ambiguous admitted | Exact propositional function over conservative may-model; typed relations only with proof | Node/work cap → unknown; display path cap → truncated text | Condition/root/node IDs, test spans and coverage |
| Does a user scenario admit a fate? | All operation candidates; query entry formals → source sites along summary paths; multiple sites/paths preserved | Scenario conjunction and bounded compatibility | Currently **undefined** alignment (F01); cannot equate spelling | Unproved mapping → unknown, operational cap → unexamined | Same-generation fate, proof and condition IDs |
| Which operations match a filter? | Whole library operation universe, then output selector; effect/role facts and completeness status | Deterministic scan and cursor | Match only supported verdicts; open where source incomplete | Budget stop keeps unvisited open and `complete=false` | Operation IDs, row IDs, verdicts, boundary reasons |

**Relationship structures.** BDD edges are low/high decisions on a named atom, not call/dataflow edges. Effect-stability witnesses are path-sensitive relationships between two evaluations and must carry their own identity and scope. Summary call arcs preserve modality; witness traversal is over typed evidence relationships, not an untyped union.

**Boundaries.** Delta facts → immutable bundle requires a versioned flow schema and full BDD node closure. Bundle → PyO3 must validate format, atom IDs and node references before evaluation. PyO3 → FastMCP must preserve verdict, boundary reasons, budgets, truncation and evidence IDs; Python may format but must not recompute truth. Current `generation.py` mirrors Rust schemas and validates manifest hashes, row counts and schema digests; both sides need coordinated format migration.

## 5. Journeys

| Journey | Trace and result |
|---|---|
| Add a fact family or stable-place rule | A new proof relation needs owner, source-site/value/effect evidence, schema/codebook if needed, validator, bundle projection and one Rust consumer. ADR-0024 currently names the proof but not its contract (F02). |
| Change analyzer or code release | New Pyrefly observations or source spans produce new snapshot/condition IDs; old generation remains pinned. A type narrowing is never silently promoted to runtime identity. |
| Boundary round trip | Compile a two-site condition, persist nodes, build bundle, load native module, compare and cite it. Without explicit node validation and ABI/format route, a malformed or older diagram can enter evaluation (F03/F04). |
| Unresolved references/effects | Missing type observation, dynamic alias or intervening effect leaves atoms independent. A zero-match query remains incomplete where a candidate might match. |
| Trace a served claim | Query condition → entry formal → stability/proof relation → source-site atom → BDD node → fate row → span, all in one generation. The first arrow is absent from current `compatible_with` design (F01). |
| Interruption and retry | A kernel preflight refusal yields unknown; `given` returns original with `not_factored`; a row-scan stop yields a resumable deterministic cursor and incomplete result. The last partition needs a concrete rule (F03). |
| Evaluation | Registered Q09 must be tested against the new semantics without feeding gold into compilation. A correct answer is not inferred from a search rank. |

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | **Unresolved** | BDD authority is well assigned, but entry-scenario identity and stability-proof ownership are not (F01/F02) | Define both identities and one proof owner |
| G2 Semantic fidelity | **Fail** | Independent source atoms and unanchored user atoms make Q09 compatibility vacuous or invite spelling-based unsound merging (F01); static type claim overstates current observations (F02) | Specify may verdict, alignment and typed proof |
| G3 Validity | **Unresolved** | Lossless persisted node graph has no declared structural validator; format check alone does not reject invalid topology (F03) | Declare and enforce diagram invariants |
| G4 Hidden behavior | Pass, design scope | Native executor is read-only, typed and excludes arbitrary code/network/Delta; current query client remains separate | Verify import/startup stdout and extension effects |
| G5 Consistency and recovery | **Unresolved** | Single-generation publication is sound in baseline; budget-stop partition and cursor semantics for new semantic scan remain underspecified (F03) | Make unvisited candidates explicit/open |
| G6 Transformation and reuse | **Unresolved** | Intrinsic-support structural IDs and `given` fallback protect rewrites; type/stability theory transfer and bundle round trip lack a checked contract (F02/F03) | Round-trip and proof-preservation checks |
| G7 Truthful capability claims | **Unresolved** | Proposed labels are honest and no hard-time promise remains; native package/ABI route and Q09 semantics need completion (F01/F04) | Specify/build integration path |
| G8 Library leverage | Pass | `biodivine-lib-bdd` owns generic Boolean operations, PyO3 the bridge, FastMCP transport, Arrow/bundle storage; bespoke code is domain proof and adapter work | Keep adapter narrow |
| CI-G1 Fidelity | **Fail** | Q09 may overclaim a compatible fate; type inference can be consumed as runtime proof without a declared assumption (F01/F02) | Preserve model/unknown labels |
| CI-G2 Evidence closure | **Unresolved** | Same-generation row/node IDs are promised, but query scenario-to-site proof and native node closure are missing (F01/F03) | Require cited proof chain and node validation |
| CI-G3 Evaluation integrity | Pass, design scope | Existing registered Stage 3 questions are separate from compiler inputs; no proposed path from gold to kernel/query parameters was found | Keep criterion fixed before evaluation |

## 7. Findings and principle verdicts

The finding table and first gate assessment document the initial proposal. The ADR and DESIGN amendments made in response are assessed separately at the end of §12, so the counterexamples and their required checks remain visible.

| ID | Finding | Principles · gate | Evidence or gap | Consequence | Correction | Verification |
|---|---|---|---|---|---|---|
| **F01** | The query condition has no value identity tied to an operation entry or a source evaluation site | DP-02/04/08/11, CI-04/06/11 · G1/G2/CI-G1/CI-G2 | DESIGN §11.3 `compatible_with: <typed condition>` (`:3400–3404`) and ADR-0025 Decision name canonical syntax but no entry formal, transfer, time or stability relation. ADR-0024 keeps different source sites independent (`:36–42`). | A user asks for `transport == 'sse'` while a fate is guarded by `transport == 'http'`. A fresh query atom makes `a & b` satisfiable; spelling-based aliasing can instead discard a feasible path after mutation. Either answer misstates Q09. | Define typed scenario as constraint on a named operation entry value/formal; explicitly map that value through summaries/effects to each test site with a cited witness. If no safe mapping exists, return unknown. State whether compatible means only “not refuted under this may-model” and keep it distinct from an observed execution. | Two-site mutation and stable-parameter shapes; opposite literals must be incompatible only with a valid witness, otherwise unknown. |
| **F02** | The typed exclusion proof assumes an observation at the test use that current facts do not provide, and treats a static type as a runtime proof without a stated trust rule | DP-02/03/08/11, CI-01/02/06 · G2/G3/G6/CI-G1 | ADR-0024 `:60–64`; `TypeObservationsRow` has only parameter/return/call/argument/raised roles (`tables.rs:1040–1057`, `codebook.rs:761–775`), whereas `flow_tests` has span and condition only (`tables.rs:861–880`). No exact-use type/proof relation is specified. | A parameter annotation or unrelated argument type can be mistaken for the value tested after reassignment; excluding `x == 1 & x == 2` can erase feasible behavior if `x` changes or `__eq__` is user-defined. | Add an attributed exact-use type observation or checked join and a typed stability proof with source/effect scope and enforcement. Define the accepted type-term predicate and runtime-model assumption; reject/leave unknown Any, Unknown, unions containing non-builtin classes, subclass/custom equality, and intervening writes/effects. | Counterexamples for reassignment, closure/global mutation, custom `__eq__`, mixed union, Any/Unknown; proof facts and source spans survive bundle round trip. |
| **F03** | The new native condition projection and budget-stopped operation scan lack validity and partition contracts | DP-03/08/19/20/24, CI-04/08/11/13 · G3/G5/G6/CI-G2 | ADR-0024 `:56–59` requires a lossless node relation but gives no node schema/closure/acyclic/reduction validator. ADR-0025 `:38–43` says unknown/truncated on budgets but does not specify how unvisited operation candidates populate `unknown_total` or a resume cursor; current Stage 1 implementation scans all candidates before computing `complete` (`operations.py:465–495`). | A malformed or missing node can be evaluated as if valid, or a budget hit after a partial scan can return zero matches while silently excluding unseen operations or reporting `complete=true`. | Define node/root/support/terminal ID and edge invariants in authoritative schema with shared publication/startup validation; define match/excluded/source-open/unexamined partitions, exact `complete`/`total` semantics and deterministic cursor across row/depth/node limits. | Corrupt-node load must fail; early-stop fixture with a matching last operation must report incomplete and resume to the same exhaustive result. |
| **F04** | The selected PyO3 serving route has no build/import/ABI contract in the actual workspace | DP-03/15/19/24, CI-13 · G3/G7 | ADR-0025 `:43–47` says extension is “built and pinned with the uv workspace and Cargo lock”; `python/lctx_mcp/pyproject.toml:20–22` uses `uv_build`, and root `pyproject.toml:10–14` has only that member. No native package/member/backend/module path or CPython 3.14 ABI target is chosen. | A developer can complete the Rust kernel yet `uv run` or an installed server loads no extension, an older extension, or one unable to read the new generation. | Choose mixed maturin package or separate native workspace member; pin PyO3/maturin and Python ABI, set import name and shared-kernel crate boundary, reject kernel/generation-format mismatch at startup, and test built wheel plus editable `uv run` loading. | Build/install into clean environment; load one matching and one incompatible generation; run FastMCP tool through actual extension. |

**Applicable principle verdicts.**

| Verdict | Principles | Reason |
|---|---|---|
| **Violated** | DP-02, DP-08, DP-11, CI-02, CI-04, CI-06 | F01/F02 allow an answer beyond its value identity, type evidence or may-model |
| **Unresolved** | DP-01, DP-03, DP-04, DP-05, DP-07, DP-09, DP-15, DP-19, DP-20, DP-21, DP-23, DP-24, CI-01, CI-03, CI-05, CI-08, CI-11, CI-13 | Proof/entry identity, persisted graph validity, partial scan and native package obligations need specified owners/checks (F01–F04) |
| **Satisfied in proposed scope** | DP-06, DP-10, DP-12, DP-13, DP-14, DP-16, DP-17, DP-18, DP-22, CI-07, CI-09, CI-10, CI-12 | One Rust condition authority, library-owned generic operations, bounded work policy, immutable serving projection and honestly Proposed claims; existing hermetic/evaluation rules continue |

**Applicability.** All core pillars bear on a new kernel, schema, native executor and publication route. CI-05 concerns the BDD, proof and witness projections; CI-09 concerns whether ranked retrieval is promoted into evidence (the proposal keeps it separate). This review does not assess unrelated graph analytics or embedding quality.

**Strengths that carry weight.** Per-site atoms preserve Stage 2's mutation counterexamples. Intrinsic-support IDs avoid variable-set-dependent persisted identity. The amended preflight product/support caps, library result-node limit, bounded lazy rendering and `given` equivalence/fallback make budget loss explicit. A single Rust kernel across compile and serve avoids duplicate Boolean semantics; one manifest-pinned generation prevents cross-snapshot row/node mixing if validation is completed.

## 8. Library-leverage ledger

| Capability | Bespoke code | Candidate libraries or built-ins | Fit and gaps | Recommendation |
|---|---|---|---|---|
| Boolean function construction, apply and comparison | Proposed Rust kernel wrapper | `biodivine-lib-bdd` 0.6.3; OxiDD as trigger alternative | Pinned biodivine has fixed-order variable sets, transfer and bounded binary apply; it does not own this domain's atom/proof meaning or persisted IDs. Stress probe exceeded ten minutes without policy. | Adopt biodivine behind a narrow adapter; enforce preflight and node caps, measure pilot; revisit OxiDD on stated trigger. |
| Native Python bridge/build | Proposed PyO3 adapter | PyO3 + maturin mixed project or separate native member | PyO3 supplies in-process extension; maturin supplies established build route. Current `uv_build` member has no selected integration. | Specify one packaging route (F04), keep query semantics in shared Rust crate. |
| Query transport and direct retrieval | Existing Python FastMCP server | FastMCP, PyArrow, existing indexed dictionaries | Already owns transport, typed responses and lookups; no need to move them to Rust. | Retain; use native module for bounded semantics only. |
| Snapshot and table validation | `cpg-schema`/`cpg-core` + mirrored Python schema | Arrow/Delta/manifest checks | Existing digests cover bytes/schema, not BDD graph invariants. | Add one shared semantic validator to publication/native load, not a test-only copy. |

## 9. Alternatives

| Alternative | Meaning duplicated / extension locality | Bespoke code carried | Correctness and operational risk | Cost / performance evidence | Selected or rejected, and why |
|---|---|---|---|---|---|
| Current DNF/materialized lookup | One evaluator; no query-time semantics | Existing DNF and facet scan | Stage 3 Q09 open predicate cannot be answered; 66 pilot budget hits | **Measured** Stage 2 pilot, 2026-09-24; no Stage 3 composition measure | Baseline only |
| Proposed BDD + PyO3 | One Rust truth authority, Python transport | Atom/theory/proof adapter, bundle schema, query executor | Sound if F01–F04 close; otherwise wrong/ambiguous Q09 | Spike evidence for BDD primitives; product and server costs unmeasured | Recommended after revision |
| Library-owned generic route | biodivine for BDD, PyO3/maturin for bridge, FastMCP for tools, Arrow for contracts | Domain-only proof and result partition | Lowest semantic duplication, but libraries do not supply runtime value-stability proof | Interfaces checked; no product benchmark | Coincides with revised proposal |
| Simplest viable Stage 3 first cut | BDD over per-site atoms, same-generation native query; no typed exclusion until exact-use proof exists | Smaller proof surface | Returns unknown more often for Q09; avoids unsound negative; compatibility must have entry anchor | No measurement yet | Viable staged narrowing if F02 cannot close in Stage 3.0 |

## 10. Verification plan

| Claim or risk | Evidence label now | Test / analysis / benchmark | Conditions and expected result | Current result or gap |
|---|---|---|---|---|
| Canonical IDs and bounded operations | **Interface-checked** biodivine source; design **Proposed** | Stage 3.0 spike: shuffled atoms, intrinsic support, apply/transfer/not/given and limit, DNF differential | Equal Boolean functions yield equal persisted IDs across construction order; preflight/limit never yields false; `given` fallback preserves original | Spike in progress; prior stress >10 minutes |
| Typed theory never excludes runtime-feasible path | **Proposed** | Stable/unstable two-site and custom-equality known-answer shapes; CPython differential on pure cases | Only proven stable builtin scalar contradiction excluded; all others independent/unknown | F02 |
| Q09 scenario alignment | **Proposed** | Entry `transport='sse'` versus guards `'http'` and `'sse'`, with/without mutation/call effect | Incompatible only with cited stable transfer; no witness gives unknown, never vacuous match | F01 |
| Native diagram round trip and corruption rejection | **Proposed** | Compile → Delta → bundle → installed extension; tamper support/node/order/cycle/child/format | Matching generation evaluates identically; corrupt/old format rejected at startup | F03/F04 |
| Budgeted exhaustive semantics | **Proposed** | Force stop before final matching operation; page and resume under same generation/query/order | `complete=false`, unvisited/open count; resume yields same exhaustive partition; wrong cursor rejected | F03 |
| PyO3 packaging | **Interface-checked** PyO3/maturin mechanism; route **Proposed** | Build wheel, clean install and editable `uv run`; CPython 3.14.7; server subprocess | Native module loads exact kernel, startup catches mismatch, stdout stays MCP-only | F04 |
| Whole Stage 3 value | **Proposed** | `just check`, `just test-all`, `just pilot`, `just pilot-live`, pre-registered Stage 3 evaluation | Report outcomes separately with budgets, RSS/latency, positive/negative Q09, unknown count; no gold as compiler input | Not run for new design in this review |

## 11. Authority changes and exceptions

**Required authority changes.** Amend ADR-0024 and DESIGN §§B10/3.9/9.9 with exact-use type, stability-proof, node-schema and validator contracts (F02/F03). Amend ADR-0025 and DESIGN §§B13/11.3 with entry-scenario semantics, exhaustive partial-query partition and native packaging/ABI route (F01/F03/F04). Align Stage 3.0/3.6 plan exit checks. Supersede ADR-0010 only after the new boundary is specifiable and the standard review gates close. ADR-0010's embedding spec and retrieval policy remain in §B14.

**Exception records.** None. The findings are MUST gaps, not SHOULD deviations.

**Deferred friction.**

| Item | Why deferred | Reopen trigger |
|---|---|---|
| Hard wall-clock deadline | The amended design explicitly promises work/node bounds, not a strict time guarantee; no measured service SLO yet | Pilot latency or memory breaches the ADR-0025 revisit threshold |
| General solver | The current target is propositional conditions and narrow scalar exclusions; no registered query requires SMT semantics | A registered question cannot be answered with justified BDD/theory proof |
| Move direct lookup/retrieval into Rust | Existing Python paths already serve one pinned generation | Duplicate semantic decisions or measured Python overhead at query scale |

## 12. Decision

**Decision: Revise.** The amended BDD order/ID and bounded-work policy are defensible, but the proposed Q09 contract can produce a misleading compatibility answer without an entry-value link. The typed theory's required test-use observation does not exist in current facts. Node validity/partial-scan rules and the native packaging route remain unresolved. These are supported-scope obligations, so the ADRs are not ready to accept as written.

| Priority | Change | Findings | Acceptance evidence |
|---|---|---|---|
| Correctness first | Anchor query scenario and prove transfers; define exact-use typed theory and runtime assumption | F01, F02 | Counterexamples and Q09 differential, same-generation proof chain |
| Validity and recovery | Version/validate BDD node projection; partition unvisited candidates under budgets | F03 | Corrupt load rejection, interrupted/resumed exhaustive query |
| Integration | Choose and pin native build/import/ABI route | F04 | Clean wheel/editable install and actual FastMCP/native integration |
| Measured cost | Complete Stage 3.0 and pilot measurements | — | Budget-hit, compile, query latency and memory reports |

**Final check.** This review judges a proposal, not an implemented Stage 3. It supports the current Stage 2 answers and acknowledges the existing single-generation baseline, but does not label new kernel/query behavior Tested or Measured.

### Corrected-proposal disposition (2026-09-24)

The author amended ADR-0024, ADR-0025, DESIGN and the forward plan during review. This assessment supersedes the initial §6 gate table and decision above; the finding rows remain the concrete counterexamples and implementation checks.

| Finding | Inspected proposed correction | Remaining boundary | Status |
|---|---|---|---|
| F01 | ADR-0025 now anchors a user condition to operation/formal entry value, requires `flow_test_value_links` for source-test comparison, returns unknown without it, and defines compatibility as may-model non-refutation rather than proof of execution. The relation names operation/formal and test/use IDs, span, place/path, proof origin, cited flow/summary facts and effect-model digest. The Rust producer starts with direct paths; its shared validator rejects ambiguous binding, alias uncertainty, unmodeled effects and mixed snapshots. A later modeled transfer must preserve identity, not merely be pure. DESIGN §11.3 and plan Stage 3 carry it. | Relation, producer, validator and query path remain unimplemented and untested. | **Corrected at proposal level; implementation gate open** |
| F02 | ADR-0024 now requires `flow_test_types` from Pyrefly's exact expression trace, with test/use IDs, span, type term, closed proof origin and cited facts. Its shared validator checks joins, role, term, origin and references; absent/ambiguous trace proves nothing. Exact builtin type needs a modeled literal or resolved exact `type(x) is builtin` guard; same-evaluation identity proves stability only. Broad annotations, Any/Unknown, subclasses, custom equality and intervening effects cannot prove exclusion. Raw Boolean IDs stay independent of theory revision. DESIGN §3.9 and plan carry this. | New attributed relation, producer, validator and theory are unimplemented and untested. Today's `type_observations` alone cannot support exclusions. | **Corrected at proposal level; implementation gate open** |
| F03 | ADR-0024 now declares fixed terminals, root/atom/low/high nodes and publication/native-load validation of closure, acyclicity, atom order, reduction and Merkle IDs. ADR-0025 now partitions matched, proven-excluded, source-open and unexamined, with `complete` requiring the last two empty and a resumable cursor/count after a budget stop. DESIGN §§B13/3.9/11.3 carry these rules. | New schema, shared validator, bundle and native executor remain unimplemented and untested. | **Corrected at proposal level; implementation gate open** |
| F04 | ADR-0025 selects a separate `python/lctx_semantics` uv member, `maturin==1.15.0`, `pyo3=0.29.2`, `lctx_semantics._native` on CPython 3.14.7, dependency from `lctx_mcp`, and a startup kernel-format handshake. PyO3/maturin interfaces and release availability were checked on 2026-09-24; DESIGN §B13 carries the route. | Current workspace is still `uv_build` only; no wheel/editable import or FastMCP/native round trip has run. | **Corrected at proposal level; implementation gate open** |

| Gate | Corrected-proposal verdict | Evidence and remaining action |
|---|---|---|
| G1 | Pass at design level | `flow_test_types` and `flow_test_value_links` have separate owners, identities and revision dependencies (F01/F02) |
| G2 | Pass at design level | Entry/source and exact-type proofs are typed and constrained; missing/ambiguous evidence returns unknown (F01/F02) |
| G3 | Pass at design level | Native node invariants and rejection points are stated; implementation/test required (F03) |
| G4 | Pass at design level | Read-only/typed native boundary remains explicit |
| G5 | Pass at design level | Four-way partition, incomplete result and resumable cursor are stated; implementation/test required (F03) |
| G6 | Pass at design level | Raw BDD IDs are separate from theory; checked link/proof relations govern interpretation and transfer (F01/F02) |
| G7 | Pass at design level | Specific native package and ABI route are selected; product capability stays Proposed until built (F04) |
| G8 | Pass | Generic BDD/bridge/build/transport capabilities have library owners |
| CI-G1 | Pass at design level | May-model wording, provider-derived test-use observation, closed exact-type origins and unknown fallback are explicit (F01/F02) |
| CI-G2 | Pass at design level | Proposed proof rows cite same-snapshot source/summary facts and validator checks closure; implementation gate open (F01/F02) |
| CI-G3 | Pass | No reference-to-input route found; registered evaluation remains separate |

**Corrected principle verdicts.** Every applicable principle listed in §7 is **Satisfied at the proposed design-contract level** by the amended proof, validation, partition, version and packaging rules; the four initial defects have specified corrections and no in-scope MUST gap remains. These are design verdicts, not implementation evidence. No new Stage 3 behavior has been labelled Implemented, Tested or Measured by this review.

**Current decision: Accept the corrected proposed ADR-0024/ADR-0025 design.** The proposal now gives each semantic fact one owner, specifies the sound unknown path, validates the native graph, preserves partial-query state and chooses a qualified bridge. F01–F04 remain implementation and verification gates: build the new proof relations/checkers, node schema, kernel and native package, then run the counterexamples, round trips, full repository checks, pilot and registered Stage 3 evaluation. Accepting these ADRs does **not** accept Stage 3 product execution.
