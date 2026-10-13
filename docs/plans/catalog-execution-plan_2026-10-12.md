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

**2026-10-12: consolidated plans authored; production remediation not started.** Next executable
scope after execution authorization is CP00's consumer/physical contract and bounded IF04 assessment,
then CP01/CP02 and CP03 primitives with CP04 integration. CU01/CU04 are independent ready branches.
The foundations committed at `7d84ce14` remain the baseline; no new product check or campaign ran
while authoring. Accepted target does not close any implementation finding.

| Source-qualified finding | Current disposition and responsible route | Closure/evidence boundary |
|---|---|---|
| catalog#F01 | Open; CP01/CP02/CP04 | Internal views stop full reconstruction; exact final union independently checked; actual CE01 journey |
| catalog#F02 | Open; CP00/CP04 | Earned protected internal selection, complete retained/cold/physical-consumer migration and removed needless lifecycles |
| catalog#F03 | Open; CP03/CP04 | Actual bounded set windows for remaining demand, exact correspondence and receiving CAS/finality |
| catalog#F04 | Open; CP03 | Prepared directed frontier, composed exact aliases, unreachable-source refusal and spill |
| catalog#F05 | Open; CU01 | Former deferral trigger activated by combined planning; selected dump preparation and independent restoration |
| catalog#F06 | Open; CU02 | Former deferral trigger activated; exact input-port partition positions and independent checker topology |
| catalog#F07 | Open; CU03 | Former deferral trigger activated; direct logical fold, frozen export once, actual independent destination admission |
| catalog#F08 | Open, low priority; CU04 | Former deferral trigger activated; immutable checked Spec, one Rust response, pure adapter parity; required before CE03 |
| lifecycle#F01 | Open, implementation/source evidence available; CP04/CE01 | Populated generated-analysis/admitted-publication/native-query assertions remain not_run |
| lifecycle#F02 | Closed at source/focused-Tested strength, inherited 2026-10-12; CP03 preserves it | Existing actual originals/audit, exact-view and late-terminal controls; not proof all catalog#F03 scalar sites migrated |
| lifecycle#F03 | Partial, protected batch preparation source-accepted and cold-batch Tested; CP02/CP04 | Existing scoped reuse survives; catalog#F01/F02 and enclosing functional acceptance remain open |
| lifecycle#F04 | Closed at source/focused-Tested strength, inherited 2026-10-12; OP01 preserves/adapts | Actual writer/host diagnostic round trips; failure/Unknown/disposal independent |
| lifecycle#F05 | Closed for conservative recapture, inherited 2026-10-12; CP00 preserves | Equal-peer/replay/mutation/ABA/retirement/cancellation controls; continuous replay is separate IF04 |
| foundations#IF01 / IF02 / IF03 | Closed at inherited source/focused-control strength; OP02 | Authenticated disposal receipt/executor, private installer/unit retirement, configured namespace inventory; actual checked disposal supports scope |
| foundations-follow-up#IF04 | Bounded decision required in CP00; conservative safety retained | Transaction-local exact replay specialization only if all exclusion/incidence/new-hold premises survive. Otherwise supported keep-conservative result with continuous-replay revisit trigger; not asserted timeout cause |
| foundations-follow-up#IF05 / IF07 | Implemented/source-accepted; runtime enclosing acceptance pending; CP02/CP04/CE01 | Corrected temporary/retained descriptor accounting; 64 MiB receiver body passed, original enclosing fixture failed/Unknown. Facts semantics still unreached |
| foundations-follow-up#IF06 | Closed at inherited source/focused-control strength; OP01 | Borrowed server observation authenticates original executable owner; no new release authority |
| foundations-follow-up#IF08 | Closed at inherited source/focused-control strength; OP02 | Durable token journal, authenticated restoration and actual beta SIGTERM recovery; disposal separate |
| foundations-follow-up#IF09 | Closed at inherited source/focused-control/checked-disposal strength; OP02 | Bounded journaled logical children and authenticated whole-executor discard; original failed outcomes preserved |
| completion#IF10 | Partial bounded fragment correction; full target revised, open through catalog#F01/F02/F03 | Existing fragment and exact-set alias preparation accepted; fresh all-family sort/hash and per-view native demand still remain |

Follow-up evidence owners are the
[foundation review](../design_review/reviews/design_review_surrealdb-lifecycle-foundations-integrated_2026-10-12.md)
and [completion review](../design_review/reviews/design_review_surrealdb-lifecycle-completion-integrated_2026-10-12.md).
The foundations review introduces IF04/IF05/IF08 and records the root/resume IF06 and reviewed
IF07/IF09 follow-ups. The completion review refines IF05/IF07 and introduces IF10. Labels above
preserve that provenance without inventing additional review findings. CP00's new decision does not
erase the original limited acceptance.

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
