# Catalog upper execution: efficiency and performance target review

**Decision: Revise. Source assessment, 2026-10-17.** The current compiler has useful semantic and lifecycle foundations, but its complete execution still performs avoidable rich reads, repeated preparation, scalar coordination and serial work across independent units. These are structural execution-fit defects; this review does **not** establish which one caused the three retained 360-second timeouts or promise a measured speedup.

The best supported direction is to acquire protected immutable inputs once per actual trust boundary, derive the compact products consumers need, hydrate rich rows for explicit demand, and overlap independent preparation and computation while integrating outputs coherently. This extends mechanisms already in the codebase. It does not require a new universal execution framework, a second canonical store or weaker analysis.

## 1. Scope, baseline and evidence

| Field | Contract |
|---|---|
| Subject | Production paths feeding Catalog Analysis, Behavioral Analysis and retrieval preparation, expanded through providers, normalization, native completion/admission, C0/C1/C2, synthesis and finalization where they materially affect execution |
| Baseline | Clean main at **8d721482f60f5b06f7a65e52a1ce51e56f0c50d0**; source citations refer to this revision |
| Standard | [Core/template3.3](../design_principles/core/design-principles.md), [efficiency heuristics1.0](../design_principles/core/efficient-architecture-heuristics.md), [code-intelligence1.5](../design_principles/profiles/code-intelligence/principles.md), [repository binding](../design_principles/binding/library-context.md) |
| Tier / purpose | Design / target: best architecture for the functional outcome, not agreement with existing mechanisms or ADRs |
| Reviewers | Codex coordinator; bounded native and analysis source inquiries; fresh independent design reviewer, whose separate assessment is retained in the [evidence folder](../evidence/2026-10-17_catalog-upper-execution-performance/README.md) |
| Functional intent | A pinned API/evidence catalog with complete attributed facts, public contracts, contextual evidence and retrieval units; optional Behavioral enrichment retains its declared finite model and uncertainty. See [product owner](../../design/sections/api-and-evidence-product.md), [semantic owner](../../design/sections/semantic-model.md), [analytics](../../design/sections/analytics.md) and [behavioral owner](../../design/sections/behavioral-analysis.md) |
| Workload premise | Whole pinned libraries, many semantic families and completed contributors, overlapping public/statement/evidence roots, skewed modules, branching and cyclic relationships, multiple contexts/frames and concurrent logical attempts on one local machine. Bounded memory and truthful refusal remain necessary; small fixtures do not replace this premise |
| Evidence mode | Source and retained evidence first; selective probes permitted for material uncertainty. Source inspection was sufficient for the structural findings. **No new product builds, tests, database queries, services or probes were run** |
| Exclusions | Embedding inference, submitted MCP query runtime, operator adoption, full Catalog/real-library campaigns, platform compatibility and environmental qualification. Code-intelligence generation and query/retrieval preparation are included |
| Maturity | Diagnoses below are **Implemented / source-inspected, 2026-10-17**. Remedies and new compositions are **Proposed**. Prior controls retain their original Tested scope; no new Measured performance claim |

### Retained timeout evidence

Run **20261017T081150.024Z-50e4b4**, native invocation **76edb7be433248999203bdfc76687837**, executed:

    cargo nextest run --release -p cpg-core --test compiler_analysis --no-fail-fast -E 'test(=analysis_preparation::catalog_keeps_selected_catalog_and_exact_native_inventory) | test(=analysis_preparation::behavioral_keeps_selected_catalog_and_exact_native_inventory) | test(=retrieval_preparation::mandatory_four_family_preparation_uses_completed_native_catalog_sources)'

All three controls **failed by timeout at360seconds per test**, during compilation, before their intended assertions. The native log locates their unwrap at crates/cpg-core/tests/fixtures/catalog_runtime.rs:425. Cargo compilation completed before Nextest started its test clocks; build time is not the explanation for these per-test timeouts. Cancellation, retry and recovery-handback errors in the terminal report do not locate the original elapsed work. Some sibling upper work was active, but the capture is insufficient to reconstruct critical-path occupancy or establish a root cause.

The [collection coordinator §6](../../plans/catalog-collection-efficiency-plan_2026-10-17.md#6-sole-disposition-and-current-checkpoint) retains original receipts and storage holds. Native Unknown/unresolved cleanup on attachment6900d42a-7d20-41e8-b1ed-51a933639d06, and the two earlier attachments, remain protected. Confirmed host cleanup is separate. This review inspected existing files; it did not observe or change the current service. Existing capture objects611e1dae-4aeb-4009-b7b2-4bcc4ecdc904 andf69e3acb-8782-4e6e-b93e-1002476de5dc retain their named citation consumer. The report route is `just verify report 20261017T081150.024Z-50e4b4 --json`; exact cases/evidence/events are available through that existing owner.

## 2. Responsibilities and the integrated target

| Owner | Meaning and contract | Physical responsibility / allowed change |
|---|---|---|
| lctx-model | Nominal identity, source/context, fidelity, expected domains, graph scope, analysis applicability, outcomes and invariants | Declare sufficient property projections and input contracts; mechanism changes cannot invent a second classifier or erase alternatives |
| cpg-extract / cpg-flow | Independent latest Ruff syntax, native Pyrefly typing and ty runtime observations over pinned captures | Preserve independent analyzer meanings, capture ownership, actual provider completion and cancellation; this review inspects their scheduling integration rather than repeating the collection review |
| lctx-native | Exact protected physical observations, immutable payload access, original bytes, pins, effects and native finality | Acquire/batch/protect once for the actual boundary; distinguish compact selector preparation from full logical identity materialization |
| lctx-compiler-store | Completed contribution/view ownership, selected providers, independent receiving admission, relational execution | Preserve exact owner observations and atomic premise publication while grouping compatible comparisons and hydration |
| cpg-core | Stage readiness, selected immutable inputs, domain kernel adaptation and typed output integration | Schedule actual dependency readiness; reuse prepared inputs; overlap independent pure work and keep publication coherent |
| lctx-analytics / model kernels | Explicit projections, finite structural/behavioral/concept algorithms and governed heuristic results | Use indexed correspondence, factorized incidence and existing graph/SCC mechanisms without changing the declared model |
| C0/C1/C2, S0 and retrieval | Public universe, original evidence/associations, completeness-aware selection, grounded synthesis and contextual units | Preserve public-root independence, exact root membership, original anchors and applicability/context distinctions |

The target has three deliberately different reusable products. **Protected acquisition** establishes what exact data was observed under which live premises. **Consumer preparation** derives membership partitions, properties, indexes or topology over those observations. **Semantic results** belong to their analysis owner and exact invocation. Sharing acquisition or preparation never implies another owner's result, fresh provenance or admission can be replayed.

Keep these products within existing operation, attempt, completed-view and receiving lifetimes. Extend InstallationAudit, model-owned property/index contracts, PreparedRootBatch/IndexedRootColumns and existing stage/blocking execution owners. Add a helper or type only where it hides a real invariant; do not create competing caches for each finding.

### Fact and fidelity boundary

| Input / result | Authority and scope | Required distinction retained by every remedy |
|---|---|---|
| Ruff/Pyrefly/ty assertions and supports | Exact producer/profile/context/source and nominal assertion/support families | Syntax, inference, runtime flow and candidates remain different facts; all qualified alternatives survive |
| Native inventory | Qualified assertion identity and original support fidelity | A stored ID or matching digest alone does not validate payloads, attribution or support |
| Uniform NotRequested | Owner-specific expected domain over exact completed sources | NotRequested is neither success with an empty result nor absence; physical Complete describes production of the compact declaration |
| Graph preparation | Declared projection universe, direction, multiplicity and lineage | Dense positions and collapsed algorithm views never replace persistent relationship identity |
| Behavioral completion / Summary | Stated model, finite transfer/condition semantics and supported bodies/calls | Structural reachability is not execution; incomplete, unsupported, Resource and Unknown stay explicit |
| FCA/RCA and analytic rankings | Attributed incidence, method policy and scope | FCA and RCA have different incidence; heuristic results cannot become semantic controls or facts |
| Evidence / retrieval | Same-snapshot originals, root membership and renderer definition | Supporting references do not become roots; contextual text cannot nominate applicability; original bytes stay pinned |

## 3. Findings

IDs F01–F14 are stable within this review. All corrective directions are Proposed. Priority expresses consequence, not benchmark rank or implementation order. **High** means pervasive or materially amplified work on the supported path; **medium** means growth/failure dependent or a more localized improvement. Current unscheduled dispositions are in §9.

<a id="F01"></a>
### F01 — Live selection eagerly materializes full logical backing, after a separate audit

**High · lctx-native · FP-07/A4; DP-03/10/19; H5/12/16/19/20.** Ordinary select_live_view_scoped in installed_view/earned.rs:293–331 independently audits memberships/aliases, then calls prepare_closure_scoped and retains LogicalContributorRows in EarnedContributionSet::_prepared. Logical preparation in installed_view/audit.rs:5152–5294 scans memberships again, reads compiler_record backing again and creates sorted contribution/membership/backing runs. selected_prepared_member_result at6276–6300 uses only relation_memberships for ordinary selection. The extra full runs are retained and cloned with the earned capability; they are not required by its selected-row methods.

Necessary assurance already reads complete memberships and backing payloads (audit.rs:2736–2813,3014–3134). Boundary assurance explicitly combines that audit with separate full preparation at2598–2614. This confirms the preliminary double-read lead, but sharpens the remedy: **avoid eager full logical materialization for ordinary selectors**, rather than merely fuse two full passes.

Produce compact audited membership partitions while performing necessary protected assurance. Build full normalized contribution/backing runs only for actual frozen/retained identity or export demand. When that demand exists at the same trust boundary, feed assurance and normalized runs from one acquisition. Preserve complete output digest/EOF, canonical payload checks, endpoint/original guards and holds, alias reachability, contributor incarnation/descriptor, final transport drainage and live pin/context/resource identity. A subset request must not silently reduce a promised whole-contributor audit.

The simpler alternative is one fused audit/full-preparation pass. It removes repeated acquisition but retains needless sort and live state for internal selection; use it only if a real consumer requires those runs. Closure evidence: an ordinary selector obtains identical exact members without building full backing runs; full-identity consumers still earn identical logical roots; late failure, aliases and independent receiving remain revealing controls. Independent receiving may share data within its own boundary, never inherit producer authority.

<a id="F02"></a>
### F02 — Capacity fallback restarts an entire contribution

**Medium · native logical preparation · FP-07/A4; DP-20; H14/24.** audit.rs:5126–5150 halves the backing-read window after any terminal local Resource and restarts prepare_logical_contributor_window. Membership scanning, normalization, sorting and backing-ID acquisition repeat before the smaller backing page is attempted. A refusal from a sorter or an irreducible row is not evidence that a smaller page will help. The timed-out runs do not establish that this fallback occurred.

Separate completed immutable membership/ID preparation from backing-page acquisition. Retry only the exact effect-free page when the typed refusal identifies reducible page workspace; retain the original Resource for irreducible row, retained-run or unrelated refusal. Preserve failed/poisoned sorter exclusion, complete EOF/drainage before publication, singleton semantics and original causes. F01's preparation split is a prerequisite, not a reason to hide this separate failure-amplification obligation. Closure evidence should force a later-page capacity refusal and show that completed membership work is retained while failed page state is discarded; transport/Unknown errors must remain terminal.

<a id="F03"></a>
### F03 — Original completion ignores absent outputs and repeatedly reads scalar payloads/blobs

**High · compiler-store completion / native originals · FP-07/A4; DP-10/19; H5/8/18/23/26.** compiler.rs:6303 unconditionally invokes complete_contribution_originals. The helper at6092–6148 probes both SourceArtifact and ArtifactChunk even for upper contributions producing neither. A membership page holds up to32 rows, but payload readback at6121 is one awaited SELECT per membership. originals.rs:523–546 deduplicates blob completion only within that call/page, while complete_blob at436–514 fully validates bytes even for an already completed header. A shared blob across pages or source/chunk families can repeat that readback.

Pass the exact earned output inventory/counts to original completion; skip only families established absent by that inventory. Fetch exact backing IDs and layout/source metadata in bounded sets, and retain completed-blob preparation across the actual operation keyed by complete blob/incarnation/revision premises. Every layout, source relationship, hold and pending-piece obligation still needs its own check. A completed flag is not an independent byte audit.

At independent installed admission, audit_original at3137–3263 hashes selected ranges through repeated original_bytes calls and separately audits the full blob. A protected byte pass can feed full-blob and range-specific hash obligations at **that** boundary. Existing original_bytes_batch/read_ranges (reconciliation.rs:365–447; originals.rs:620–680) supplies a useful exact range mechanism, not a ready-made replacement for full audit. Do not share producer completion authority with independent receiving. Closure evidence: original-free output performs no original-family query; shared blobs complete once per valid operation preparation; every selected range/layout is still verified, including mutation/incarnation and late-read refusal.

<a id="F04"></a>
### F04 — Nominal payload agreement is planned pair by pair

**Medium · receiving admission · FP-07/A4; DP-10/13/14; H8/9/10.** admission.rs:2321–2387 compares each same-relation pair of applicable nonempty reference targets. With K such inputs it plans K(K−1)/2 joins. Identity-only and earned-empty cases are correctly skipped. reference_preparation.rs:61–94 already materializes each original input once: the defect is repeated local joins/planning, **not quadratic native rereading**.

Perform one relation-group disagreement operation keyed by nominal ID over complete selected typed payloads. Viable alternatives are DataFusion union/group processing or a sorted stream that compares complete payloads within each ID group. Select the implementation only after establishing equivalence to the present IS DISTINCT FROM semantics (admission.rs:3009–3039), including NULL, nested/list values and list order. A digest alone is insufficient. Preserve each original owner's current observations, cancellation and failure-atomic premise publication before canonical reference-family use. Existing equality controls around3432–3507 are a starting point. Actual importance for the three tiny fixtures is unresolved; growth with multiple views/epochs is source-established.

<a id="F05"></a>
### F05 — Compact unrequested owners repeatedly rebuild common metadata

**High · model expected domains / cpg-core uniform path · FP-07/A4; DP-09/10; H7/12/14/19.** compilation/uniform.rs:30–64 constructs a fresh AnalysisUniformInputs for each invocation and reads whole batches. Deduplication is local to that call. Seven Catalog behavioral owners follow this route; model inputs distinguish common expected-domain information from owner-specific roots (uniform_outcome/analysis_owner.rs:170–235). expected.rs:544–565 fully decodes/reidentifies SourceArtifact to retain input and classifier. **SourceArtifact contains metadata, not source text**; the preliminary finding must not be described as seven full source-text scans.

Prepare sufficient model-owned properties once over compatible exact protected views and share the immutable index across consumers. Reuse the existing artifact-property route in consumed_rows.rs:10–34 rather than create a second classifier. Bind preparation to complete profile/view/context/definition dependencies and the actual resource owner. Each owner still constructs its own domain, ordered sources, declarations and total NotRequested outcome. Preserve duplicates/conflicts and exact EOF/count checks, including empty domains and multiple prefixes. This does not justify caching another owner's result or adding a global rich cache. Closure evidence compares shared preparation with independent owner domains under changed context, absent roots, overlaps and cancellation.

<a id="F06"></a>
### F06 — C0 transports rich catalog inputs during a metadata pass that discards them

**High · C0 preparation · FP-07/A4; DP-10/14; H5/7/16.** catalog_core.rs:48–104 populates ConsumedInputs from CatalogData::validation_inputs and inventories all catalog_inputs. The callback retains expected-domain data and only AnalysisDefinition, MethodParameters and ProviderRun metadata. Other rich catalog types are ignored: expected::CoverageAdmission::visit_if_expected at expected.rs:1142–1159 explicitly returns Skipped for unrecognized relations. C0 later discovers selected public windows and hydrates their rich inputs again (catalog_core.rs:225–290).

Make the metadata pass declare and acquire only the sufficient expected-domain/property and authored metadata inputs. Retain declaration reachability checks without using physical payload reads as evidence that a loader exists. Rich catalog premises belong to selected C0 windows. F05's common preparation may supply this pass, but removing the **discarded** reads does not depend on cross-owner sharing. Preserve the complete public/negative universe and independent domain admission; a selected output subset is not permission to omit global completeness premises. Closure evidence should show that an unhandled rich relation is never read by metadata preparation and still appears correctly when the selected C0 grain requires it.

<a id="F07"></a>
### F07 — Native inventory reconstructs rich payloads to publish compact premise pairs

**High · model NativeInventory / cpg-core analysis_prepare · FP-07/A4; DP-03/10; H7/15/19/20.** analysis_prepare.rs:59–141 sequentially streams full rows for selected assertion/support families after qualifications, from an inventory of75 declared pairs. Catalog excludes Flow families (analysis/native.rs:40–46), and absent assertion families are skipped (analysis_prepare.rs:99–117); neither profile automatically pays75 scans. analysis/native.rs:35–80 fully decodes, validates and reidentifies selected rows before retaining assertion→qualification and support/fidelity information. Its stage inputs explicitly demand support validation. Collection completes before output begins (analysis_prepare.rs:145–190), an important refusal boundary.

Define a model-owned sufficient inventory projection, carrying relevant properties earned by exact semantic admission through the protected immutable-view lifetime. Consume only assertion identity, qualification, original attribution/fidelity and other fields the authoritative inventory predicates actually require. If a predicate cannot be justified from carried properties, retain its full read; stored IDs alone do not satisfy it. Preserve all75 family contracts, empty-assumption basis, missing/foreign qualification rejection, alternatives, assertion/support failure semantics and collect-before-write refusal. Parallel family acquisition is secondary and must integrate deterministically. Closure requires independent malformed-support/qualification cases and exact inventory equivalence, not merely matching row counts.

<a id="F08"></a>
### F08 — Retrieval batches discovery but still multiplies membership queries and union scans

**High · retrieval preparation · FP-07/A4; DP-10/14; H8/9/10/15.** retrieval_preparation.rs:563–626 already discovers roots together, then awaits EvidenceRoot and RootSubject headers per root. load_window at353–437 executes membership queries per partition/relation and hydrates each relation's union once. RenderWindow::data at254–290 nevertheless scans every union row for each root to construct a fresh membership mask. Windowing bounds the multiplier; it does not remove it.

Batch root/subject metadata and carry partition identity through compatible membership acquisition. Build positions for each **actually filtered** partition once and gather selected rows directly. Extend the existing scoped_positions.rs:287–379 IndexedRootColumns contract, or a narrow equivalent within it. Retrieval's allowed-type, artifact, text-range and brief projections are stricter than raw graph closure: do not replace them with unfiltered PreparedRootBatch keys. Preserve exact port/prefix/namespace, physical duplicates, source order, root/brief distinction, absent roots and same-snapshot originals. Keep one union hydration and charged backing until all borrowers finish. Closure evidence includes overlap, duplicate requests, missing references, multiple epochs and context text that cannot become applicability.

<a id="F09"></a>
### F09 — Remaining statement/body/frame consumers repeat single-root acquisition

**Medium · Completion, Structural, Analytic and Summary adapters · FP-07/A4; DP-10; H8/12/16.** semantic_execution.rs:885–965 and execution_scope.rs:334–398 acquire a closure and rich inputs for each Completion statement/body. structural.rs:145–176,259–278, analytic.rs:241–275 and semantic_summaries.rs:229–257 similarly prepare one frame at a time. Overlapping roots repeat discovery/query/transfer work.

Migrate compatible consumers to bounded union discovery/hydration and exact indexed partitions already used by SourceCall/Enriched/Models (source_call_scope.rs:253–330; semantic_models/scope.rs:49–77; scoped_positions.rs). Preserve whole ordered callable bodies, model-specific negative universes and distinct invocation/source receipts. Supporting callable metadata cannot become a root; one body cannot be split into independent statements when its transfer semantics require order. Avoid retaining a whole-library rich union just to remove reads. Benefit for one sparse frame is uncertain; windows should follow actual overlap and ownership. Closure evidence uses overlapping statements/bodies/frames, cycles, missing premises, genuine empty domains and unchanged independent outcomes.

<a id="F10"></a>
### F10 — FCA/RCA attribute extraction repeatedly searches whole inventories

**High when concepts are selected · analytic Correspondence · FP-07/A4; DP-10; H5/9/12/15.** analytics/attributes.rs:877–1022 scans all type observations per object and searches owner/parameter inventories. native_attributes.rs:110–245 scans class metadata/members/supports per class;341–429 scans variants/signature ports/supports per callable;459–470 searches capture/support inventories. Existing charged Correspondence and parameter/class caches (attributes.rs:25–48,111–215,270–350) already own relevant preparation.

Extend those indexes by exact occurrence/owner/context, callable/context, signature and assertion/support identity. Use known membership IDs for point access; retain all qualified alternatives. Resolve symbol-to-class candidates once for their complete context. Keep premise validation at the original consuming scope: eager global rejection could fail a legitimate request because of irrelevant rows. Preserve multiple incidence sources, class-member authority independent of flags, declared-versus-inferred distinctions and missing outcomes. Closure evidence is exact attributed incidence under shuffled, duplicated, qualified and irrelevant-invalid inputs; smaller scan scope is directly inspectable. Concept enumeration's theoretical complexity is not evidence that its current algorithm is wrong.

<a id="F11"></a>
### F11 — Concept scopes repeatedly partition common facts and rebuild FCA/RCA bases

**Medium · analytic concept preparation · FP-07/A4; DP-10; H9/12/16.** attributes.rs:1156–1259 builds a common selected-object fact inventory, then scans it repeatedly to count, construct and publish each namespace scope, and scans selections per scope. analytics/build.rs:958–974 invokes concepts separately for Concepts and RelationalConcepts, repeating their common base acquisition when both are selected. RCA-only relationships are a distinct extension (attributes.rs:1038–1136).

Index attributed facts/selections by object once or distribute references into exact namespace partitions once. Share the base incidence preparation when object/context/policy dependencies match, then derive RCA-only additions. Each technique retains its own scope, completeness, cap, implication basis and output. Identical attributes from different evidence sources remain distinct incidence support. Do not cache a shared lattice: FCA and RCA intentionally differ. Closure evidence compares complete attributed facts, concepts and implication outputs under multiple namespaces, overlapping exposures and both method selections; partial-enumeration semantics remain unchanged.

<a id="F12"></a>
### F12 — Broad execution epochs override the declared dependency graph

**High · compiler scheduling / admission · FP-07/A4; DP-08/19/20; H2/22/23.** compilation.rs:1178–1190 awaits all facts plus facts availability before any normalization. It then completes every normalization and whole normalized semantic admission before launching any upper task (1413–1422). Upper scheduling uses dependency readiness only inside that later epoch. ConfigurationGroup declarations have no completed inputs (analysis/preparation.rs:114–153), yet also wait behind all of this work. freeze_completed_inputs at560–581 additionally enforces ordered publication-group prefixes.

Separate readiness to prepare/compute from authority to consume/publish. Release a consumer only after its complete declared producers, exact selected views and required semantic checks are ready; permit independent configuration/preparation and eligible normalization/upper work to overlap. Preserve a full final admission barrier before publication and all genuinely cumulative binding/coverage dependencies. Whole Facts/Normalized barriers are current rules subject to RC01, not axioms of the target. A single undifferentiated DAG that treats computed, completed, admitted and published as one state is also wrong.

The first safe simplification can overlap input-free configuration and other genuinely independent preparation while broader admission factoring is decided. Closure evidence must block an unrelated provider, permit only eligible consumers, exercise late malformed inputs and cancellation, and show no premature public visibility. Exact eligibility and whether prefix order can be relaxed are open decisions in §7; no claim that every normalization can run before all facts is made.

<a id="F13"></a>
### F13 — Independent contributors and pure grains are computed serially inside stages

**High for many independent grains · compiler/native/analysis owners · FP-07/A4; DP-11/19/20; H21/22.** Native assurance/preparation loops await contributors one by one (audit.rs:2630–2676,4724–4769,5928–5999). C1 artifact_batch at catalog_evidence.rs:343–373 hydrates the union, then awaits each independent blocking builder and emission before starting the next. C0 windows, model partitions and retrieval roots are likewise serial; analytics/build.rs:616–975 runs selected techniques serially. Existing stage concurrency does not parallelize these inner operations. block_in_place makes other tasks able to progress; it does not parallelize one closure.

After removing repeated work, expose independent contributor preparations, windows, roots, frames or method computations under the existing execution owner. Share immutable prepared input and compute private results concurrently; integrate in a defined semantic order through the existing typed output owner. Respect actual method dependencies, ordered bodies and SCC/fixpoint dependencies. Reservations must cover simultaneous input/work/output and queued results; useful overlap cannot turn predictable admission into uncontrolled arrival-dependent Resource failures. Do not impose arbitrary job/thread caps or create nested pools to make tests pass. Reuse available concurrency and existing ownership, with demand and actual resource admission governing launches.

Single dominant grains gain little from this change; many independent grains can avoid unnecessary serialization. Closure evidence is revealing progress under a deliberately blocked independent peer, deterministic shuffled completion, cancellation with joined cleanup, preserved original/secondary errors and release after the last borrower. Per-row producer push is buffered and is **not** a database write per row; parallelizing writers is not the proposed fix.

<a id="F14"></a>
### F14 — Cold audit sorting and spill run inside ordinary async workers

**High for substantial closures · native preparation execution · FP-07/A4; DP-19/20; H21/23/26.** InstallationAudit::run_work_many uses tokio::spawn (audit.rs:623). Its logical, selected-member and alias preparations call synchronous SortedRows push/finish, compose_selected_member_runs and folds (5152–5294,6220–6251,6306–6338,4771–4783). ordered_rows.rs:209–261,540–610 performs blocking file I/O and merges with Value serialization/decoding. This occupies runtime workers that also progress transport and cancellation. Actual starvation in the failed run is unmeasured.

Move substantial sorting/merging/folding behind an owned blocking boundary. Reuse acknowledged_candidates.rs:84–110,672–715, already consumed by compiler membership preparation at compiler.rs:8643–8683, or execute a complete bounded unit under the same ownership contract. Avoid adding a handoff per row; batch commands where appropriate. Retain actual worker joins, scratch/pin/resource ownership and primary/secondary failure reporting until terminality. Failed or poisoned preparation cannot publish.

The existing higher-layer borrowed_cpu helper already uses block_in_place on multithread runtimes (stage_runtime.rs:7–30); Structural/Analytic/Summary use it. Thus the claim that all CPU work runs unprotected on async workers is false. Tokio blocking tasks cannot be aborted once started, so merely changing spawn to spawn_blocking without retaining drainage is insufficient. Closure evidence includes fitting/spilled runs, cancellation during a merge, waiter loss and independent I/O progress; blocking placement and amount of useful parallel work remain separate questions.

## 4. Strengths, related paths and preliminary-lead disposition

The existing design supplies much of the target's foundation:

- Native providers accept selected nominal sets and projections; compiler_provider.rs:305–398 distinguishes Exact/Inexact/Unsupported filters and selected_batches chooses bounded demands. selection.rs:1234–1290 uses exact contributors/relation/keys and indexed membership. This is not evidence of whole-store scans.
- InstallationAudit already coalesces successful operation-scoped work with exact context/pin ownership and evicts failed flights (audit.rs:117–151,472–650). Retain this owner rather than add global proof caches.
- PreparedEdges discovers nominal union adjacency once, terminates cycles with visited keys, then propagates exact root memberships over compact adjacency (compiler-store/consumed_rows.rs:1412–1610,1743–1880). Replacing it with all-path enumeration or blanket transitive closure would regress the design.
- SourceCall/Enriched/Models already use indexed positions. GraphPreparation coalesces hydration per projection using OnceCell and retains one exact Behavioral SCC schedule (analysis_graphs.rs:150–190,351–373). These are preservation constraints, not new tasks.
- Summary uses a callee-first SCC schedule, indexed calls/seeds, lazy call preparation and a semantic progress frontier; equal/dominated proofs do not reopen recursive work (summary_production.rs:2445–2475,2522–2646; summary_worklist.rs:74–211). There is no demonstrated naive whole-graph fixed-point defect here.
- Base already has shared charged preparation and deterministic actual-owner adoption. Synthesis and C1 already use bounded union hydration and selected typed copies. Assess whether a copy releases backing or simplifies ownership before replacing it with borrowing.
- Ingress batches, atomic guarded effects, exact replay, complete cuts and durable Unknown handling solve real concurrent/failure obligations. The three timeouts do not prove stripe contention, excessive retry or an obsolete need for independent admission. No production support limit or assertion is weakened by this review.

| Preliminary chat lead | Final assessment |
|---|---|
| Double assurance/preparation reads | Confirmed; F01 improves the target by removing eager full logical backing before fusing necessary acquisition |
| Repeated Catalog compact metadata | Confirmed F05; metadata, not source-text scans |
| Full-payload native inventory | Confirmed F07; projection must preserve model validators and actual admitted premises |
| Retrieval membership/filter multiplier | Confirmed F08; union hydration and native pushdown already exist |
| Whole-contribution Resource retry | Confirmed F02; occurrence in the timeout is unknown |
| Pairwise reference payload joins | Confirmed F04; original native inputs are already materialized once |
| Preparation tests compile whole frontiers | Confirmed; feedback recommendation R01 below, not a production speed claim or proven timeout cause |

### R01 — Make focused contract feedback independent of full-frontier setup

analysis_preparation.rs:5–38 compiles the full Analysis frontier for each profile with FCA enabled; retrieval_preparation.rs:6–61 compiles through Catalog to inspect mandatory four-family units. These are useful integration journeys. Their assertions and later fixture inspection SQL were not reached in the failed run.

Add minimal revealing controls at the actual inventory/property/window contracts, using genuine persisted completed inputs where persistence matters and pure inputs for pure kernels. Retain the full-frontier journeys for their integration purpose and unchanged assertions; do not replace them with mocks or silently remove coverage. Their clocks, normal concurrency and cleanup contracts remain. Separating feedback improves implementation iteration; only production changes F01–F14 can establish production improvements. Use existing coarse producer/native phases and selective request evidence for a future causal run, rather than invent a mandatory per-row tracing or benchmarking system. The actual test fixture already initializes explicitly requested RUST_LOG tracing to captured stderr (tests/fixtures/native.rs:7–25); a filter such as warn,lctx_phase=info can expose existing coarse events without enabling per-root debug detail. Availability of that route is not evidence that the old capture contains those events.

## 5. Composition, scenarios and preservation contracts

| Stage family | Inputs, outputs and scope | Proposed physical route / limits |
|---|---|---|
| Providers / normalization | Exact pinned captures and independently attributed facts → complete normalized relations | Actual dependency readiness with complete producer sets; preserve final availability/coverage and independent trust checks |
| Native inventory / compact owners | Exact admitted native/domain metadata → premises or owner-specific NotRequested | Model-owned property projections and shared immutable preparation; no inherited result/admission authority |
| C0/C1/C2 | Complete public/negative universe and scoped native premises → contracts, evidence, exact selection outcomes | Compact initial inventory, bounded discovery/union hydration, private parallel builders, coherent typed emission |
| Behavioral completion / Summary | Whole ordered bodies, calls, qualifications and stated model → bounded verdicts/proofs | Indexed windows where compatible, existing SCC/worklist kernels, component/owner order retained |
| Structural / Analytic | Declared projection/frame/universe and policy → exact structural or qualified analytic outputs | One protected topology, indexed incidence/partitions; independent computations may overlap, reductions retain declared determinism |
| S0 / retrieval | Same-snapshot facts/findings/evidence, renderer and original bytes → grounded briefs/units/windows | Retain current bounded source grounding; exact partition-position gathers, delayed hydration and ordered integration |
| Final admission / publication | Completed immutable views, model/codec/definition identity and protected ownership → admitted manifest | Full promised admission and coherent visibility remain, even if preparation and acquisition are shared |

All compositions preserve declared applicability, identity, multiplicity, source order where required, full payload conflict refusal, independent receiving, original evidence closure, complete negative domains, explicit uncertainty, actual owner/resource lifetimes and joined native finality. Broad sharing that erases a trust boundary conflicts with this target. Fine-grained batching that reproduces scalar queries inside a wrapper also fails to remove the identified work.

The following analysis records identify the semantic constraints on execution changes. Exact settings and limits remain with the cited executable owners; this review does not introduce new defaults.

| Question / stage | Universe, selector and relationships | Method / settings | Fidelity and model | Resource / incomplete behavior | Output and evidence |
|---|---|---|---|---|---|
| What native premises exist? NativeInventory | Exact selected assertion/support families and qualifications; attribution retains original direction and multiplicity | Model finite pair inventory and native validators; no seed | Extracted/native-qualified, never relabelled runtime semantics | Attempt budget; malformed/foreign inputs refuse before writes; no successful prefix | NativeAssertionPremise and NativeQualification linked to original assertions/supports |
| What was not requested? Compact owners | Complete owner-specific expected input/context/root domains, including absent/empty cases | AnalysisUniformInputs, owner definitions and exact profile | Derived applicability; NotRequested explicitly distinct from unknown/absent | Charged shared properties, fresh owner output; exact EOF and duplicate refusal | UniformNotRequested plus exact ordered source snapshots |
| What API/evidence exists? C0/C1/C2 | Complete admitted public/negative universe; explicit root namespaces and model scope relationships, alternatives retained | Catalog/evidence/selection definitions and authored parameters | Derived/resolved under the catalog model; uncertainty explicit | Bounded windows and actual-owner admission; insufficient premises do not certify completeness | Public contracts, original evidence/associations, witnesses and qualified selection |
| What completes / propagates? Completion and Summary | Whole ordered bodies and exact calls/conditions; CallableInvocation projection for Summary, callee-first SCCs; nominal relationships remain distinct | Selected finite model/catalog, transfer kernels, condition limits and semantic progress worklist | Conservative/model-relative behavioral verdicts; reachability alone is insufficient | Budget/refusal/unsupported/Unknown remain typed; no silently truncated fixed point | Outcomes, derivations/proofs, source receipts and coverage |
| What structure exists? Structural | Declared CallableInvocation/DefinitionContainment universes and frame roots; direction, weights and any algorithm collapse come from the projection owner | Structural method definitions and settings; canonical prepared topology | Exact structural claims within the declared projection, not execution | Shared charged graph and selected frame workspace; limits retain explicit outcomes | Structural frames/findings with original graph witnesses |
| Which analytic patterns rank/group? Analytic/FCA/RCA | Declared graph frames and public namespace/object scopes; attributed incidence; RCA-only relation additions remain separate | AnalyticsConfiguration and method policy, including seeds/convergence where applicable; FCA support/bound from retained policy | Heuristic rankings/communities, exact incidence/finite concept results within the declared context; partial lattice stays partial | Combined preparation/kernel/output budget; nonconvergence and enumeration bound explicit | Analytic results, method/scope identity, attributed incidences, concepts/basis and witnesses |
| What grounded material should an agent read? S0/retrieval | Same-snapshot selected findings/native premises and original ranges; independent public/evidence/brief roots; context versus applicability preserved | Synthesis/renderer/retrieval definitions and configured brief selection; embedding inference excluded | Programmatic derivation and documentary evidence with explicit execution/uncertainty status | Bounded exact root windows and original bytes; unsupported/unavailable outcomes stay distinct | Briefs, four-family units/windows, original anchors, root/evidence links |

| Revealing scenario / change kind | Current consequence and target obligation |
|---|---|
| Large skewed library; growth | F01/F03/F05–F08 amplify preparation/transfer even with bounded output. Minimize necessary work before adding concurrency; an irreducible large row remains an honest refusal |
| Empty or sparse Catalog; instance | F03/F06 should skip earned-absent physical work while retaining complete negative/NotRequested evidence. Empty statistics must not bypass current native authority |
| Overlapping roots and duplicate exposures; composition | F08/F09 share union acquisition while retaining root ownership, duplicate positions, namespace/prefix and exact context restrictions |
| Cyclic calls, branching bodies; domain behavior | Existing visited/SCC/fixpoint semantics remain. No arbitrary truncation or body splitting; changed algorithms must preserve implications, verdicts and evidence |
| Add a native assertion family; domain extension | One model declaration supplies full and sufficient-property contracts plus validators; inventory/compact consumers use them, not new per-consumer classifiers |
| Replace the conflict comparison mechanism; mechanism substitution | Same typed full-payload equality, original observations and atomic premise publication; no permanent second canonical representation or duplicate validator |
| Concurrent attempts or receiving owners; lifecycle | Share only compatible protected preparation. Each destination earns independent observations; pin/incarnation/pool mismatch invalidates reuse |
| Cold versus reused preparation; instance | First-miss work and retained working set matter; existing caches do not excuse eager materialization. Failed flights cannot become successful cached prefixes |
| Resource refusal, cancellation, late stream failure; failure | Retry the justified unit, retain original causes and cleanup/Unknown. Concurrent private results remain charged and joined; none publish after failure |

## 6. Library fit and alternatives

**Interface-checked,2026-10-17.** Cargo.toml/Cargo.lock resolve DataFusion55.1.0, Arrow59.3.0, Tokio1.53.2 and petgraph0.8.3; the native integration declares SurrealKit1.0.0-beta.1 and the local patched SurrealDB3.3.0 family. Current upstream documentation was fetched through Context7 as discovery; exact local implementations and pinned capability contracts govern transfer. No dependency upgrade was performed or selected by this review.

| Capability | Best supported composition / rejected shortcut |
|---|---|
| Set reads / projections | Existing parameterized native ID sets and indexed membership queries, preserving model-owned field selection. [SurrealQL SELECT](https://surrealdb.com/docs/reference/query-language/statements/select) documents set/field selection; existing local queries demonstrate the needed route. A single large SQL string or an EXPLAIN label does not establish optimal physical execution |
| Relational disagreement / membership | DataFusion full-payload union/group or ordered comparison; extend current prepared inputs. [DataFusion55.1 TableProvider](https://docs.rs/datafusion/55.1.0/datafusion/datasource/trait.TableProvider.html) exposes projection/filter contracts. Exact pushdown must fully enforce semantics; Inexact needs residual evaluation. Do not reclassify filters to eliminate residual work without equivalence |
| Row selection | Existing IndexedRootColumns over Arrow selection. Use checked exact positions, source order and multiplicity; take versus filter is a semantic choice. Pinned Arrow59.3 take helpers have bounds/null/zero-column differences, so the current tested gather owner is preferable to a fresh handwritten mask/index subsystem |
| CPU / spill execution | Existing borrowed_cpu and acknowledged blocking owners. [Tokio1.53.2 spawn_blocking](https://docs.rs/tokio/1.53.2/tokio/task/fn.spawn_blocking.html) requires retaining started work through completion; it is not cancellable once running. Avoid nested executors or per-element handoffs that add more machinery than work removed |
| Graphs / cycles | Existing petgraph topology/SCC plus model worklists and bounded conditions. No evidence that moving these kernels into database traversal or adopting another graph engine removes the dominant repeated preparation. Use declared graph semantics; shortest-path/traversal features do not replace behavioral transfer or FCA implication contracts |
| Concept enumeration | Current charged equal-length bitsets and explicit partial enumeration provide a credible starting point. Index/factor preparation first. In-Close/other enumeration or parallel variants are investigations only until required basis, support and partial-prefix semantics are established |
| Native transactional ownership | Retain atomic guarded effects, complete cuts, exact replay and receiving admission. Source does not justify wholesale replacement of the database, transport or lifecycle protocol to solve these findings |

The recommended alternative extends existing owners and removes duplicated work. A smaller first increment can delete skipped C0 reads, scope original completion, project compact properties and correct blocking placement without redesigning all scheduling. Retaining the baseline avoids transition risk but leaves demonstrated amplification. A new general scheduler/cache framework or wholesale backend change adds migration, invalidation and recovery burdens without evidence of a necessary capability gap. Selected native grouping remains a valid alternative to host grouping when exact semantics and physical access favor it; database placement is not a goal in itself.

## 7. Open decisions and further investigations

These are specific unsettled choices, not reasons to require a full benchmark campaign or postpone clear removals. A negative result may justify retaining a mechanism; it does not erase a source-established defect in a different mechanism.

| ID | Decision / uncertainty | Useful settling evidence and effect |
|---|---|---|
| I01 | What actually occupied the timed-out controls: query execution, preparation, commit retry, coordination, CPU or cleanup? | Existing request/producer/native evidence first; if insufficient, one selected revealing current-source journey with existing coarse phase capture and unchanged timeout. Inspect actual pending ownership separately. Necessary for a causal claim, not for F01–F14 structural diagnoses |
| I02 | Which normalized/upper consumers can start before global Facts/Normalized admission, and can publication-prefix order be relaxed? | Derive each exact complete producer/selected-view/semantic prerequisite; blocked-peer and late-invalid-input controls. Input-free configuration overlap is already justified; global-to-scoped admission changes require RC01 and a coherent trust contract |
| I03 | Which NativeInventory predicates can consume earned properties instead of rich records? | Trace every authoritative validator and its immutable premises; qualification/support corruption and cross-context controls. Retain full reads where a sufficient projection is not established |
| I04 | Which full logical runs are truly demanded, and can assurance supply their rows without expanding retained state? | Enumerate actual selector/identity/export consumers and source-owner lifetimes; cold live-selection and full-root equivalence. Choose lazy split versus fused full preparation based on those consumers |
| I05 | Grouped payload agreement in DataFusion, native grouping or sorted exact comparison? | NULL/nested/list/order/equal-duplicate cases and physical plan inspection at the exact pin. Do not rely on COUNT DISTINCT dropping NULL or an unverified digest encoding |
| I06 | Window size/representation and useful parallel grain for sparse versus overlapping roots? | Minimal overlap/skew cases, actual lifetime/reservation inspection and independent-peer progress. Keep normal available threading; compare complete working set and deterministic integration rather than tuning arbitrary caps |
| I07 | Would a different concept algorithm or basis construction improve required output? | Inspect current concepts.rs:132–247 and alternatives against support, canonical implication basis, enumeration cap and partial meaning. No algorithm substitution until equivalence is clear; F10/F11 do not depend on it |
| I08 | Can Summary release/stream settled components earlier, or execute independent condensation components concurrently? | Trace cross-component proof/evidence borrowers and mutable actual-owner adoption; retain current worklist if release or independence cannot be established. Current SCC implementation is a strength, not a diagnosed defect |
| I09 | How often do Resource fallback, original sharing, multi-view agreement and query-plan differences matter in actual work? | Existing captures, small exact counters/plan observations only if the answer changes remedy selection. Quantitative latency/throughput claims require later representative evidence; no historical hard benchmark prerequisite |

Unexamined breadth: full serving/index runtime, embedding inference, OS/store-engine tuning, arbitrary third-party analyzer internals and full-library timing/capacity. None is silently accepted, diagnosed or scheduled here.

## 8. Architectural judgment and priorities

| Judgment | Verdict | Scoped reasoning |
|---|---|---|
| A1 Localize change | Satisfied at source-inspected owner boundaries | Model, native effects, kernels and output integration have identifiable owners. R01 records disproportionate focused-test setup; new sufficient-property contracts must remain owned rather than duplicated |
| A2 Encode domain meaning | Satisfied for inspected meanings | Nominal identities, attribution, typed fidelity, expected domains, graph scopes, NotRequested and failure/finality distinctions govern the inspected behavior. No efficiency remedy is permission to weaken them |
| A3 Extend through composition | Satisfied for inspected capability contracts | Existing native/DataFusion/indexed-window/graph owners support the proposed composition. Their incomplete adoption and physical serialization are assessed under A4; a new generic framework is not warranted |
| A4 Fit execution to workload | **Violated** | F01–F14 show avoidable acquisition/materialization, multiplicative searches/joins, overbroad retries/barriers and serialized independent work for legitimate growth/failure scenarios |

FP-01–FP-06 are satisfied within these inspected semantic boundaries; FP-07 and the applicable DP-10 preparation/bulk-execution default are violated. The composed-library and machinery considerations of DP-13–DP-16 reinforce the named A4 findings; no unsupported generic-library replacement is asserted to manufacture a G8 failure. DP-01/02/03/04/07/08/09/11/12/18/19/20 and CI attribution/fidelity/unknown/projection obligations constrain the remedies. In particular, overbroad retry or serial execution does not by itself demonstrate false publication, missing drainage or an ungoverned effect. This is qualitative judgment, not a checklist score. Unexamined support remains unexamined.

| Gate | Verdict | Evidence / limit |
|---|---|---|
| G1 Authority | Pass, inspected scope | Model declarations and exact producer/view identities remain authoritative; no competing semantic definition demonstrated |
| G2 Semantic fidelity | Pass, inspected scope | Typed source/role/fidelity and distinct availability/outcome meanings remain; proposed projections must retain them |
| G3 Validity | Pass at source strength | Authoritative validators, complete inventory refusal and final admission exist. This is not fresh malformed-input or upper-journey acceptance |
| G4 Hidden behavior | Pass, inspected scope | Provider effects and native writes are explicit; pure kernels use captured inputs; no ambient/evaluation input route demonstrated |
| G5 Consistency / recovery | **Unresolved for assembled execution** | Source has real joins/cuts/Unknown protection, but retained upper controls ended with native finality unresolved. No proof that the report is false, nor proof that those obligations have settled |
| G6 Transformation / reuse | Pass at source strength for existing inspected routes | Exact views, consumer permits, namespace/position owners and SCC/worklist semantics constrain reuse. Proposed grouping/projections/parallel reductions need their own equivalence evidence |
| G7 Truthful claims | Pass for this bounded assessment | Tested historical scope, unmeasured benefit and unresolved timeout cause are explicit; no full-frontier performance/acceptance claimed |
| G8 Library leverage | Pass for selected generic mechanisms; composition needs revision under A4 | Existing DataFusion/Arrow/Tokio/petgraph/sort owners cover the relevant capabilities. Incomplete bulk/index adoption is concrete A4 damage; bespoke spill/trust specialization is not proved replaceable by a generic library without a contract gap |
| CI-G1 Fidelity | Pass, inspected scope | Provider meanings, heuristics, models and Unknown remain distinct |
| CI-G2 Evidence closure | Pass at source strength for inspected preparation | Original-root/context/anchor contracts remain; timed-out retrieval assertions and submitted serving are not qualified |
| CI-G3 Evaluation integrity | Not applicable to changed behavior | No evaluation/gold/heldout input or evaluator change; those populations were not inspected or used |

**Consequence priority:** address pervasive cold preparation/original work F01/F03, discarded and repeated metadata F05/F06, inventory width F07, retrieval amplification F08 and blocking placement F14 first. F10 is equally important when concepts are selected. F12/F13 address elapsed execution overlap but should not amplify the remaining redundant work. F02/F04/F09/F11 address important failure/growth shapes; they are not dismissed because their manifestation in this capture is unknown.

**Prerequisite order differs:** establish complete sufficient-property and protected-preparation contracts before sharing or narrowing their consumers; split membership versus logical-root preparation before localizing fallback; retain exact indexed root/backing ownership before window migration; reserve and join private compute before overlapping grains. F06's skipped-read removal and F03's declared-output scoping are independent early corrections. F04's exact equality design and F10's correspondence indexing can proceed independently. Detailed execution packages and validation scheduling belong to subsequent plan creation.

## 9. Disposition, rule impacts and verification boundary

**F01–F14 and R01: Deferred from implementation; trigger is authorization of a follow-up implementation plan consuming this review.** This source review owns these unscheduled dispositions until transfer to one active plan ledger. **I01–I09: Deferred investigations with the decision-specific triggers/evidence in §7.** No original review or prior finding is edited, closed or superseded. The collection coordinator remains the sole owner of P/R/IR/E acceptance and protected native obligations; this review adds target findings rather than retroactively changing PC90's scoped source acceptance.

The following are proposed changes to current rules, not approvals. Accepted ADR bodies remain immutable; plan creation presents consequential dispositions, then the owning ADR/architecture route changes the rule. Keeping a rule narrows the remedy as stated.

| ID | Current rule / source | Proposed change and dependent findings | If retained |
|---|---|---|---|
| <a id="RC01"></a>RC01 | Whole Facts/Normalized barriers and eager ordered publication prefixes; semantic owner §15.11, ADR-0174 and compilation.rs | Distinguish complete selected readiness/assurance from final whole-frontier admission; permit eligible overlap and only justified prefix ordering; F12 | Keep barriers; implement input-free/private preparation overlap where safe, leave the broader critical-path finding open |
| <a id="RC02"></a>RC02 | Earned selectors retain full normalized logical preparation; installed_view/earned.rs and ADR-0181's native preparation realization | Separate compact audited membership capability from demand-driven full logical identity runs; F01/F02 | Fuse acquisition while retaining full runs; report residual eager work and memory |
| <a id="RC03"></a>RC03 | Every native inventory family supplies full typed rows and replays payload predicates; analysis/native.rs, semantic owner §15.1/§15.11 | Admit model-owned sufficient properties carried under exact checked immutable premises; preserve independent receiving; F07 | Retain rich inventory validation; improve batching/placement only, leaving repeated rich reconstruction |
| <a id="RC04"></a>RC04 | Uniform owner inputs use full batches and fresh local metadata; uniform_outcome/analysis_owner.rs, compilation/uniform.rs | Add sufficient property input and shared compatible immutable preparation, retaining fresh owner-specific outcomes; F05/F06 | Remove demonstrably skipped reads, project only already-supported properties, retain other repeated preparation |
| <a id="RC05"></a>RC05 | Original completion invoked for all contributions, page-local blob preparation; compiler.rs/originals.rs, storage owner §6.1–§6.2 | Use earned output absence, bounded set reads and operation-local exact blob preparation; F03 | Batch payloads within a page but retain absent-family probes/repeated cross-page blob work |
| <a id="RC06"></a>RC06 | Pairwise full-payload comparison before reference-family use; admission.rs/reference_preparation.rs | One complete relation-group disagreement operation with the same typed equality and independent observations; F04 | Retain pairwise joins; reuse existing materialization without claiming grouped improvement |
| <a id="RC07"></a>RC07 | Single-root/frame acquisition and retrieval partition masks; execution/retrieval adapters and semantic owner §15.1 | Extend exact indexed window realization to remaining consumers; no graph/model/root reinterpretation; F08/F09 | Keep per-root acquisition/filtering; improve local field scope only |
| <a id="RC08"></a>RC08 | Serial contributor/grain/method compute and mixed async sorting placement; current compiler/native adapters, ADR-0179 realization | Owned independent preparation/compute overlap plus acknowledged blocking work, with deterministic integration and actual resource/finality ownership; F13/F14 | Keep serial computation; still remove redundant work; leave material serialization/worker occupancy findings open |

F10/F11 change private indexed/incidence preparation within existing model semantics; no rule change is required if policy, output, validation timing and partial meaning remain. R01 changes test organization without deleting integration coverage. A new concept algorithm, reordered observable failure semantics, new shared-cache authority or altered supported universe would require a further explicit impact; none is approved here.

**Verification for this review:** source, owners, exact pinned contracts and retained capture inspection completed2026-10-17. New product checks/probes/service operations are **not_run**, deliberately: the findings are structurally decidable, and runtime causal uncertainty remains honestly open. Documentation checks for publication are recorded in the current handoff, independently of product acceptance. Follow-up implementation should use minimal revealing functional controls described with each finding, then schedule full validation separately. Larger deadlines, discarded Unknowns, arbitrary worker caps and weakened fixtures/assertions are not closure evidence.

The bounded decision is **Revise** on A4, with assembled G5 unresolved. Existing semantic/ownership strengths remain worth retaining, PC90 remains scoped, and the enclosing Catalog/Behavioral/native product is not newly qualified. The next decision is creation of a reconciled implementation plan for these target changes and required investigations; this review itself performs no remediation.
