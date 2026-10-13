# Catalog execution and lifecycle completion plan

**Accepted direction / Proposed implementation, 2026-10-12.** Plan authoring implements the
operator's approved consolidation; it does not execute the production packages below. This
coordinator is the sole current disposition, dependency and receipt owner for the combined scope.
It entirely replaces the four SurrealDB lifecycle plans dated 2026-10-12. Other active plans remain
paused, separately owned and unsuperseded until CE03 reconciles affected scope after this track.

## 1. Purpose, authority and reading route

Deliver every actionable finding, recommendation and necessary investigation from the
[Catalog execution/patterns review](../design_review/reviews/design_review_catalog-execution-patterns-target_2026-10-12.md),
plus all surviving obligations of the
[lifecycle target review](../design_review/reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md)
and its implemented foundation/completion follow-ups. Source-qualified IDs remain distinct:
`catalog#F01` is not `lifecycle#F01`. The original reviews retain their dated judgments; §6 owns
current disposition. Do not reinterpret source acceptance as a runtime pass.

| Companion | Responsibility |
|---|---|
| [Protected completion and canonical preparation](catalog-execution-completion-plan_2026-10-12.md) | CP00–CP04: earned contribution/internal/boundary authority, reusable ordered families, native windows, alias topology, complete physical-consumer cutover |
| [Consumer preparation](catalog-execution-consumers-plan_2026-10-12.md) | CU01–CU04: independent restore lookup, exact partition positions, direct logical identity/frozen export, immutable Python recipe response |
| [Lifecycle and development operations](catalog-execution-operations-plan_2026-10-12.md) | OP01–OP03: diagnostics/taxonomy, exact SurrealKit/schema/target ownership and all C01–C11 platform decisions |

The architectural owners are [storage/publication §6](../design/sections/storage-and-publication.md),
[semantic model §15.1](../design/sections/semantic-model.md) and
[serving](../design/sections/synthesis-and-serving.md).
[ADR-0165](../adr/0165-prepared-completion-and-owned-lifecycle.md) records the approved completion
change and carries unaffected lifecycle decisions forward. ADR-0152's independently retained
completion guarantees remain; internal dependency consumption receives a different explicit contract.
Core principles 3.3, efficiency heuristics 1.0 and code-intelligence profile 1.5 govern this work.

**Authoring baseline:** main `7d84ce14`, clean before these document edits. It commits the formerly
dirty lifecycle implementation. The Catalog review froze `5090aeec` plus that dirty source;
all ten source hashes in its [snapshot](../design_review/evidence/2026-10-12_catalog-execution-patterns-target/review-snapshot.json)
match the current baseline, checked 2026-10-12. No intervening source correction closes its findings.
The source-led assessment below is not a build or service qualification. Minor concurrent edits
must be preserved and checked only where they change a consumed contract.

**Scope boundary:** prioritize all functional scope and narrow functional validation. Actual generated
code-intelligence analysis linked to native queries is required. External content embedding and
MCP-submitted query execution are excluded; affected components retain compile checks, and CU04
has pure local adapter controls. No broad `qualify`/`test-all`, unrelated quality repair, historical
benchmark campaign or operator adoption is a prerequisite. The operator selected this narrower
acceptance for the predecessor track; consolidation does not restore its former assembled gate.

## 2. Combined target and foundation assessment

The current implementation has valuable typed semantic owners, protected selected copies,
independent admission, terminal streams, original evidence, typed request reconciliation and real
host/service/storage ownership. Keep those guarantees. The Catalog review is right that protected
fragments still feed fresh full-root sorting/hashing, and semantic internal views still trigger
independent native member/effect lifecycles. Reduced byte observation does not remove this work.
Latest timeouts establish missing functional acceptance, not a measured dominant cause or engine bug.

The target has three operations: independently complete/protect a contribution, select an exact
internal view through its live owner, and earn a frozen independently retained boundary only when
admission/publication/retention/cold access actually needs it. Preserve logical views and bindings;
map them to exact selectors within an independently necessary compound boundary. A separate retained
product may need its own boundary. Internal relation declarations do not automatically allocate
member copies, full identities or another durable completion lifecycle.

Canonical preparation belongs to the protected operation. Normalize native IDs before ordering;
merge reusable immutable sorted runs without decoding/re-encoding/resorting unchanged fragments.
Hash the exact six-family v4 union only at its real identity boundary. A changed contributor set
still requires its proper canonical fold; hashing fragment digests or physical member order is wrong.
Family reuse binds full exact dependencies, selected physical incarnations and interpretation context,
not logical contributor IDs alone. No new durable proof cache, universal prepared-data object,
background service, scheduler or global compile serialization is selected.

The foundation assessment favors extending existing native ordered runs/point indexes and compiler
compact root preparation over a new execution framework. DataFusion/Arrow remain appropriate for
relational batches/selection, but their generic merge does not supply native key-conflict refusal,
logical normalization, ownership and terminality. BLAKE3 does not expose arbitrary fragment-state
concatenation. Required set-shaped native assurance must use actual bounded identity-correlated
queries. Alias closure uses prepared directed adjacency and a frontier, including cross-contributor
paths. Restore, root partitions and transport identity are separate migrated consumers of these
principles; sharing a primitive does not close their findings by association.

Credible changes include another output relation with explicit empties, larger overlapping libraries,
a fresh retained reader, a changed binding/alias and interrupted effects. New semantic declarations
stay model-owned. Physical adapters add only genuinely needed indexes/boundaries; cold readers
re-earn authority. No cloud/distributed topology, new ranking semantics or behavioral-research queue
is implied. Expected efficiency follows removed repeated work; speed/capacity remains unmeasured.
If a useful comparable observation makes the reasoned preferred design more than 3× slower, revisit
its premise. Do not create a benchmark campaign to choose it, or infer that ratio from unrelated
parallel workloads, nested spans, a timeout or unlike bounds.

### Hard cutover and retention

Regenerate useful current views and fresh evidence from pinned inputs. Existing data, historical
examples/benchmarks, compatibility readers, dual writes, rollback assets, converters and old-format
runtime records are not delivery prerequisites. Keep actual live owners, unresolved effects and
current evidence consumers until authenticated drainage/disposal releases them. Fresh owned targets
need not repair old paused campaigns. Replacing selected project state requires its real owner;
this plan does not inspect/reset/activate shared or operator databases. Original failed/Unknown
outcomes remain truthful even when whole-target discard settles their host obligations.

## 3. Confirmed rule changes and decision routes

The operator explicitly confirmed these Catalog dispositions during the authoring approach on
**2026-10-12**, separately from the earlier lifecycle RC01–RC06 approval. ADR-0165 and its owning
architecture edits apply the confirmed direction before dependent implementation.

| Source impact | Operator disposition | Route / consequence |
|---|---|---|
| [catalog#RC01](../design_review/reviews/design_review_catalog-execution-patterns-target_2026-10-12.md#RC01) | Accept reusable canonical preparation and identity at actual boundaries | CP00/CP01/CP02/CP04; retire mandatory fresh six-family reconstruction per internal view and IF10's conservative universal-root target |
| [catalog#RC02](../design_review/reviews/design_review_catalog-execution-patterns-target_2026-10-12.md#RC02) | Accept earned protected contribution/internal selection and demand-shaped durable realization | CP00/CP04; explicitly migrate externally retained/cold consumers and physical codec instead of silently weakening completion |
| [catalog#RC03](../design_review/reviews/design_review_catalog-execution-patterns-target_2026-10-12.md#RC03) | Retain flat v4; do not adopt a canonical DAG | CP01/CU03 preserve exact identity. Revisit only for a concrete residual repeated boundary-identity consumer after demand reduction, with a separately approved version/consumer change |
| lifecycle#RC01 / RC02 | Prior accepted owned destructive/schema targets and qualified binding; managed default retained | OP02 preserves explicit whole-service containment before private beta effects; secure remote connector only for actual demand |
| lifecycle#RC03 | Prior preserved Rust/model schema authority | CP04/OP02 regenerate assets; no handwritten equivalent schema or introspection authority |
| lifecycle#RC04 | Prior preserved hard regeneration cutover | CE02; no old-data/history prerequisite or legacy reader |
| lifecycle#RC05 | Prior preserved patched progressive/finality contracts and honest provenance | OP01/OP02; SurrealKit beta.1 is development tooling, product SDK/RPC remain patched 3.3.0 |
| lifecycle#RC06 | Prior preserved ownership, Unknown and non-adoption | OP01/OP02/CE02; no timer, generic sweep retirement, implicit shared reset or operator selection |

The [lifecycle review's rule table](../design_review/reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md#verification-and-rule-impacts)
provides the original RC01–RC06 wording. The successor ADR carries these decisions without reopening
approved targets or upgrading their evidence. Material new rule impacts require explicit disposition;
routine implementation choices inside these contracts do not require another approval round.

## 4. Packages, dependencies and completion

Packages are coherent changes, not a barrier after every row. Ready independent work may proceed
concurrently with one coordinator for model/native boundaries, integration and acceptance. Shared
files are editing constraints even when capabilities are independent. Working prerequisites require
implemented consumers and focused evidence; an agreed interface alone is insufficient.

| Package | Prerequisite and result |
|---|---|
| CP00 — earned authority and bounded decisions | Approved ADR/target; trace actual producer, internal, retained, cold and freeze consumers. Specify physical selectors/codec, mutation premises and IF04 decision. Fresh bounded design advice on consequential interfaces before CP04 integration |
| CP01 — reusable ordered union | Existing charged ordered-run owner; normalize first, non-consuming exact union, EOF/conflict/failure-safe fold. Revealing spill/overlap/late-error controls |
| CP02 — dependency-aligned preparation | CP00's actual ownership and CP01 runs; protected contribution preparation and exact per-family reuse keys. No reusable partial success or detached released owner |
| CP03 — native windows and alias frontier | Existing reader/budget/hold owners; implement tested primitive windows/adjacency early. Migrate remaining assurance consumers alongside CP04's reduced-demand route |
| CP04 — all internal/retained/cold consumers | Specified CP00 plus working CP01/CP02 and needed CP03 primitives; migrate contribution completion, providers, frozen/admitted/published/retained inventories, sparse access, backup and cold codecs together. OP01/OP02 adapt affected diagnostics/assets |
| CU01 — selected dump closure | Immutable dump and existing spool/run owners; independent demanded lookup preparation and restoration; can proceed independently, integrates with CP04's cold codec |
| CU02 — union partition positions | CP00 exact inputs and current compact topology; input-port-aware positions/gathers without repeated rich-row scans. Consumes CP04 internal authority |
| CU03 — direct identity and frozen export | CP01 logical fold and CP04 frozen/retained contract; migrate checksum and portable consumers. Independent incoming/destination admission retained |
| CU04 — one immutable recipe response | Existing Rust parser/recipe owner; eager checked private response and immutable fields. Independent low-priority package, still required before final handback |
| OP01 / OP02 — changed operations seams | Existing implemented diagnostics, schema, sessions and campaign owners; adapt changed completion identities/definitions and only affected consumers/controls |
| OP03 — capability decisions | Retain all C01–C11 supported outcomes/triggers. Reopen only a changed complete-route premise; adopted mechanism gains its migration/acceptance before dependent execution |
| CE01 — scoped integrated acceptance | All required migrated functional scope; independent integrated design review and narrow actual controls below. May exercise ready critical-path journey before CU04 without closing the track |
| CE02 — selected current-state regeneration | Working replacement and relevant CE01 evidence plus live-owner inventory; migrate useful views or regenerate, remove obsolete runtime/state, verify only affected acceptance after cutover |
| CE03 — reconcile older tracks and hand back | CE01/CE02 complete and no unsatisfied required scope; update affected older owners using exact fulfillment/remaining obligations, then resume their queue |

CP03's primitives are early prerequisites; its full consumer migration completes with CP04. This
breaks the apparent CP03/CP04 cycle. CP04 includes the CU01/CU03 physical-codec seams before deleting
the old format, rather than deferring necessary cold consumers to an inventory phase. CU02 may develop
against CP00 but needs working protected selection to finish. OP02 is an existing safe schema probe
capability, not a mandatory probe for every source decision. No package needs a new historical corpus.

### CE01 — narrow functional acceptance and independent review

During implementation use touched-crate compile checks and minimal affected contract controls with
normal available concurrency. Use explicit `just verify --select FAMILY[:BOUNDARY]` filters or bare
controls under `just fixture --`; verification observes prerequisites and does not implicitly sync,
install or restart services. An environment-shaped failure has its existing guard-free repair route.
Actual privileged schema tests use OP02's wholly owned target; ordinary product attempts use installed
logical validation. No scratch fallback, job cap or raised budget/deadline makes acceptance pass.

Required composition evidence is actual Catalog generation of code-intelligence analysis, independent
admission, unselected publication, native get/search/compare, attributed evidence and exact original
bytes, with embeddings disabled. Use the existing focused control
`current_catalog_analysis_links_public_native_operations_and_original_evidence` and facts availability
`availability_reuses_exact_admission_and_refuses_changed_invalid_membership`, adapted only for the
new legitimate contract. Preserve their meaningful assertions and current supported bounds. The old
16 MiB facts capacity contract was retired as incompatible; the current full-compilation budget
establishes semantics, not low-memory capacity. Keep the unchanged 64 MiB receiver and revealing
insufficient-reservation controls where their premises still apply.

Also require independently specified canonical known-answer unions, changed-set bindings/aliases,
exact-copy/absence/foreign-row refusal, late-error/cancellation/finality, cross-contributor alias
closure, fresh-owner retained/sparse access, independent cold/import/restore identity, root-partition
semantics and affected diagnostic/schema/owned-cleanup controls. The companions define revealing
cases; do not replay whole families merely to satisfy a row count. A passed body with failed enclosing
cleanup is not a full pass. Review the integrated physical authority, consumer coverage, failure paths
and execution fit independently; source acceptance is separate from runtime acceptance.

External embeddings and MCP-submitted query behavior stay not_run; touched components compile and
CU04 exercises only pure local recipe behavior. Broad assembled qualification, unrelated quality
leaves, old paused recovery and real-library/operator activation remain separately scoped. If actual
Catalog still fails, diagnose its current failing phase and owned state before retrying; neither driver
substitution nor another unchanged journey closes a structural finding. Timeouts may collect evidence,
but do not by themselves decide the preferred architecture or establish a performance ratio.

### CE02 — regeneration and CE03 — delayed older-plan reconciliation

Inventory actual consumers and owners of selected project views. Quiesce their writers/readers,
replace/migrate supported current views or regenerate from pinned inputs, and delete superseded
physical schema/readers, compatibility-only helpers and transition assets. Retain minimal transition
identity only while an actual recovery/disposal obligation needs it. Unknown executor drainage blocks
that target's discard; confirmed whole-target discard may settle logical cleanup without preserving
its old data. No shared/operator activation is implied. If cutover changes acceptance, rerun only the
affected functional journey before completion.

Only then compare the efficiency, persisted-execution, native/holistic/unified/SA9 and other paused
plans with actual delivered contracts. Preserve original IDs; close precisely fulfilled scope,
retire historical-only requirements explicitly when they lack a current consumer, and leave broader
obligations at their existing owner. S04/F01 and N10/F05 overlap without importing all N10 migration.
SA-F02, BC3, SM8 and old interrupted campaigns do not close automatically. Updating links now only
transfers the lifecycle coordinator; it does not preempt CE03's substantive older-plan reconciliation.

## 5. Complete predecessor transfer

These are provenance mappings, not a second progress ledger. Section6 owns their current state.
All 18 old packages have a successor; unchanged implemented foundations are reused, not restarted.

| Retired package | Surviving obligation and new route |
|---|---|
| LC00 | Approved rules/architecture and pre-stage consequential review → ADR-0165, §3, CP00 |
| LC01 | Independent integrated source review plus actual scoped functional acceptance → CE01 |
| LC02 | Live-owner current-state migration/regeneration and obsolete-path retirement → CE02 |
| LC03 | Exact older-plan reconciliation/handback → CE03 |
| LN00 | Mutation/retirement validity map and original typed outcomes → CP00/CP02 preserved premises |
| LN01 | All protected selected serving/evidence consumers and populated acceptance → CP04/CE01 |
| LN02 | Bounded independent auditing foundation retained; remaining scalar assurance → CP03/CP04 |
| LN03 | Protected blob/closure reuse retained; family preparation and internal demand corrected → CP02/CP04 |
| LN04 | Conservative same-incarnation recapture retained; continuous replay fit decision → CP00/IF04 |
| LN05 | Supported no-new-cross-owner-witness decision retained; new authority is protected operation/boundary, not a portable proof → CP00 |
| LO00 | Typed bounded durable diagnostic writer/host round trip retained; changed completion producers migrate → OP01 |
| LO01 | Composed inspection, lifecycle taxonomy, honest provenance and valid recovery routes → OP01 |
| LO02 | Every conditional platform avenue and future-change trigger → OP03 C01–C11 |
| LD00 | Exact beta.1 development-only dependency/session/capability boundary retained → OP02 |
| LD01 | Generated sole-authority definitions, complete ordered module closure and equivalence → CP04/OP02 |
| LD02 | Exact owned remote campaign binding, beta private-client containment and checked disposal → OP02 |
| LD03 | Pure/schema/remote controls, reports, setup/failure/cancellation distinctions → OP02/CE01 |
| LD04 | Actual server/SDK/tool/schema/epoch provenance and operational support limits → OP01/OP02 |

The source review's taxonomy, promotion/admission/publication/selection, retirement, original bytes,
backup closure and cross-owner independence survive through CP00/CP04/OP01/CE01. C01 references,
C02 events/changefeeds/live/history, C03 sweeps/timers, C04 graph/materialization, C05 retrieval,
C06 database APIs, C07 WASM, C08 buckets, C09 parser/plans, C10 Studio/MCP/telemetry and C11 complete
storage alternatives all retain supported decisions in OP03. C09's useful on-demand diagnostics/
schema parsing are adopted through OP01/OP02. Other non-adoptions are demand-triggered, not permanent
bans. No required finding is closed by a conditional avenue's justified rejection.

## 6. Sole disposition and execution checkpoint

**2026-10-13: remaining-scope execution resumed from `1b45554b`. CU01–CU04 and overlapping CP04
equivalence are source-integrated; focused acceptance is partial.** Work is on shared main;
no auxiliary worktree was created. Source integration is not whole-package or CE01 acceptance.
[ADR-0166](../adr/0166-exact-completed-transport-and-selector-eligibility.md) accepts the operator's
separate exact-copy carrier and conservative selector eligibility. Semantic format4/v4 stays unchanged;
SnapshotHandle/publication v3 binds typed eligibility, and schema8 distinguishes fresh imported copies.
Required caller-retained artifact/backup receipts replace inferred incoming expectations. CU01's demanded
dump preparation, CU02's exact input-port positions, CU03's direct identity/frozen export and CU04's
immutable eager recipe response are implemented. CE01's generated-analysis composition, CE02 selected
current-state regeneration, CE03 older-track reconciliation and whole-plan acceptance remain distinct.
Shared/operator and old paused targets remain untouched.

The operator approved fresh-current-quiescence transfer of only dead ordinary campaign maintenance
markers (ADR-0167); missing historical marker-child ancestry is not an independent prerequisite.
Actual current local and executor drainage remains required; specialized recovery/SOURCE_JOIN and
original Unknown outcomes are unchanged. Disposal, original-store last-use finality and exact
attempt/epoch cold assertions precede new native acceptance. Retained `69c2a4` output shows native
inventory reached output registration about0.41s after entry with only a few seconds left on the
aggregate deadline; it does not establish inventory reads as the dominant cause. Current source
work removes an independently demonstrated duplicate complete output map and adds phase boundaries.
Fresh owned schema8 campaign i install `20261013T210510.943Z-2a149b` passed with confirmed local
cleanup, installation `c3e1f0a3-8e7e-42a1-924c-eabab61bc866`, configuration
`catalog-ce-20261013-i/installation.json`, retained reviewed SOURCE_JOIN server generation `ced8e696…`.
This does not qualify product composition or dispose campaign g.

**Passed current disposal and tooling, 2026-10-13.** Operations source `8599248a` and reviewed
privacy/participant-retention follow-up implement ADR-0167.67 focused host/token controls passed;
status/development projection controls passed2+2, participant retention controls passed8, and
terminal supplementary-run lifetime control passed1 (`3f0a0e`, `8330cb`, `984699`, `7cec89`).
Checked G disposal `20261013T212545.600Z-45058a` passed every step with confirmed local cleanup;
ordinary-marker transfer and fresh supplemental `62296c` quiescence were journaled. Exact daemon,
datastore, attachment, installer, unit, storage and credentials are released. Original native Unknown
remains unchanged. Current schema8 tooling build `a95f1a`, generated/projection/parser controls
`22ad64` (4/4), beta Sync/readback `26f90f` and declarative sample `eb0e28` (1 suite/6 cases) passed
with confirmed local cleanup on owned i. The worker still truthfully reports remote uncertainty
until that target's checked disposal.

**Current functional corrections.** `ff0c0eb3` and `3d3608f0` migrate actual last-use success owners
through owned drainage, original native finality and signal completion; generic borrowed artifacts
remain reusable. Actual original-session invalidation/closed-state preservation passed in `f8a22a`;
its Facts cold companion failed strict source absence after113s. Its exact source nominations were
spent while the source publication reader remained pinned, returning terminal retained results.
The changed control closes/retires publication first, then advances only those exact captured
source nominations through an independent observer; no arbitrary/global root or new nomination is
introduced (`f41508` running). `8886ccca` removes the duplicate complete native-inventory output
map, preserves whole-projection prevalidation and streams through bounded writers; compile and8
pure projection controls passed. Remaining serial transport/selector/held-alias observations were
found during current consumer review and are being migrated through existing exact windows before
final CE01. No timeout cause or measured speedup is inferred from these source corrections.

Independent implementation review found selector-crossing alias witnesses, partially committed
receiving attachment, and abandoned retained-boundary pin release. Corrections are source-reviewed:
alias edges carry selector eligibility; full attachment prevalidation precedes one receiving CAS;
retained construction/Drop/failed reader close preserve Unknown. Product replay rechecks and reuses
its original exact compound boundary under a per-request gate instead of minting another parent
after unrelated source incidence changes. Controls inspect actual contributor holds and concurrent
replay. Runtime evidence remains separate. IF04 retains conservative typed recapture: no complete
simpler specialization was established; sustained replay is the explicit revisit trigger, not an
asserted timeout cause.

| Source-qualified finding | Current disposition and responsible route | Closure/evidence boundary |
|---|---|---|
| catalog#F01 | Implemented/source-reviewed; focused acceptance pending; CP01/CP02/CP04 | Internal views stop full reconstruction; exact final union independently checked; actual CE01 journey |
| catalog#F02 | Native reuse/retained collision and exact portable incidence implemented/source-reviewed; focused acceptance in progress; CP00/CP04/CU03 | Protected independent eligibility, root-qualified copy closure, fresh receiving authority and complete cold/physical consumers; enclosing CE01 remains separate |
| catalog#F03 | Implemented bounded windows; full inventory-demand source-reviewed/actual controls pending; CP03/CP04 | Actual bounded set windows for remaining demand, exact correspondence and receiving CAS/finality |
| catalog#F04 | Directed selection implemented; 25 pure controls passed2026-10-13; cross-boundary equivalence and enclosing native acceptance open; CP03/CP04 | Prepared directed frontier, composed exact aliases, unreachable-source refusal, spill and safe reuse |
| catalog#F05 | Implemented/source-reviewed; backup reached; cold restoration open at closeout; CU01 | Demanded typed metadata, one combined original/payload pass, complete EOF/conflict checks, independently supplied receipt and fresh restoration |
| catalog#F06 | Closed at source/focused-Tested strength2026-10-13; CU02 | Exact input-port position/gather controls passed, duplicate physical rows and source order preserved, independent compact/spill checker topology unchanged |
| catalog#F07 | Implemented/source-reviewed; focused transport bodies passed, enclosing acceptance partial; CU03 | Direct unchanged v4 fold, opaque frozen/published preparation once, root-qualified exact copies and receiving semantic/eligibility admission |
| catalog#F08 | Closed at source/focused-Tested strength2026-10-13; CU04 | Factory-checked recursively immutable Spec, one eager versioned Rust response; all11 pure adapter controls passed after native extension refresh |
| lifecycle#F01 | Open, implementation/source evidence available; CP04/CE01 | Populated generated-analysis/admitted-publication/native-query assertions remain not_run |
| lifecycle#F02 | Closed at source/focused-Tested strength, inherited 2026-10-12; CP03 preserves it | Existing actual originals/audit, exact-view and late-terminal controls; not proof all catalog#F03 scalar sites migrated |
| lifecycle#F03 | Partial, protected batch preparation source-accepted and cold-batch Tested; CP02/CP04 | Existing scoped reuse survives; catalog#F01/F02 and enclosing functional acceptance remain open |
| lifecycle#F04 | Closed at source/focused-Tested strength, inherited 2026-10-12; OP01 preserves/adapts | Actual writer/host diagnostic round trips; failure/Unknown/disposal independent |
| lifecycle#F05 | Closed for conservative recapture, inherited 2026-10-12; CP00 preserves | Equal-peer/replay/mutation/ABA/retirement/cancellation controls; continuous replay is separate IF04 |
| foundations#IF01 / IF02 / IF03 | Closed at inherited source/focused-control strength; OP02 | Authenticated disposal receipt/executor, private installer/unit retirement, configured namespace inventory; actual checked disposal supports scope |
| foundations-follow-up#IF04 | CP00 source decision complete2026-10-13: retain conservative typed recapture | Transaction-local exact replay specialization only if all exclusion/incidence/new-hold premises survive. Otherwise supported keep-conservative result with continuous-replay revisit trigger; not asserted timeout cause |
| foundations-follow-up#IF05 / IF07 | Implemented/source-accepted; runtime enclosing acceptance pending; CP02/CP04/CE01 | Corrected temporary/retained descriptor accounting; 64 MiB receiver body passed, original enclosing fixture failed/Unknown. Both profile-availability/memo and failed-replacement preservation assertions reached; full facts availability/refusal and enclosing cleanup passed in0e9394; populated Catalog composition remains open |
| foundations-follow-up#IF06 | Closed at inherited source/focused-control strength; OP01 | Borrowed server observation authenticates original executable owner; no new release authority |
| foundations-follow-up#IF08 | Closed at inherited source/focused-control strength; OP02 | Durable token journal, authenticated restoration and actual beta SIGTERM recovery; disposal separate |
| foundations-follow-up#IF09 | Closed at inherited source/focused-control/checked-disposal strength; OP02 | Bounded journaled logical children and authenticated whole-executor discard; original failed outcomes preserved |
| completion#IF10 | Native target and exact cold-alias incidence source-integrated; focused acceptance in progress through catalog#F01/F02/F03 | Reusable canonical ordered runs, exact final logical union and independently necessary physical boundaries; actual current-source composition remains required |

Follow-up evidence owners are the
[foundation review](../design_review/reviews/design_review_surrealdb-lifecycle-foundations-integrated_2026-10-12.md)
and [completion review](../design_review/reviews/design_review_surrealdb-lifecycle-completion-integrated_2026-10-12.md).
The foundations review introduces IF04/IF05/IF08 and records the root/resume IF06 and reviewed
IF07/IF09 follow-ups. The completion review refines IF05/IF07 and introduces IF10. Labels above
preserve that provenance without inventing additional review findings. CP00's new decision does not
erase the original limited acceptance.

### CU01–CU04 execution, 2026-10-13

Baseline `e98bc2e5` was clean; all changes in this slice are on shared main. No worktree, dependency
move, operator selection, external embedding, MCP-submitted query or benchmark campaign is part of
this execution. The accepted envelope/eligibility decision has no legacy-reader/data preservation
obligation; CE02 still owns actual selected-state regeneration. The current source review accepts
native eligibility before reuse/convergence, root-local retained collision serialization, exact
copy/alias reconstruction, independently supplied receipts and checked exporter finality. It also
accepted the final public observation-window call and physical-row sorter correction. This is
Interface-checked evidence, not an enclosing runtime pass.

- **Passed** guard-free `just ready`, `20261013T172759.801Z-356d02`; then CU04
  `uv run --no-sync pytest python/lctx_mcp/tests/test_embedder.py -q` under run-owned `--basetemp`,
  `20261013T173232.444Z-4710d2`, all11 passed/local cleanup confirmed. Rust owns the unchanged hashes;
  no external encoder or MCP request ran.
- **Passed** affected host schema/installer selection, `20261013T170432.951Z-58766c`,
  `uv run --no-sync pytest tests/scripts/test_surrealdb_service.py -q -k
  'current_installer_schema or installer_handoff or installer_generation'` under run-owned
  `--basetemp`, all24 passed/local cleanup confirmed. Initial host setup used another user's
  `/tmp/pytest-of-paul` and failed before bodies; the run-owned destination repaired that prerequisite
  without changing another owner's directory.
- **Passed** current CLI/serving/cpg test-target compile,
  `20261013T175509.346Z-973db5`, `cargo check --locked --release -p lctx -p lctx-serving -p cpg-core
  --tests`, local cleanup confirmed. Subsequent sorter/fixture changes are covered by their explicit
  native rerun; no claim of whole-workspace compilation follows.
- **Partial** focused pure Rust `20261013T173232.621Z-a58152`:46 passed/2 failed. The passes include
  independently specified transport/eligibility distinctions, streamed JSON framing parity, both CU02
  gather/topology cases, compact maximum Bytes framing, oversize refusal, demanded original/spill/
  late-error and backup-carrier controls. Failures were stale SnapshotHandle required-field and frame
  fixture expectations; both corrected bodies passed in `20261013T174828.943Z-f17a04`.
- **Partial** `f17a04`:5/7 bodies passed, including distinct-root equal-descriptor fresh copies,
  retained same-root equal/unequal eligibility, small cross-pool transport and the two pure corrections.
  The alias-chain fixture mutated completed incidence, and maximum import's family sorter incorrectly
  used the narrow1MiB default. Fixture setup and the existing64MiB physical-row profile are corrected;
  no row, workspace or timeout limit was raised. Its enclosing fixture failed/native cleanup remained
  Unknown. Exact owned campaign a (`83262d69-adb2-4286-acb3-391c752a94b5`) checked-disposal
  `20261013T175441.904Z-c53c12` passed executor/datastore/attachment/installer/unit/storage/credential
  removal; logical drain independently failed. Original failed/Unknown outcomes are unchanged.
- Schema generator `20261013T170702.102Z-d2aa4c` **passed**. Owned schema8 installs
  `20261013T172759.626Z-071b67` and `20261013T175509.590Z-a87e22` **passed**. Campaign b is
  `e9a7b4ea-1431-48bb-a832-5e16eefb8ff9`, using `catalog-cu-20261013-b/installation.json`; its
  subsequent checked disposal is recorded below.

- **Passed** CLI receipt/parser controls `20261013T180310.881Z-98edbd`,
  `cargo nextest run --locked --release -p lctx --bin lctx -E
  'test(native_commands_keep_artifact_and_selection_boundaries_explicit) |
  test(each_command_takes_its_own_options)'`,2/2/local cleanup confirmed.
- **Partial/failed** native/backup `20261013T175908.710Z-aa28ee`,6 cases: maximum full transport
  decoder and actual maximum fresh import passed their bodies; alias-chain and cold-binding fixtures
  failed. The first requires independently reachable dependency seeds as well as the composed root;
  the corrected fixture adds its typed EntityRef output before completion, retaining cross-copy
  chain/order/absence assertions. The second expected the removed decoder's diagnostic; current
  Serde rejects the obsolete outer tag before later malformed bytes and native effects. Both are
  corrected. Two existing Normalized backup journeys timed out before backup assertions; logs reached
  artifact admission, not independent restoration. Neither is a passed CU01/CE01 case. Enclosing
  fixture failed/native Unknown, attachment `0e9fe09a-f75c-41ef-990c-797ae18b95ae`.
- **Partial/failed** composition `20261013T175908.991Z-69c2a4`: facts availability/refusal body
  passed; existing detached Normalized artifact and Catalog journeys timed out. Catalog completed
  normalization/configuration and entered `analysis_native_inventory`; cancellation then reported
  `native transaction retry cancelled`. Generated analysis, admission, publication and linked-query
  assertions remain unproven. This is a phase boundary, not an elapsed-time attribution or design
  speed verdict. Enclosing fixture failed/native Unknown, attachment
  `a94530b7-97be-484a-9e35-0fae8e60d1ed`; host workers drained/local cleanup confirmed.
- **Passed** checked whole-target disposal `20261013T181257.608Z-6e30d8` for campaign b, including
  executor/datastore/attachment/installer/unit/storage/credentials; logical validation drain failed
  independently. Original failed/Unknown receipts remain unchanged. Owned schema8 campaign c
  `e267f0b6-1792-4249-9180-a2da0ca6b9b4` installation `20261013T181326.182Z-b006a1` **passed**.
  Its narrowed controls use actual Facts compilation/admission for artifact/backup contracts, exact
  originals, source retirement and concurrent fresh restore; original Normalized/Catalog outcomes
  stay separate. The alias fixture initially failed compile on ambiguous EntityRef::encode
  (`20261013T181448.173Z-11360a`); explicit Record::encode corrects that test-only call.

- **Partial/failed** narrowed c `20261013T182430.029Z-6db885`,7 cases:5 bodies passed (maximum
  decode/import, distinct-root copies, reversed local alias chain with exact absence and cold binding
  admission). Facts artifact and backup cases exposed real lifecycle/selection defects: the artifact
  outlived Workspace but lost its original cancellation owners; publication supplied binding-key
  order to a reader requiring a sorted unique selector set. Both diagnoses were source-reviewed.
  Enclosing fixture failed/native Unknown, attachment `7abe9ac2-87fc-4b4d-84c6-98a0d40278d4`.
  Separate retained collision `20261013T182711.755Z-d6ac39` **passed**1 body plus enclosing fixture,
  normal release selection. Checked campaign c disposal `20261013T182859.683Z-1649e2` **passed**;
  logical drain independently failed. Corrected test-target build `20261013T182002.394Z-31b139`
  **passed**, following the test-only EntityRef encode correction.
- **Partial/failed** narrowed d `20261013T183404.821Z-269b5e`,9 cases:8 bodies passed, including
  actual Facts artifact export/independent admission, exact originals, wrong-receipt/tamper refusal,
  retained collision and all previous primitives. Backup stopped before backup at publication
  inventory construction: published preparation incorrectly closed a borrowed transport view of
  the parent reader/pin. It now drains only its own operations and leaves shared closure to the
  enclosing owner; unconfirmed results retain the parent's Unknown. Independent review accepted
  that correction. The tuple-only runtime retention in this revision prevented cancellation but
  could not finish the original bridge/signal after Workspace disappeared. Its pure test manually
  drained weak references, so that pass does not close production finality. The accepted correction
  now extracts the complete existing execution owner, including provider/cache obligations, and exposes
  explicit two-phase finalization through the admitted artifact without retaining the read catalog.
  Neither export nor a borrowed seal automatically finishes that lifetime. Enclosing fixture remained
  failed/native Unknown, attachment `cc71f7bb-171a-43da-ac23-f161116458a5`.
  Owned d installation `20261013T183011.544Z-0aaeb6` **passed**; checked disposal
  `20261013T184543.345Z-87743f` **passed** with logical drain independently failed.
- **Passed** current bridge compile `20261013T190729.990Z-4dfc58`,
  `cargo check --locked --release -p lctx-semantics`, local cleanup confirmed. Independent source
  review accepts the shared execution-owner extraction and both borrowed-reader corrections.
  The replacement production finalizer control and actual Facts callers are awaiting their focused
  rerun; the earlier manual weak-reference control is not substituted for that evidence.
  Owned e installation `20261013T185539.619Z-5d8cec` was **blocked75** before service effects by the
  absent local CLI after build cleanup. CLI build `20261013T190028.381Z-78f37a` then **failed** on
  missing borrowed-preparation result type annotations; those annotations are corrected. Current
  rebuild and test-target preparation use normal concurrency and unchanged bounds.
- **Passed** current CLI build `20261013T190833.578Z-774eab` and selected test-target build
  `20261013T191506.274Z-be7d4a`, local cleanup confirmed. Owned e installation
  `20261013T191652.226Z-ca9685` **passed**, exact target `2214dfcc-3d93-49af-aa32-49388d3485e1`.
  Focused `20261013T191805.817Z-7f76ef` was **partial/failed**:8/9 bodies passed, including the
  production shared-owner finalizer, actual Facts artifact independent admission/originals and every
  selected copy/alias/binding/maximum primitive. Backup reached its own SearchOccurrences selection
  and failed on binding-key order/duplicate view requests. That consumer now sorts/deduplicates its
  selector set without changing the full binding inventory; independent source review accepts both
  nearby selection calls. Enclosing fixture failed/native Unknown, attachment
  `a51a08de-1eb5-4462-ba07-9685433f11df`; host cleanup confirmed/no survivors. Checked e disposal
  `20261013T192602.369Z-130287` **passed**, including logical drainage in both databases and all
  executor/datastore/attachment/installer/unit/storage/credential steps. This settles its target
  obligations without rewriting the original failed/Unknown receipt.
- **Passed** corrected backup test-target build `20261013T192620.152Z-14a765`, local cleanup
  confirmed, and owned f installation `20261013T192630.245Z-c6ea23`, exact target
  `e0a33b16-4d42-4094-b78e-c9a4a9c8ba62`. Backup rerun `20261013T192728.717Z-9bef79`
  **failed** after successful backup and publication retirement, before cold restore: source
  contributions/memberships remained after publication retirement. The initial warm-product
  explanation was a source lead, subsequently overturned below; an empty `remaining` list does
  not imply an empty `retained` list. Shared execution drainage closes sessions, not unrelated
  retention or every nominated collection job. The cold control retains strict source-row absence
  and is being corrected around the exact source ownership/nominations. No production retention
  policy or benchmark premise changes. Enclosing fixture
  failed/native Unknown, attachment `c419cf9d-ebde-4fbf-8698-21fd332fb114`; host cleanup confirmed.
  Checked f disposal `20261013T193429.319Z-4a7954` **passed**, including both logical drains and all
  physical target/attachment/credential steps; original failed/Unknown receipts remain unchanged.
- **Passed** cold-premise test-target build `20261013T193712.184Z-34cd23`. The initial source review
  accepted its exact product-release mechanism but did not establish that Facts creates products.
  Runtime `20261013T193741.794Z-45c683` **failed** on an empty product inventory; cold restore was
  not reached. Source reinspection confirms Facts does not arm product capture. The warm-owner
  explanation and that helper are withdrawn, not treated as an implementation defect or a reason
  to change production retention. Enclosing fixture failed/native Unknown, attachment
  `bbb911ea-95d4-4e0b-b321-088e8d2dc651`; local cleanup confirmed/no survivors. Owned g installation
  `20261013T193503.367Z-015c2c` **passed**, target `846bad4f-f1fa-467f-986a-429a62e0973b`;
  its blocked disposal remains an explicit obligation below.
- Owned g diagnostic `20261013T194424.092Z-62296c` was **blocked75** before diagnostic execution
  by the unreleased attachment, leaving ordinary maintenance closed. Exact named recovery
  `20261013T194527.981Z-f70df0` was **blocked75** by the required reviewed registered-drain server
  recipe; the borrowed405 generation is not that recipe. Checked disposal
  `20261013T194748.053Z-f8fabf` was **blocked75** by the unresolved ordinary maintenance marker.
  All three commands have confirmed local cleanup. No SQL diagnostic ran; actual remaining holder
  rows have not been observed. No guard, server recipe, marker or original Unknown was bypassed.
  Host status at closeout confirms its daemon is active, maintenance=true, development_workers=[],
  and attachment `bbb911ea-95d4-4e0b-b321-088e8d2dc651` protected. Its service/datastore remain
  protected with admission held closed; host observation grants no database inspection.
- The next cold-test hypothesis is intermediate source view roots: terminal attempt cleanup
  nominates exact outgoing roots but does not advance every collection job, whereas publication
  retirement traverses its own closure. The replacement test captures the source's held roots and
  incarnations, requires each untouched existing job's complete original root set and digest to
  match that evidence, and advances only those jobs. Epoch is a filter, never uniqueness or release
  authority. Incoming-holder diagnostics remain on failure, and strict source absence, independent
  dual restoration, fresh ownership, originals and no-selection assertions remain unchanged.
  Initial replacement build `20261013T195344.103Z-a81169` **passed**. Independent review then found
  that publication adds staging/marker roots after the initial capture; the final small augmentation
  validates exact published marker/staging roots against source identity and incarnations. Independent
  final source review accepts it; its runtime is **not_run** at operator closeout.
- **Passed** final frozen helper compile `20261013T195902.726Z-b2e940`,
  `cargo nextest run --locked --release --no-run -p lctx-publisher --test publication -E
  'test(facts_backup_cold_concurrent_restores_preserve_originals_and_fresh_ownership)'`;
  local cleanup confirmed/no survivors. This compiles the final late-root augmentation, not a test
  execution. All subagent work is frozen; no new native journey or quality campaign follows.

**Closeout / resumption boundary.** No native journey is launched after the operator's closeout
request. The helper and subagent source work are frozen. Owned h installation
`20261013T195514.888Z-29d2a7` passed (target `2146dff7-43f8-4936-aeac-88207f379548`);
no native test ran on h, and checked disposal `20261013T195745.242Z-91da95` passed every step,
including both logical drains. No worktree exists outside main. CU01 cold restoration remains open; successful backup and8 focused
component bodies do not replace it. On resumption, first reconcile owned g through an authenticated
supported recovery/disposal route. A dead ordinary marker alone cannot prove predecessor child
drainage; the managed diagnostic's direct child differs from the marker's descendant owner, and
the persisted record does not supply that exact binding. Do not loosen SOURCE_JOIN or remove the
marker manually. Then exercise the corrected cold control on an explicitly owned target and record
actual remaining holders if it still fails. Catalog analysis/admission/publication/query CE01,
CE02 regeneration and CE03 older-plan reconciliation remain separately open.

Scoped `just turn-end --paths …` completed ADR-index generation and formatting; build-features was
skipped because no Cargo manifest changed. This is housekeeping, not quality qualification. Final
`just adr-lint` still **failed** on14 pre-existing architectural-reference/decision mismatches;
ADR-0166's affected reader Decision link was corrected, with no remaining166 diagnostic. Unrelated
reference repairs and Ruff advisories remain outside this closeout. No broad checks were started.

Publication convergence independently validates incoming and winning eligibility but retains one
immutable winning root. Unselected losing staging is nominated by checked attempt closure; it is not
additional winning selector authority or a reason to retain needless historical copies. The old
publication→contribution freshness query no longer represents this physical contract. Its replacement
observes fresh receiving attempts through the exact incoming capture discriminator, separately from
winning published inventory and without weakening two-attempt/epoch assertions. Narrow Facts controls
isolate CU01/CU03 behavior from unrelated normalization; they cannot establish enclosing CE01.

All cited managed receipts/logs retain `citation:catalog-execution-patterns-target`. The existing run
record owns the full exact command and cleanup axes; there is no duplicate raw evidence folder.
Broad qualification/quality campaigns, CE02 regeneration and CE03 reconciliation remain **not_run**
by this slice. Their obligations are not closed by source review, a component pass or target discard.

### CP00–CP04 receipts preceding CU execution, 2026-10-13

Managed run IDs retain the exact command, output and independent local cleanup outcome. Current
checks use release, normal available parallelism and unchanged test bounds. Source review is not
runtime qualification.

- **Passed compile:** `20261013T040435.425Z-7ae3dd` checks compiler-store, cpg-core, publisher and
  serving; `20261013T043006.925Z-1c07bb` checks cpg-core/publisher/serving test targets after affected
  consumer migration. Current native test compile initially failed on an obsolete selector-record
  test call (`20261013T042537.646Z-a3449b`); correction is being verified.
- **Passed current CLI build:** `20261013T041034.593Z-7103d7`; subsequent schema index correction
  replacement `20261013T042716.197Z-b92741` passed after the schema index correction.
  **Passed owned installation:** `20261013T043952.231Z-e72383`, fresh target
  `c7f9cd78-b6bf-4063-b2a1-53490eb52c36`, schema7; this target was subsequently checked-disposed.
- **Passed host seams:** `20261013T043158.513Z-f0ecc0`, 33 selected service/development controls for
  schema, installer handoff, independent inspection outcomes and worker provenance; local cleanup
  confirmed. **Passed CP01 ordered/registered-worker:** `20261013T042638.616Z-2fc789`, 36 tests,
  cleanup confirmed. **Passed generator:** `20261013T042716.377Z-b56804`; post-generation worker
  rebuild/freshness and focused tool controls are in progress.
- **Passed pure selector correction:** CP03's `cargo nextest run --release -p lctx-native --lib
  -E 'test(alias_topology::tests) | test(selection::tests)'`, 25 passed, run
  `8130f56e-f145-446a-8009-999bb8f9f562`. Both cross-selector witness/chain regressions were included;
  this is not native persistence evidence.
- **Passed current native pure foundations:** `cargo nextest run --release -p lctx-native --lib
  -E 'test(compound_controls) | test(parent_hold_controls) | test(family_dependency_controls) |
  test(shared_preparation_controls) | test(installed_view::earned::tests) | test(retained_pin_controls)'`,
  9 passed, run `ff6d9108-b587-4310-aadd-7e7d69d8ff30`. Covers final inventory/factory source and
  retained-pin construction/Drop; native persistence controls remain separate.
- **Passed actual index inspection:** `20261013T044240.874Z-9dac0e`, installed contribution_rows
  and contribution_nodes definitions and plain EXPLAIN IndexScan selection. No workload timing.
  **Blocked sample setup:** `20261013T044240.388Z-e6dc84`, exit75, stale embedded worker assets
  immediately after generation; repair is the queued worker rebuild before sample retry. No test
  report or native body pass follows from setup refusal.
- **Failed owned installation:** `20261013T042421.157Z-17673e`, new target
  `dfea362f-f911-462b-835f-1b9be8bf579a`, schema7 index declaration refused implicit `id`. New
  completion indexes now use declared compound fields; query EOF/id correspondence is retained.
  **Passed checked disposal:** `20261013T042706.728Z-062dea` confirmed exact executor shutdown,
  datastore/attachment/installer/unit/storage/credential disposal. Original init Unknown remains
  unchanged. The shared target was not a fallback.
- **Initial checkpoint not_run, superseded by integration receipts below:** native attachment/
  retention/window/cold controls, generated-definition readback, Catalog generated analysis/
  admission/unselected publication/linked query/evidence/original bytes and facts availability.
  Broad quality, full qualification,
  real-library, embedding and MCP runtime remain excluded from this authorized slice.

### Current integration corrections and rerun boundary

The first actual native contract run `20261013T044049.896Z-74e929` ran19 controls:15 passed and4
failed; enclosing fixture failed/retained native Unknown. The six actual successes include bounded
record/guard/hold windows, stale-CAS refusal, exact alias/sparse absence, dependency hold refusal and
source-abandon-before-read lifetime. The failures exposed legitimate retained source revision zero,
an unnecessary64MiB aggregate-freeze demand in the8MiB live-attachment control, and nested shared
cause matching in acknowledged cleanup. These were corrected without increasing budgets. Retry
`20261013T045416.286Z-49dd08` passed3/4 (both retention cases and empty refusal), then actual
attachment exposed SurrealDB's FOR UPDATE target-expression restriction. Simple RecordId LET
lowering now preserves the same single atomic receiving effect; final actual rerun is pending.

The first composition runs did not pass: Catalog `20261013T044704.012Z-13b897` completed providers,
entity and relation normalization, then timed out at callable normalization under its unchanged360s
allowance. Analysis/publication/native-query assertions were not_run. Facts
`20261013T044704.460Z-0cb928` reached initial Catalog availability/memo/profile-refusal assertions,
then timed out during the second Behavioral availability audit; later mutation/corruption assertions
remain unproven. Cold `20261013T044704.926Z-d096e1` reached source admission and freeze, then refused
cross-pool audit reuse at export before independent destination assertions. Source-led corrections
batch semantic declaration effects and share phase-local physical observations; separately budgeted
explicit boundaries now earn fresh audits rather than consume foreign cached preparations. These
changes are independently source-reviewed. No elapsed-time ratio or dominant-cost claim is made.

Current affected test-target compilation `20261013T050649.323Z-adb07e` and final CLI build
`20261013T050945.007Z-0fdcde` passed. The broad-target no-run build
`20261013T050945.223Z-72eb87` was cancelled while queued, cleanup confirmed, replaced by explicitly
selected integration targets `20261013T051017.945Z-3c625e`; this is not a test failure or pass.

Fresh owned schema7 installation `20261013T051505.993Z-5562bb` passed for campaign
`3966ee5b-70cc-4460-8039-04c6138ab26f`. Native correction run
`20261013T052042.405Z-f7bf35` ran21 controls:20 passed, including actual atomic attachment,
retained product replay and native windows;1 failed. The added maximum projection control
`projection_backing_preserves_typed_keys_payloads_and_cold_state` failed before ingestion at
`write_batch`: complete expanded request60,891,911 bytes exceeded the unchanged8,388,608-byte
limit. The maximum chunk and request-limit mismatch also exists in the baseline; this is an open
native transfer/CU03 investigation, not a successful cold transport. Its payload and assertion
remain intact. A separate small binary-payload control tests the8MiB scan→256MiB independent
finalization/export/import boundary; its result cannot qualify the maximum-size case. Local
process cleanup was confirmed; fixture attachment `42400cbf-07ac-4443-88ef-4ecde395dd5d` retains
native Unknown until checked owned-target disposal. No shared target is borrowed as fallback.

Current touched native/compiler/CLI test-target compilation
`20261013T052718.876Z-66296e` passed before the final packet-accounting refinements. Independent
review accepted the semantic declaration writer's lazy charged inputs, byte/count windows and
separate retained descriptor/raw-record reservations. Final compound-child writes now split against
complete prepared requests, including native authorization and object expectations, with
serialization admission preceding allocation. Independent review also exposed and accepted the
correction for a large-child/tiny-tail refusal: the writer holds preparation workspace while accepting
children and reuses it during flush, including across split siblings. Shared pure authorization lowering
preserves the original request/intent/outcome route; preflighted requests execute unchanged. The
complete-inventory final CAS and independently necessary contributor holds remain required.
Selected test-target build `20261013T055548.743Z-d154d5` passed; it predates final headroom source.
The next native run `20261013T055859.733Z-a2db5c` ran49 controls:48 passed, including final
packet/headroom controls; the small projection case failed at destination `completed_state` because
import does not freeze its receiving attempt. The control now explicitly freezes that destination;
source export/finalization and byte round trip are not inferred from the earlier failure. Facts
`20261013T055859.714Z-1abb91` passed both profile-availability and memo assertions, then its manually
constructed invalid fixture stopped at a missing `uniform_not_requested` binding before the intended
undeclared-producer refusal. The fixture now declares that required tenth input empty, retaining both
original refusal assertions. This prerequisite already existed in the baseline.

Cold `20261013T055859.718Z-40d526` failed at export under the unchanged256MiB budget: repeated
per-view closure observations retained full native rows and typed descriptors. Catalog
`20261013T055859.709Z-10575f` timed out during callable-aspect normalization after entity, relation,
callable and receiver normalization completed. Analysis/admission/publication/query assertions were
not_run. All four local workers drained; their native attachments retained Unknown. Source inspection
found the same repeated metadata/dependency discovery preceding the already reusable contributor
preparation; no elapsed-time attribution or ratio is claimed.

Corrections are source-integrated and independently reviewed: exact charged physical observations
and typed descriptors are shared within a fresh boundary phase; operation-owned internal selections
reuse metadata only after actual exact receiving CAS under the operation's pin/context. Each selector
still independently validates its traversal, mappings, membership and alias closure. Required-edge
reuse relies on protected immutable endpoints and retirement's incoming-hold exclusion. Independent
boundary phases start empty. Reader setup deduplicates opaque contributor capabilities before any
effect, refuses conflicting bytes/incarnations, receives count/complete-byte bounded exact CAS pages,
and checks the union's current context, rows and source holds. Retained classes preserve their own
parent/selector/admission/publication checks. Providers use their already held exact capability,
correlated with the store registry, and freshly check current authority before every delayed scan;
consumer pool charges remain distinct from the source's original charges. Terminal recovery uses one
fresh inspection phase and the same independently earned prepared membership recipe, with unique
charged retained descriptors. The obsolete production membership-rescan helper is removed.

Touched test-target compile `20261013T064546.960Z-0c618b` failed on one missed private import-scan
argument; corrected compile `20261013T064832.970Z-65250f` passed. CLI build
`20261013T065203.536Z-6d1386` passed. Checked disposal `20261013T065203.537Z-02cd5a` passed
exact executor/datastore/attachment/installer/unit/storage/credential disposal for campaign
`3966ee5b-70cc-4460-8039-04c6138ab26f`; logical drain remained failed. Original Unknown and failed
outcomes are unchanged. Those original native outcomes are preserved; latest reruns use the separately owned r4 generation
recorded below. Shared/operator and old paused targets remain untouched.

Generated freshness `20261013T044054.292Z-56dda9` and six worker controls
`20261013T044755.261Z-95b1f3` passed. Rebuilt sample retry `20261013T045024.659Z-2ca717`, nested
worker `20261013T045025.642Z-bd80eb`, passed1 suite/6 cases with report capture, child cleanup and
credential cleanup passed; remote certainty stayed unknown until checked target disposal.
`20261013T050944.793Z-ebdb98` passed exact executor/datastore/attachment/installer/unit/storage/
credential disposal of `c7f9cd78-b6bf-4063-b2a1-53490eb52c36`; failed logical native drain remains
failed, and original native outcomes stay unchanged. Current reruns use a newly owned generation,
not shared/operator state. New cited managed receipts/logs retain
`citation:catalog-execution-patterns-target`; release only after the last current citation retires.

### Inherited receipts and their limits

These are prior executions, not checks run during authoring. Exact managed run IDs identify receipts
and original commands through `just runs status RUN --json` / `just runs logs RUN`; use verification
reports for case/capture/cleanup details. No rerun is authorized merely by citing one.

- **Passed scoped audit/recapture:** `20261012T192358.901Z-bb8644`, eight compiler-view controls
  `20261012T213654.901Z-9e4094`, late-terminal control `20261012T222225.607Z-540ad4`.
  These support lifecycle#F02/F05, not current whole-track acceptance.
- **Passed durable diagnostics:** `20261012T192108.721Z-ec09c9` and
  `20261012T192108.926Z-bbaff5` retain actual writer/host scopes; deliberate committed/failure body
  `20261012T192109.212Z-1915fa` had failed enclosing fixture/Unknown, independently settled for
  host disposal by `20261012T224717.024Z-31ece6` / `20261012T225024.896Z-e30ded`.
- **Passed cold-batch:** `20261013T004845.788Z-a53c24`, body plus enclosing cleanup, exact23-edge
  physical chain and strict refusal. **Limited passed bodies:** `20261013T004135.077Z-383cbb`
  late-terminal/fragment retry and unchanged64MiB receiver; enclosing fixture failed/Unknown.
  **Passed compile only:** `20261013T004041.509Z-72f7c3`; no downstream acceptance follows.
- **Failed latest Catalog:** `20261013T004735.484Z-76f1cf`, existing360s allowance; providers and
  entity normalization completed, relation normalization continued until cancellation. Analysis,
  admission/publication and linked-query assertions were not_run. **Failed facts:**
  `20261013T004857.554Z-e28d61`, existing allowance before availability assertions. Local cleanup
  confirmed; original native Unknown records preserved. No measured ratio/dominant-time verdict.
- **Passed owned disposal:** `20261013T005654.382Z-0148c9` / preservation postcheck
  `20261013T005805.779Z-df05d6`, and final unused target `20261013T011359.080Z-c6fed4` /
  `20261013T011453.931Z-d5e7e5`. All newly owned targets are disposed; original native outcomes and
  old paused/shared/operator state remain unchanged. OP02's actual root/record/token, Sync,
  required-definition, declarative-report and recovery evidence stays scoped to its earlier receipts.

Earlier broad qualification was interrupted/failed with mixed compiler/provider/tooling/quality
outcomes. Those results remain in source reviews/evidence and Git; they are neither current passes
nor this track's repair backlog. Lifecycle receipts retain their existing storage consumer `citation:surrealdb-lifecycle-foundations`;
the Catalog evidence uses `citation:catalog-execution-patterns-target`. Release either only when
its last current citation retires; no duplicate raw evidence tree or new retention registry is created. Unknown native records have independent owners.

### Plan-publication evidence

Independent read-only focused plan advice found no material architectural contradiction or missing
obligation across all 8 Catalog findings, 5 lifecycle findings, 18 predecessor packages and C01–C11.
The advised IF04–IF09 provenance correction is incorporated; follow-up advice confirms that validated
selectors plus protected boundaries preserve ADR-0152's retained view-local realization guarantees. This is planning advice, not CP00
interface acceptance or integrated subsystem qualification. Authoring checks are recorded below. Product builds/tests,
services, fixtures, schema changes, real-library compiles, benchmarks and activation are **not_run**:
this turn writes documents only. Scoped housekeeping and navigation checks do not establish design
implementation, generated-analysis acceptance or whole-product quality.


- **Passed:** `python3 /tmp/check-catalog-plan-docs.py`, focused new/changed document links/fragments,
  all source finding/package/capability IDs, inherited full receipt IDs, ten unchanged reviewed source
  hashes and STATUS's 120-line bound. This is navigation/coverage evidence, not semantic certification.
- **Passed:** `git diff --check`; no whitespace errors in tracked document changes.
- **Failed:** `just adr-lint` and `just docs-check`, the same 14 existing ADR/architectural-owner
  mismatches in ADR-0152/0155/0156/0157/0158/0160/0161 and ADR-0151's decision link. No diagnostic
  names ADR-0165. Full publication checking stopped before later stages. These unrelated metadata
  repairs remain outside scope; no accepted predecessor body or checker was rewritten to pass.
- **Not_run:** product/runtime checks and broad repository-quality validation, by the authoring scope
  and operator boundary above. End-of-turn housekeeping is scoped to this document slice.

### Latest focused continuation, 2026-10-13

Corrected CLI build `20261013T065203.536Z-6d1386` and selected test-target no-run build
`20261013T065834.052Z-0d14c6` passed. Owned schema7 installation
`20261013T065833.880Z-9404b9` passed for campaign `69560696-1e27-4474-82c3-10481831c3fa`
(`catalog-cp00-04-20261013-r4`); exact disposal remains required after workers drain.

Facts availability `20261013T070050.432Z-0e9394` **passed**, including both profiles, memo reuse,
failed-replacement preservation, repeated undeclared-producer refusal and enclosing fixture cleanup;
local cleanup confirmed07:07:42UTC/no survivors. Command: `just fixture -- cargo nextest run --release
--locked -p cpg-core --test facts_admission -E
'test(=availability_reuses_exact_admission_and_refuses_changed_invalid_membership)'` on that owned
campaign. This does not close generated-analysis composition.

Native run `20261013T070050.327Z-936df9` ran56 controls:52 body passed,4 failed; enclosing fixture
failed/native Unknown attachment `3f40a574-6997-4865-8dca-08b088b0844a`. Corrections preserve the
contracts: pure detached pin control now owns a Tokio runtime; corruption injected outside the native
protocol is independently reselected under a fresh audit; guard-revision replay starts after required
first metadata pins; diagnostic backing reads use their actual caller budget and close their reader.
Independent review accepted the two fixture adaptations without weakening production checks.

Cold `20261013T070050.374Z-316821` passed export under the unchanged256MiB budget and destination
import, then failed restored facts acquisition at an omitted contributor→canonical-endpoint hold.
Original restored equivalence assertions were not reached. Attachment
`818f0e23-3af2-4205-a379-76b244dc96fb` retains native Unknown. Import now reconstructs exact endpoints
through the same charged canonical payload validator as native assurance and installs fresh contributor
ownership in bounded windows; current verification is pending.

Catalog `20261013T070050.286Z-e9d612` timed out during projection normalization under unchanged360s;
providers, entity/relation/callable/receiver/callable-aspect/event/binding normalization completed.
Generated analysis/admission/publication/query assertions remain not_run. Local cleanup confirmed
07:11:44UTC/no survivors; native Unknown attachment `def8e03f-9d65-4eb7-bf21-93685adde019` is protected.
Independent source diagnosis found fresh metadata discovery repeated across every view in retained
and final physical-boundary inventories. Both now share one phase-local reader/charged observation
map, preserving each actual domain traversal, exact mappings and independent boundary assurance.
Touched native/compiler/publisher test compile `20261013T072235.374Z-e979dd` passed after endpoint
validator extraction and phase reuse; later native-window corrections need their own verification.

**New open CP04/CU03 obligation:** portable aliases carry only union rows and lose contributor→alias
incidence. Reconstructing ownership from a shared source, or unioning differing equivalent physical
copies, can widen selector eligibility. Nonempty alias import now explicitly refuses instead of creating
the wrong source→alias edge. Exact transport needs an independently bound owner-incidence carrier
and an explicit physical-envelope format, or a revised semantic identity. A self-declared carrier digest
cannot establish the independently expected v4 identity; identical logical contributors with differing
physical alias incidence must not silently merge. Keep v4 semantic roots unchanged pending that
bounded decision; no full cold-alias closure or full CP04 completion is claimed. Selected dump/backup
must preserve the same incidence independently. Native selector/alias behavior is separate from this
portable-state gap. Maximum opaque-chunk ingress remains its separately recorded open investigation.

Final fresh-window source and parent→contributor exact hold pages received independent static review
with no concrete blocker. Fresh phases now batch all selected physical/view observations across the
complete inventory, including empty dependency views, and heterogeneous required holds. Parent-copy
pages retain original frozen/live guards and full completed-row/incarnation checks before any page
hold, split by actual prepared request bytes, and leave final child inventory EOF/parent CAS intact.
Selected no-run build `20261013T072342.364Z-cfbd85` and touched test compile
`20261013T072635.926Z-fd1e2c` passed; they predate the final parent-copy page edit.

Current native `20261013T073145.486Z-a6f7fa` and Catalog `20261013T073145.809Z-eb4d45` are running
against the final source. Cold `20261013T073145.648Z-d9fda3` reached restored facts equivalence,
exact completed descriptors and exact binding inventory, then failed at frozen-state identity because
the destination was still mutable. Its fixture now explicitly freezes the independently imported
destination before that existing assertion; no equivalence assertion was removed. Retry
`20261013T073749.661Z-6afac1` is running. Original failed body/fixture remains failed; native Unknown
attachment `2e4be72b-6b53-42cd-8694-fd69b0ba2c07` is retained until checked whole-target disposal.
The alias transport identity choice was presented to the operator; it does not block these independent
functional checks. No completed CP04 or full-plan claim follows from the interim refusal.

Native `20261013T073145.486Z-a6f7fa` ran56 controls:55 body passed,1 failed; final fresh observation,
receiving and parent-copy page implementations compiled and exercised. Enclosing fixture remains
failed/native Unknown attachment `b1201ba7-a7b1-4411-acbf-8b28f20b7a2f`. Remaining failure was the
privileged corruption fixture's restoration comparing entire native guards after a fresh incoming pin
legitimately changed revision. Restoration now checks the exact edge and endpoint active phase,
incarnation and retirement fields; ordinary guard revision may change. The exact missing-hold refusal,
restored reselection and both audit closures remain. Only that changed case is rerunning in
`20261013T074327.059Z-0c6cc1`; earlier failed outcomes are unchanged.

Focused corrected retry `20261013T074827.694Z-c6df84` **passed2/2 plus enclosing fixture cleanup**,
nextest `7578ab7a-e550-4576-b930-61a730905222`, local cleanup confirmed07:49:17UTC/no survivors.
The exact shared-dependency refusal/repair uses `ModelError::primary` to preserve the shared worker
failure wrapper. Cold admission/transport/frozen identity and every original descriptor/binding/view
comparison passed under unchanged256MiB. Its invalid-header subcase now owns only the actual native
ingress component, not an unused semantic workspace whose started owner receipt was never drained.
Previous cold retry `20261013T073749.661Z-6afac1` passed the full body but failed enclosing fixture
cleanup; native Unknown `43b3aa3d-a063-465b-8b35-90ae72f16d63` remains original evidence. Previous
single-case `20261013T074327.059Z-0c6cc1` restored the edge and rejected correctly but failed its
unwrapped-error pattern; no production check changed for the successful retry.

Catalog `20261013T073145.809Z-eb4d45` **timed out** under unchanged360s, nextest
`e9cb5a60-f6d0-4ec3-a812-5028cc429d29`. All normalization and normalized admission completed;
analysis configuration stages completed, then analysis native inventory ingress was cancelled.
Admission/unselected publication/native queries/evidence/original-byte assertions remain not_run.
Local cleanup confirmed07:50:42UTC/no survivors; native Unknown
`27303c6d-0078-470e-9484-fb9fc45ea0fb` is protected. Current-phase source diagnosis precedes another
retry; this is not a design-speed ratio or a reason to raise bounds. Owned r4 checked disposal is
running in `20261013T075307.026Z-55f7a2` after all root functional workers drained. Exact native
executor/target disposal, logical drainage and original failed outcomes remain separate.

Checked r4 disposal `20261013T075307.026Z-55f7a2` **passed** exact whole-executor/target disposal;
its logical native drain remained failed. Original Unknown receipts and failed runs are unchanged;
confirmed executor discard settles only that owned target's remote obligations. Fresh r5 schema7
installation `20261013T075951.598Z-fee2f7` passed; exact identity is in its managed installation receipt.
No shared/operator/old paused state was used.

Latest source diagnosis located repeated whole-closure record/guard/hold reads in same-store cached
view acquisition, provider execution and sparse member access before normalized/analysis consumers.
These now borrow actual immutable opaque premises beneath their real live pin and check current
installation generation/validity; independent receiving/full assurance remains unchanged. Product
absence discovery no longer allocates a receiving pin merely to return no reuse. Touched test compile
`20261013T075753.758Z-4b9ed9` passed before final replay/budget edits. Focused combined functional run
`20261013T080134.180Z-c2a406` is building/running on r5 with the same selected native controls plus
exact empty/source-close, facts availability, cold and Catalog composition controls. This consolidates
one dependency-feature build and keeps normal parallelism and unchanged budgets/deadlines; it does
not broaden into unfiltered families. Previous independent static reviews support the earlier window/
retention/import changes; independent review of the newest borrowing seam was unavailable because
reviewer model capacity rejected the attempts. Root source reasoning and runtime outcomes remain
explicitly distinct from independent acceptance.

R5 is `aa85313a-5a40-4a51-a395-59b427eb2b68`, schema7, exclusively owned. Combined
`20261013T080134.180Z-c2a406` compiled and ran64 selected controls: **63 body passed,1 Catalog timed
out**, nextest `b0717956-66ae-46f6-96f7-2ed3c3768170`. Facts availability/refusal, cold identity/
admission, provider/cache and exact empty/reader-source lifetime controls passed. The added actual
borrow check passed while the source was live and refused after source close; separately receiving
readers still returned exact bytes. No nonempty-alias transport claim follows. Local cleanup confirmed
08:12:41UTC/no survivors; enclosing fixture failed/native Unknown
`2d437603-6f2a-48ac-9c5a-47a271657508`. Catalog again completed normalization/normalized admission
and reached analysis native inventory before cancellation; there is no performance-ratio attribution.

Source-level reductions in repeated metadata work are supported by these controls, but do not establish
the remaining timeout's dominant cause. Existing phase observations now additionally name actual
retained-product finalization and semantic-reference validation; selected invariant names are diagnostics,
not admission receipts. Compile `20261013T081755.940Z-f06916` passed. One Catalog-only diagnostic
retry `20261013T081814.039Z-499814` is running with the same dependency feature selection and
unchanged bounds; no other passed control is repeated without a code change. R5 disposal waits for
that worker and native executor drainage. Independent source acceptance of newest borrowing remains
not_run because reviewer model-capacity failures prevented that assignment; root reasoning plus
focused controls do not claim an independent review occurred.

Catalog attribution `20261013T081814.039Z-499814` **timed out** under the unchanged allowance;
nextest `e10b3db5-44be-42d0-9737-49178fd7d679`, native Unknown
`150003df-58bd-4fea-86f1-f93d806b5cc0`. Retained-product finalization occupied much of later
normalization phases; semantic-reference validation was a smaller separate observed interval.
Analysis native inventory reached its retention work before cancellation. These observations localize
work; they do not measure a comparative design ratio. Mapping discovery, selection assurance,
family union, child installation and inventory receiving now have coarse existing-owner phases to
separate that remaining work. No diagnostic emits an admission capability or hides failed finality.

Source inspection also found discarded already-validated selector member runs followed by native
membership/guard rereads for each sparse demand. Earned selectors now retain that private exact run
and a bounded point index beneath their actual source pin; typed results preserve selector, relation,
key, content and envelope correspondence. The existing ordered-run point cursor exposes one borrowed
decoded row, with original source ownership and independently charged consumer workspace. Retained/
independent native paths keep their previous full reads. Compile
`20261013T084442.421Z-9101a5` passed. Combined affected runtime/pure controls plus two revealing
memory/spill/exact-member controls are running in `20261013T084546.000Z-865dc5` on the same owned
r5 target; no budgets or deadlines changed. Independent fallback source review accepted these exact source seams on2026-10-13;
configured reviewer capacity was unavailable, so the coordinator model was used in a fresh context.
The reviewer ran no builds/services/tests and made no runtime or measured-performance claim. R5 remains owned and must be checked-disposed
after this run and native executor drainage; no shared state or historical data is discarded.


Prepared-member combined run `20261013T084546.000Z-865dc5` compiled and ran66 selected controls:
**65 body passed,1 Catalog timed out**, nextest `1124ed60-8a72-4f12-8132-bb77ead34b8b`.
The memory/spill point lookup, exact member envelope, source-close, cold admission and facts controls
passed. Local cleanup confirmed08:57:34UTC/no survivors; enclosing fixture failed/native Unknown
attachment `78dff7f6-cb88-40ea-8a31-36a82cb1d7b9` remains protected on owned r5.
Fine phases localize later normalization retention's dominant observed interval to canonical family
union (about21s within26–27s retention), rather than child installation or inventory receiving
(each below1s there). This is a single-run diagnostic, not a comparative benchmark claim.

Source inspection found the same protected contributor fragments entering the union once per
cumulative selector, and the same dependency view normalized repeatedly. All exact descriptor,
incarnation and view contradictions still reconcile first; family composition now adds each exact
physical contributor once and lowers each reconciled view once. Per-selector independent assurance,
complete family dependencies, canonical conflict/EOF checks and flatv4 hashing remain unchanged.
Compile `20261013T085913.185Z-bd8847` passed. An overlapping17-selector control compares all six
roots with independently reconstructed native cold roots. Catalog and family-dependency controls are running in `20261013T090002.628Z-055d40`, with unchanged
allowances and normal parallelism. Its exact-name filter does not select the namespaced late-terminal
control; that corrected uncached overlap test and literal identity controls still require their own
focused invocation. Independent follow-up source review accepted the reduction and identified the
previously warmed overlap assertion; the assertion now precedes the first successful scoped fold.
The reviewer ran no tests/builds/services. Original failed receipts remain failed.


Family-input reduction Catalog retry `20261013T090002.628Z-055d40` compiled and ran2 controls:
family-dependency control passed; Catalog timed out, nextest `72eed9f1-e747-4136-b382-20618882e6b8`.
Local cleanup confirmed09:09:52UTC/no survivors; native Unknown attachment
`0e1c52df-348b-4696-ae8f-088671ad9642` remains protected on r5. Fine family-union intervals were
small after input reduction; the journey progressed through further analysis and reached Catalog
evidence generation. This supports the diagnosed removed work, not a whole-journey speed or
comparative performance claim. Current normalized admission and later selection assurance remain
material intervals; admission/publication/queries are still not_run.

Corrected uncached overlap and literal v4 identity control `20261013T091030.105Z-f09622` **passed2/2
plus enclosing fixture**, nextest `6df38616-6be2-4aaa-9990-3abdeee9b241`, local cleanup confirmed/no
survivors. Build-only preparation `20261013T090530.065Z-bc8a8a` passed; the earlier retry's namespaced
exact filter did not select this control and is not its evidence.

The maximum projection ingress failure is a unit mismatch: `plan_bytes` estimates resident/native/
JSON coexistence (about60.9MB for the1MiB opaque chunk), not gRPC message length. The same native
protobuf encoder carries Bytes directly. Source now keeps that original workspace estimate for
resource admission and uses a separate complete QueryRequest encoded-length bound for transfer
preflight. The bound includes maximum-width native request context/batch fields and both final
payload and final intent before any remote intent. Canonical bytes, nominal IDs, chunk size and8MiB
transfer limit are unchanged. Protocol0.13.1 moves from dev-only to production; prost0.14.4 becomes
a direct exact dependency already present in the lockfile; no dependency versions re-resolved.
Compile `20261013T091821.094Z-84e90a` passed (before completed borrowed-reader integration); original maximum ingress and near-wire-bound /
pre-intent-refusal controls require current-source acceptance. Context7 did not cover the exact SDK
QueryRequest seam; pinned generated protocol and patched SDK source own that contract, with
prost0.14.4 Message documentation checked separately. No new schema or semantic identity is implied.

A separate private same-operation canonical-reader route is being implemented. Public independent
receiving readers keep their own pins/session and source-abandon survival. Only exact same-store
Earned source capabilities beneath admitted store setup/scan leases may borrow source pins/session;
source closure refuses future work. The small guard must follow mechanical preparation and every
raw stream, preserve Unknown, avoid reader/cache cycles and never release source pins/invalidate the
shared store session when the borrowed reader closes. Current epoch advancement requires complete
native-pin drainage, preserving observed context under live source pins. This is source work, not
runtime acceptance; the current owner remains open until lifetime controls and Catalog composition.


Borrowed canonical-reader source is integrated (2026-10-13). A private store helper requires its
actual admitted parent and exact registry Arc correspondence, borrows only all-Earned selections,
and falls back to independent receiving for retained/mixed sets. The small pin/context guard follows
mechanical preparation and reader operations without a scope/cache cycle. Borrowed close drains its
own operations and leaves source pins/session owned by the source. Public independent-reader behavior
is unchanged. The lifetime control now exercises borrowed close, continued source scans, cached/new
query refusal after source abandonment, and independent-reader exact-byte survival. Builds/runtime
were not_run by the worker. Independent wire admission review accepted exact protobuf coverage,
conservative context fields, pre-intent placement and separate memory reservations; independent
borrowed-reader source review is in progress. Combined affected source/runtime checks are building in
`20261013T120839.170Z-ef6831` on owned r5, including the original maximum projection payload,
wire/refusal controls, uncached overlap, source lifetime, facts, cold admission and Catalog composition.
No budgets/deadlines or normal available parallelism changed. Source/review acceptance alone does
not close CP04; r5 remains exclusively owned and awaits checked disposal after actual workers drain.


Independent borrowed-reader review accepted the ownership/context/cache separation and found one
delivery race: `NativeRows::next` checked a source pin only before awaiting a row. Root now retains
the small borrowed guard across the await and rechecks before exposing either row or EOF; failure
uses the existing remembered-error/drain owner. Review accepted that correction. A pure pending-read
control loses actual detached source-pin authority while awaiting and then supplies a row or terminal
EOF; both must refuse without losing drainage or consumer reservations. Current `ef6831` predates
this correction in at least some built targets and cannot establish the new control's result; its
separate focused run remains required after the current worker drains. No full-receiving/publication
check was weakened, and the ordinary private caller's admitted parent lease remains required.


Combined `20261013T120839.170Z-ef6831` compiled and ran70 controls: **68 bodies passed,1 maximum
projection failed,1 Catalog timed out**, nextest stored by that run. It exercises native source borrowing,
wire and final-wrapper refusal, cold admission, facts, literal roots and unchanged narrow native controls.
Local cleanup confirmed12:25:05UTC/no survivors; native Unknown attachment
`0782ad39-d4db-47fc-9542-e3948aa42b72` remains protected on r5. Catalog normalized admission completed
in its observed interval and progressed to Catalog evidence; required publication/query composition
remains unproven. Maximum ingress now passed actual write admission but failed cold installed-view
preparation: the8MiB scan pool cannot admit the167,930,976-byte conservative decode workspace.
That is not an actual168MB allocation measurement. The prepared canonical JSON frame itself is
about16MiB, so tightening the multiplier alone cannot establish cold8MiB capacity.

The approved CP00/CP02 contract separates source preparation from its consumer. Independent review
found no required cold8MiB maximum guarantee. Only the maximum fixture now explicitly earns its
source capability under its existing256MiB producer pool before the unchanged8MiB scan. All payload,
nominal-key, export/import and cold-state assertions remain. This establishes neither8MiB total
memory nor cold acquisition; the original failed observation remains visible. The small binary fixture
continues first acquisition under8MiB. This is owner/pool correspondence, not a new or raised budget.

The initial pure pending-delivery control `20261013T122415.709Z-f03a01` failed its assertion and then
aborted during unwinding; local cleanup was confirmed/no survivors and no native service was used.
Its assertion conflated sticky logical Conflict with unconfirmed physical drainage. The corrected
control checks the original pin Conflict, successful explicit physical drain, sticky subsequent refusal
and separately preserved source uncertainty preventing pin release. Compile
`20261013T122840.352Z-cf28b5` passed. No passing race result is inferred from the initial failure.

Repeated retained-phase context observations now borrow the actual fresh `ClosureInspectionScope`
after its current-context observation and exact receiving CAS. Private member/alias/preparation bodies
retain real pin/client/pool and full exact predicates. Public wrappers still observe context; final
live/frozen native completion fences current generation/validity. Independent source review accepted
this reduction; no portable or metadata-only authority is added. Combined affected controls including
the corrected race and maximum-source preparation are running in `20261013T123350.322Z-5fabe2`.
Its maximum case reached a further legitimate row-width mismatch: a full expected backing observation
used the1MiB ordinary stream default for a5,245,805-byte opaque/native row. Full record observation
and repeated exact readback now use the existing declared64MiB physical-row bound; guards remain
narrow and all resource/EOF/correspondence checks remain. That later correction is not exercised by
`5fabe2`; a focused maximum/observation retry is required. Original failed outcomes remain failed.

Combined `20261013T123350.322Z-5fabe2` finished with **69 bodies passed,1 maximum projection
failed,1 Catalog timed out** (71 selected). The corrected pending-row/EOF authority-loss control
passed. Local cleanup confirmed12:44:49UTC/no survivors; enclosing fixture failed with native Unknown
attachment `e20ed8ec-0f15-4ea3-9493-597acc63217f`, protected on r5. Catalog reached evidence ingress;
the cancellation includes a fenced-uncommitted transaction retry and does not establish publication
or linked queries. Neither body counts nor local cleanup settle its native obligations.

The full-row observation correction also exposed an adjacent unit mismatch in
`retained_row_charge`: its32×+64KiB workspace reservation was compared with the64MiB physical row
cap. Source now checks actual native row bytes against that cap and independently reserves the
unchanged workspace through the actual pool. No budget or row/wire cap is raised.

After all local alias/member/backing proofs, source now composes the complete alias run first and
observes its actual empty EOF beneath the audit pin. An empty run returns before composing membership
seeds or constructing reachability indexes; nonempty aliases retain the complete existing path and
outer exact-owner/finality checks. Independent source advice accepted this narrow correction; no
timing significance is claimed. The single-entry whole-set alias cache still recomposes alternating
sets; an exact-key operation-local cache is a possible later correction, not yet implemented.

Focused current-source maximum, observation, alias, lifetime-race and Catalog controls are building
in `20261013T125026.117Z-816e09` with normal concurrency and unchanged allowances on owned r5.
Its receipt/log are retained for `citation:catalog-execution-patterns-target` immediate use. Earlier
failed runs remain failed; r5 requires checked disposal after workers and native executors drain.

Focused `20261013T125026.117Z-816e09` compiled and selected5 controls: **3 bodies passed,1 maximum
import failed,1 Catalog timed out**; nextest `c7a2c700-ef72-4fdb-9151-45c9339d2a8f`. Observation/
stale-incarnation and borrowed-row/EOF controls passed. The filter's compiler-view/empty cases were
not selected because their targets/packages were omitted. Local cleanup confirmed12:59:48UTC/no
survivors; native Unknown attachment `82dadbe1-774a-4a13-9b8a-a3f8ed2bef8b` remains protected.
Catalog completed evidence generation and timed out during execution enrichment. Admission,
publication and linked queries remain unproven. The empty-alias correction is not a benchmark result.

Maximum import failed the outer full-state stream's1MiB default after actual source preparation,
scan and export succeeded. Full-state sources now preserve the declared64MiB physical-row bound
through both inner native streams and outer owned delivery. Expected hold observations also separate
their actual row cap from unchanged8×+4096 workspace reservation. Independent source review accepted
the row/workspace distinction. `20261013T125630.696Z-822fa3` then **passed1/1 plus enclosing fixture**,
nextest `39190872-d9e7-43a0-8881-35198436986a`; local cleanup confirmed12:58:26UTC/no survivors.
This is the original maximum payload/key/scan/export/fresh-import/identity control with source256MiB
and consumer8MiB as recorded above; its other filter terms were not selected. It closes this narrow
maximum semantic/transport failure, not the original cold8MiB capacity claim or full CP04.

Independent source inspection found a completed selected-member EOF/count/digest fold discarded
inside the existing registered work item and immediately recomputed by boundary earning. The private
prepared result now retains run/count/content together beneath the same actual pin and exact selector
key. Earning consumes that checked result; subsequent boundaries need no repeat fold. Failure publishes
no partial result, retained metadata is charged, and fresh receiving observations/CAS remain unchanged.
Source integration2026-10-13 requires the next focused current-source controls; no timing claim follows.

First-time logical backing preparation still used one record+guard observation per backing row.
It now uses the existing exact-ID primitive in charged pages of at most128, retaining required-ID
extraction, full row validation/normalization and complete record/guard EOF/drain. Independent review
identified a processing-capacity trap in acquisition-only page halving: sibling reservations could
poison the output sorter after successful acquisition. The integrated correction discards the entire
effect-free provisional preparation on direct local Resource, retries a smaller page, and finally
uses the original singleton path with unchanged capacity/refusal. Semantic/transport/combined-drain
errors do not retry. Source/context/reader-close/pin checks and success-only work insertion remain.
Independent source review accepted `4bcbad17` on2026-10-13; no runtime result is inferred.

Current-source focused controls are building in `20261013T151417.297Z-79e30d`, including the129-row
late-terminal/uncached overlap control, independent literal v4 identity, maximum import, exact native
record/guard/hold refusals, empty/dependency and nonempty alias selection, source lifetime, failed-work
eviction and Catalog composition. Budgets/deadlines/available concurrency are unchanged. Run receipt/
log retain the same immediate-use citation consumer. R5 remains exclusively owned until checked
worker/executor drainage and whole-target disposal; its prior native Unknown receipts stay unchanged.

Independent static publication/receiver review found two additional CP04 equivalence defects:
unchanged-publication discovery and both staging convergence paths validate only the winning
boundary, without comparing incoming selector-local alias eligibility; retained attachment can keep
an older same-logical-view mapping/capability after independently validating a different candidate.
The consumer companion records exact seams and required collision/concurrency obligations. Original
request replay and terminal recovery preserve their original exact boundaries. Catalog F02/F04 and
CU03 therefore remain open beyond cold framing; a first-publication pass cannot close these defects.
The operator identity question now explicitly includes these consumers, preserving the approved
RC03 restriction on a separately approved identity-version/consumer change. No unsafe-equivalence
correction or new identity direction is claimed implemented.

`79e30d` exposed a wrapper issue in the new resource fallback: owned streams share terminal local
errors through `SharedCause`, so a direct Resource match missed them. Source now unwraps only that
transparent wrapper before recognizing Resource; Completion, Cause and all other variants remain
terminal and opaque. A focused pure control rejects failed-drain/context/semantic wrappers, and
`20261013T152450.918Z-337db8` is building the corrected maximum,129-row preparation, alias/empty,
lifetime and exact-window controls. It excludes another unchanged Catalog retry. The original failed
maximum body remains failed; its final result and Catalog/fixture outcome belong to the existing run.

`20261013T151417.297Z-79e30d` finished12 selected controls: **10 bodies passed,1 maximum source
preparation failed,1 Catalog timed out**. Local cleanup confirmed15:25:18UTC/no survivors; native
Unknown attachment `fa2a7f40-c8e7-4cad-979b-337acac3f364` remains protected; nextest
`880d45af-6c46-4e34-8212-76fa154b973b`. Catalog completed
evidence and execution enrichment, then timed out at `apply_models`. No complete generated Catalog,
independent admission/publication/query result is inferred. `20261013T152450.918Z-337db8` compiled
and finished12 selected controls: **11 bodies passed,1 maximum source preparation failed**; local
cleanup confirmed15:27:39UTC/no survivors; native Unknown attachment
`cdb94d67-b771-4248-96cb-a1a5debba23a` remains protected, nextest
`0fd40fb6-61af-4caf-8997-a1a41ac9e4d0`. The pure shared-error/finality retry control passed, as
did129-row late-failure/overlap, literal identity, alias/empty/lifetime and exact-window cases.

That second refusal disproved wrapper recognition as the complete capacity diagnosis. Source review
localized a separate lifetime mismatch: a full ExpectedRecord decode/receiving arena could coexist
with the sorter's spill/carry merge arena after its native row was no longer needed. After full
canonical backing validation, the byte-preserving compiler_record normalization now moves that row
and its existing reservation into a private sorter entry route. The original arena remains live
through actual JSON encoding; once the native Value is consumed, it shrinks to the exact encoded
Vec capacity/key/Entry footprint and remains charged while normal sorter/merge ownership is admitted.
No guessed smaller native multiplier, budget increase or validation removal is involved. Failure
poisons/drops the same provisional sorter and retains the whole-attempt fallback. Independent source
review accepted this lifecycle transfer and its narrow same-pool call sites on2026-10-13.

`20261013T153459.537Z-66fd07` is building the current maximum and affected controls plus a revealing
8MiB forced-spill/merge control that transfers7MiB decode arenas, checks original rows/EOF and zero
remaining reservation. Catalog is excluded from this focused retry; its broader timeout and the
publication/receiver equivalence decision remain open. R5 and cited receipts remain owned/protected.

Current owned-arena run `20261013T153459.537Z-66fd07` **passed13/13 plus enclosing fixture**,
nextest `1696221d-ec06-40f9-9f27-86d6a5bac809`; compile passed3m21s, local cleanup confirmed
15:38:26UTC/no survivors. This includes the original maximum payload/key/scan/export/fresh-import/
identity control under the recorded256MiB source and8MiB consumer contract; the8MiB forced-spill
arena transfer; shared-error/finality exclusion;129-row uncached overlap/late failure; literal v4;
exact record/guard/hold refusal; empty/dependency and nonempty alias selection; and source lifetime.
It resolves the latest maximum preparation regression. It does not validate cold8MiB acquisition,
the Catalog composition or the newly exposed publication/attachment equivalence contracts.

All actual test workers have drained. Root-owned r5 checked disposal is running in
`20261013T153850.333Z-d9ea57` for exact installation
`aa85313a-5a40-4a51-a395-59b427eb2b68`. A failed logical pin/hold drain is not reclassified as a
pass; only authenticated executor/daemon shutdown and confirmed whole-target discard can settle
that target's remaining logical cleanup. Shared/operator/old paused targets stay untouched.

R5 checked disposal `20261013T153850.333Z-d9ea57` **passed**, local cleanup confirmed15:39:06UTC/
no survivors. Exact disposal receipt reports executor shutdown, datastore disposal, attachment and
storage release, unit/installer removal and credential removal all passed; logical drain failed on
retained readers/backup holds. Confirmed whole-target discard settles only that target's remaining
cleanup. Original failed/timeout/native Unknown outcomes remain unchanged, with retained receipts;
no shared/operator/old paused target was inspected, reset or adopted. No owned native worker remains.

Guard-free `just ready`, managed `20261013T153947.674Z-7f7a0e`, **passed** after current direct
dependency-edge changes, with checkout environment/native sync, skill links and tool readiness.
No dependency versions moved and no whole lock resolution occurred. This is readiness, not product
qualification. Applicable focused source controls are the13/13 run above; broad repository quality,
assembled qualification, embedding/MCP runtime and real-library/operator activation remain not_run
by the approved scope. No worktree was created; main remains the integration owner. The open identity
choice now has a concrete consumer map and two native correctness seams, and is required before
that dependent CP04 work proceeds. No complete Catalog, CP04 or whole-plan claim is made.
