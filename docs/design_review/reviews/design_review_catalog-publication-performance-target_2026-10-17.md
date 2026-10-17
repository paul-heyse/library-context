# Catalog publication efficiency and performance — target design review

**Decision: Revise.** The current design has strong semantic ownership, immutable completed
inputs, explicit uncertainty and meaningful publication protection. Its physical execution does
not yet fit the supported workload. Several independent forms of amplification occur before
Summary, inside finite composition, during output emission, throughout native persistence and in
public publication preparation. Fixing the nested deferred lookup, increasing the128-row window
or adding concurrency would leave these causes substantially intact.

The principal correction is to preserve semantic distinctions while changing execution units.
Dependencies need not cause rich reads, canonical rows need not individually create the current
physical ownership machinery, and a bounded native page need not independently repeat the
complete host recovery protocol. Existing contracts and physical formats can change deliberately.
The replacement must retain complete logical meaning, independent receiving admission, coherent
visibility, exact ownership and actual finality.

Diagnoses are **Interface-checked against implemented source,2026-10-17**, with bounded retained
**Tested/Measured** evidence identified below. Replacement directions are **Proposed**; this review
establishes neither a speedup nor release qualification.

## Scope, standard and evidence

| Field | Assessment |
|---|---|
| Baseline | `7f353fa1fffd081eb9f25003b61cbea9f74ded67`; production source unchanged during review. Concurrent changes were new review evidence documents. |
| Tier and purpose | Design / target |
| Reviewer | Fresh accountable principal design reviewer,2026-10-17; coordinator published and reconciled bounded source inquiries without changing the verdict. |
| Standard | [Core3.3](../design_principles/core/design-principles.md), [template3.3](../design_principles/core/design-review-template.md), [efficient-architecture heuristics1.0](../design_principles/core/efficient-architecture-heuristics.md), [code-intelligence1.5](../design_principles/profiles/code-intelligence/principles.md) and [library-context binding](../design_principles/binding/library-context.md), selected by [standard.toml](../design_principles/standard.toml). |
| Functional target | [API and evidence product](../../design/sections/api-and-evidence-product.md): version-scoped APIs, configuration and connected original evidence; behavioral enrichment remains qualified and optional. |
| Examined scope | Complete recursive Analysis control; relevant predecessors, Summary preparation/kernel/output, native ingress/completion/admission/recovery; ordinary/export/restored public publication; selected backup, receiving reconstruction, read pins and analogous patterns. |
| Workload premise | Pinned real-library extraction and catalog construction; finite behavioral analysis over shared call topology; many logical evidence records, shared dependencies, recursive or high-degree outliers, multiple semantic kinds and concurrent compile/serve lifetimes. |
| Excluded effects | No deployed DDL, maintenance, recovery, restart, operator adoption, implementation remediation, full Catalog qualification or assembled acceptance. |
| Evidence limits | No fresh product executions. No server-side decomposition of parse/planning/UDF/index/commit time. Full recursive output cardinality, disposition mix and semantic result remain unknown. |

The standard and functional intent are the criteria. ADRs, current physical layouts, row limits
and generic effect protocols describe the reviewed subject; they are not constraints protecting
it from revision.

### What the retained executions establish

The commands, raw capture locations and qualifications are recorded in [retained execution](../evidence/2026-10-17_catalog-publication-performance/retained-execution.md).
The principal reviewer inspected the recursive log and synthetic control's JUnit capture.

The three-control invocation was:

```text
just fixture -- cargo nextest run --release -p cpg-core --lib --test compiler_artifacts --no-fail-fast -E 'test(selected_assurance_retains_exact_owner_closure_and_refuses_missing_input_binding) | test(scoped_summary::actual_summary_matches_complete_finite_whole_operation) | test(scoped_summary::actual_recursive_summary_preserves_scc_delta_answers_and_exhaustion)'
```

Its outcomes were:

- **passed:** selected assurance,1.436s;
- **passed:** ordinary Summary control,874.607s;
- **failed:** recursive control timed out at900.231s before its oracle comparison;
- **confirmed:** enclosing host cleanup;
- **unresolved:** recursive remote finality; its attachment remains protected.

The recursive Summary phase began595.618s after initial native setup. Nineteen complete4096-row
PairOutcome writes were acknowledged before termination:77,824 rows were already being emitted.
The kernel had returned; the capture does not establish its isolated duration, eventual output
size or correctness. Phase totals overlap and cannot be added into a wall-time breakdown.

The separate physical control was:

```text
just fixture -- cargo nextest run --release -p lctx-compiler-store --lib --no-fail-fast -E 'test(native_summary_pair_outcome_batch_has_committed_window_receipts)'
```

It **passed** in15.714s. Its4096 synthetic PairOutcome rows produced4096 payload nodes,4096
memberships and73 immutable effects. The observed atomic-apply bucket was12.697s across84 effects
including surrounding lifecycle work. Recovery-settlement time overlaps nested phases. This is
evidence of the physical operation and confirmed cleanup, not a Summary oracle or a speedup
comparison.

The900-second control does **not** call public manifest publication. It compiles the Analysis
frontier, admits semantics, reacquires inputs, invokes the ordinary finite Summary producer,
compares output families, performs source-specific assertions and cleans up. Calling its timeout
"public sealing time" would be incorrect.

### Coverage and supporting inquiries

The [analysis and emission inquiry](../evidence/2026-10-17_catalog-publication-performance/analysis-and-emission.md)
covers the actual control, all44 Summary output families and14 vocabulary families, their
downstream uses, Base/Completion/SourceCalls/Enriched preparation, scheduling and producer
recurrences. The [native inquiry](../evidence/2026-10-17_catalog-publication-performance/native-effects-and-lifetimes.md)
covers lowering, all17 production control-schema functions, guarded effects, recovery, completion,
candidate verification, cleanup, SDK session lifetimes and contrasting set access paths.
The [publication and admission inquiry](../evidence/2026-10-17_catalog-publication-performance/publication-and-admission.md)
covers ordinary publication, roles/search, executable definitions, retention readiness, assurance,
cold verification, originals and selected backup/restore. The [library inquiry](../evidence/2026-10-17_catalog-publication-performance/library/README.md)
qualifies existing library compositions against exact locked/patched source and current docs.
These notes support this combined judgment; they do not own separate overall verdicts.

This is broad source coverage, not a claim that every instruction or every input distribution
has been measured. New probes were unnecessary to establish the structural verdict. Deployment
correspondence, server memory amplification, detailed timing shares and replacement performance
remain explicit uncertainties. Serving was examined where publication pins, evidence consumers
and access-path analogues matter; embedding inference and submitted MCP execution were not run.

## Responsibilities, meaning and preservation constraints

The existing decomposition is useful:

| Owner | Responsibility and consequential contract |
|---|---|
| `lctx-model` | Typed identities, provider fidelity, coverage, qualifications, operation meanings, finite proofs, Summary questions and verdicts. |
| Native providers and normalization | Attributed observations and exact source/role correspondence; unresolved and conflicting information remain explicit. |
| `cpg-core` | Bind exact completed inputs, prepare selected computational views, execute domain owners and integrate completed output. |
| `lctx-compiler-store` | Attempt/contribution ownership, native completion and independently earned semantic admission. |
| `lctx-native` | Physical addressing, guarded immutable effects, membership, holds, pins, transport and recovery. |
| `lctx-publisher` | Consume admitted capabilities; prepare derived content; reconcile and publish one immutable manifest. |
| Serving | Retain a complete publication and definition realization through answers, evidence references and continuations. |
| Restore/evaluation | Independently challenge incoming claims; imported bytes and private expectations do not grant production authority. |

Dependency direction generally respects these owners. The important defects are predominantly
in the physical composition between them, rather than a lack of meaningful record types.

### Fact and fidelity

| Family or relation | Basis and fidelity | Coverage/unknown meaning | Identity and consumers |
|---|---|---|---|
| Native syntax, references, typing and flow | Provider-attributed captured observations under their declared roles. | Native boundaries and unsupported observations remain explicit. | Nominal model identities; normalization, Local and execution. |
| Call binding and invocation topology | Normalized, qualified alternatives and declared projection. | A possible target or topology edge does not establish definite invocation or complete dispatch. | Exact callsite/attempt/member identity; Summary scheduling and composition. |
| Local/Model transfers | Derived under explicit entry, model, condition and qualification premises. | Unknown entry, aliases and unsupported operations cannot become absence. | Transfer keys, alternatives, origins and evidence. |
| Summary witnesses and paths | Finite composition under exact call bindings and bounded proof semantics. | Exhaustion produces residuals; incomplete topology cannot become complete coverage. | Witness DAG and proof identities; consequences, S0 and native explanation. |
| PairOutcome | Outcome of an actually consumed caller/callee proof pair. | Disjoint, Subsumed, Proven and Refused are distinct. | Current closure-question construction and replay, as well as diagnostics. |
| Catalog and source evidence | Public identity, declared options and original captured material. | Declaration, effective behavior, observed use and unresolved support remain distinct. | Catalog selection, source hydration and serving. |
| Manifest/search/roles | Derived physical realizations of admitted semantic content. | Completion, admission, publication and selection remain separate. | Exact publication/view/eligibility/definition identities and reader pins. |

No new provider-relabeling, unknown-as-absent or ungrounded served-claim defect was demonstrated
in this review. These distinctions constrain every correction below. Analyzer requalification
is not part of this review; existing exact provider/model identities must survive the changes.

### Analysis and execution meanings

| Operation | Universe/method | Limits and completeness | Output/evidence |
|---|---|---|---|
| Base and Completion | Selected syntax and authentic earlier evaluation owners. | Missing evidence remains a boundary; shared preparation must not widen a selected root. | Evaluations, statement/body completion and exact premises. |
| Summary | Complete declared invocation projection; callee-first SCCs, qualified transfer composition, BDD conditions and finite worklist. | Depth, proof, work and member bounds have distinct meanings; residuals preserve unfinished questions. | Finite witnesses, aggregate alternatives, questions, standings and obligations. |
| Semantic admission | Actual complete selected dependencies and model-owned invariants. | Selected authority cannot escalate into whole-artifact authority. | Privately constructed admitted capability. |
| Public publication | Exact admitted views plus executable definition epoch. | Drain/freeze/reconcile before marker; retain actual staging/incarnations. | Immutable unselected manifest and retained derived root. |
| Backup/restore | Selected recoverable closure within the appropriate snapshot/pin. | Export EOF/terminality; independent incoming and installed admission. | Data-only transport and freshly earned authority. |

The SCC scheduler genuinely uses petgraph's `kosaraju_scc` and binds its canonical schedule to
the actual materialization (`summary_schedule.rs:65–101`). The finite worklist distinguishes
semantic progress from proof alternatives and explicit refusal. These are strengths. Neither
database traversal nor a generic graph closure is an adequate substitute for transfer semantics.

## Findings: remove repeated work before tuning execution

The findings share interfaces but do not collapse into one remedy. Input acquisition, proof
generation, physical representation, recovery and public reconciliation have different
obligations. Source basenames below are resolved in the linked bounded inquiries; all line
references refer to the pinned baseline.

<a id="F01"></a>
### F01 — Predecessor adapters read rich inputs that their immediate operation does not consume

**High · FP-03, FP-06, FP-07; DP-08, DP-10; A4 violated.**

`semantic_execution.rs:33–100` routes `load_ignored` through the normal rich stream and a no-op
visitor. Base, Completion, SourceCalls and Enriched declare and traverse broad inventories
before their selected computation reads. Examples include `:206–259`, `:1137–1190` and
`:1514–1585`. `cpg-core/src/consumed_rows.rs:119–126` issues `SELECT *`. These declarations bind
relation, epoch and order; `load_ignored` still reads full payloads even where no visitor uses
them. Coverage dispatch can explicitly return `Skipped`
(`analysis/expected.rs:1160–1176`).

Thus semantic dependency declaration, adapter dispatch completeness, expected-domain metadata
and actual computational acquisition have become unnecessarily coupled. Checking that a typed
loader exists does not require streaming unused bodies. These scans are upstream of Summary
and are relevant to the substantial pre-Summary elapsed interval, although their timing share
is not isolated.

A related access-path problem remains inside Base capture. `base_scope/capture.rs:134–202`
plans active-root and distinct-ID queries repeatedly over the same local membership relation,
including an earlier-root anti-query for each active partition. First consuming root can instead
be derived once by `(kind,id) → minimum root ordinal`, while retaining exact per-root membership
separately. Capture-local discovery and payload state end between successive outer capture windows, permitting
shared dependencies to be reacquired. Existing shared adjacency preparation is a positive
nonmatch; not all discovery is repeated.

Native capture occurs once per outer admitted root window. Subsequent resource-driven local
compute retries already reuse that captured backing. The remaining cross-window opportunity
must not be misdiagnosed as native replay on every local retry.

**Correction — Proposed.** Let each consumer declare sufficient metadata and actual payload
demand separately from its complete semantic/receiving dependency closure. Keep exact permits,
epochs, coverage, selected membership and final admission. Use a single operation-owned local
membership/first-use preparation and compare bounded cohort retention with invocation-owned
spillable acquisition. Local's narrower metadata loader (`local_semantics.rs:203–218`) is an
existing example, not a mandate for one universal loader.

**Legitimate counterexample.** A global analysis or negative premise may require complete
inputs. An expected-domain visitor that actually consumes rows must retain those rows and their
terminal checks. Do not suppress transport failures on a stream that is genuinely consumed, or
replace semantic authority with metadata.

**Closure.** Trace these four predecessor adapters so unused payload streams disappear;
demonstrate exact coverage and selected-read equivalence, including empty/unsupported domains
and failed acquisition. Challenge first-use preparation with overlapping roots and repeated
dependencies across cohorts. Complete-input admission remains independently exercised.

<a id="F02"></a>
### F02 — Summary's question domain is coupled to exhaustive attempted-proof-pair detail

**High · FP-04, FP-07; DP-08, DP-12; CI-08; A4 violated.**

The worklist selects proof pairs using relatively coarse owner/callee correspondence
(`summary_production.rs:1738–1813`). Exact actual/formal/port compatibility is decided later in
composition (`composition.rs:984–1005`). Each consumed pair can become a `PairOutcome`
(`summary_production.rs:2643–2809`).

These rows cannot safely be dismissed as logging. `summary_consequences.rs:918–945` derives the
closure-question universe from **all** attempted pairs and residuals; `:1043–1060` uses Refused
reasons and Proven witnesses to decide member standings. Removing Disjoint or Subsumed pairs
before replacing question construction could erase published questions. The current
full-operation comparison also compares the entire relation.

Conversely, the functional product does not establish a requirement to physically expand every
rejected proof combination as an independently owned database row. S0 consumes conclusions and
selected proof support; native explanation consumes authentic witness/path DAGs. Those consumers
justify evidence, but not automatically the present exhaustive trace realization. The recursive
capture shows substantial output amplification from a small source fixture; it does not reveal
the final disposition distribution.

**Correction — Proposed.** Make the intended question domain and its completeness authoritative
independently of incidental proof enumeration. Preserve exact witnesses, refusal causes,
alternatives and unanswered members. Compare:

1. A lossless compact/factorized representation retaining the current logical PairOutcome
   relation and identities.
2. An explicit question/coverage relation plus required finite proof evidence, with deliberately
   revised trace retention.
3. Earlier exact compatibility selection only after its effect on questions and
   work/exhaustion semantics is settled.

The first option can preserve current semantics without making every logical tuple a separate
physical lifetime. The second may remove more work but is a model change. Neither requires a
generic replacement analysis framework.

**Legitimate counterexample.** Two equal-cost witnesses can be distinct evidence; a refused
pair can expose an open question even where another pair succeeds. Distinct invocation guards
and qualification bases cannot be merged to reduce output.

**Closure.** Specify the question/evidence contract and compare the complete logical questions,
verdicts, residuals, finite witnesses and explainable support against revealing recursive,
reconvergent, disjoint, ambiguous and exhausted cases. State explicitly whether attempted-pair
trace semantics change. A faster subset of rows is insufficient.

<a id="F03"></a>
### F03 — Stable kernel preparation is repeatedly scanned or rebuilt inside fine-grained work

**Medium · FP-06, FP-07; DP-09, DP-10; A4 violated.**

Two concrete manifestations have different domain owners but the same preparation-lifetime
problem:

- `composition.rs:429–460` traverses complete vocabulary/map heaps to calculate each composition
  allowance. `:1033–1039` clones complete segment/literal maps to add a small projection overlay.
  Guard substitution is invoked twice (`:1068–1079`, `:1138–1149`); the final qualification differs,
  so blindly caching the first result is unsound.
- Base Completion calls `complete` per selected statement (`completion_production.rs:430–474`).
  `completion.rs:798–811` creates `PreparedExecution` again; `evaluation.rs:1343–1357` constructs and
  validates complete syntax preparation. Body completion can prepare syntax again. Exact
  evaluation correspondence also scans candidate rows per consumed fact
  (`completion_production.rs:498–516`).

Summary publication adds further repeated lookup: `summary_production.rs:2108` scans global
branches per SCC, and `:2189` searches members again for each proof source. Consequence
processing scans origin boundaries per question.

**Correction — Proposed.** Retain exact immutable preparation at its real frame/body/operation
lifetime; use charged keyed incidence and small overlays rather than whole-map copies. Carry
already established backing-size facts where valid. Split guard semantic restatement from
qualification-dependent evidence construction so only genuinely shared work is reused. Existing
`PreparedExecution::shared` and `complete_prepared_with_contexts` show a viable boundary; mutable
proof and completion state must remain per request.

Enriched already shares `PreparedExecution` within its owner frame. Canonical body order,
stop-after-nonnormal behavior and append-only context rounds carry genuine dependencies;
immutable preparation sharing does not justify parallelizing them away.

**Legitimate counterexample.** A different selected view, context, local owner or qualification
can change applicability. An owner/context comparison is necessary; it need not reconstruct
unchanged syntax or copy unrelated vocabulary.

**Closure.** Inspect the actual Base/Completion and Summary callers, not just new helper APIs.
Exercise changed backing/context, missing mappings, competing rows, qualification changes and
recursive proof alternatives. Demonstrate preparation reuse only under complete premises.

<a id="F04"></a>
### F04 — Canonical results are retained and cloned through a second row-building path; ordered delivery waits for the entire window

**Medium · FP-03, FP-07; DP-10, DP-19, DP-20; A4 violated.**

`semantic_summaries.rs:publish_frame` retains `SummaryRecords` while `emit_results` and vocabulary
emission borrow and clone rows. `producer_operations.rs:26–35` sends each clone through
`ProducerOutput::push`; `workspace.rs:3253–3318` repeats validation, locks a writer, buffers, then
builds another batch. The original result remains live through these transfers.

`owned_work::ordered` launches independent jobs, but collects every result before returning an
iterator (`owned_work.rs:60–129`). Summary begins publishing only afterward. For multiple frames,
a slow later frame delays the first frame and retains completed results. More frame parallelism
does not help the current single-frame recursive fixture.

The same borrowed emitter is used by normalization, Local, execution, Models, Structural,
Analytic, catalog selection and synthesis. This is a shared correction opportunity, rather than
a Summary-specific transport wrapper.

**Correction — Proposed.** Add a consuming canonical-output transfer that moves actual charged
backing into bounded batches and releases each family after its last consumer. Reuse the existing
batch owner where its conflict/dedup semantics fit; do not introduce another independent writer
protocol. Where complete-window success is unnecessary, deliver the next canonical ordinal while
retaining actual joins and failure ownership for later jobs.

**Legitimate counterexample.** Some result families remain inputs to discharge or consequence
construction. They cannot be released before that last use. Later failure must still poison the
enclosing attempt; early private persistence must not become public completion.

The fixed-width PairOutcome has no rich owned heap. The clone path therefore does not explain
its persistence cost by itself. Rich clone-before-reservation deserves a focused ownership check,
not an unsupported claim that all output memory is unbounded.

**Closure.** Inspect real producer migrations, reservation transfer and failure paths.
Demonstrate zero leaked charges after success, refusal and cancellation, exact canonical
ordering/conflicts and no completed visibility before required terminality.

<a id="F05"></a>
### F05 — Logical rows impose an unnecessarily expansive physical installation and observation protocol

**High · FP-02, FP-03, FP-07; DP-10, DP-14, DP-16; A4 violated.**

`lctx-compiler-store/src/compiler.rs:5898–6158` constructs separate payload, alias and membership
streams. `ImmutableWindow::unit` derives anchors, references, holds and guard bindings per row;
windows are split after physical expansion (`immutable_window.rs:1–156`, `:317–350`). A4096-row
transfer therefore becomes many small native effects.

`schema.surql:229–300` has useful bulk insertion, but prepares existing rows with scalar
subqueries, performs repeated parent checks and searches deferred candidates inside each
target's processing. A bulk INSERT does not remove this preceding work. Contributions and other
shared incidence addresses also remain contention points.

Preserving host passes add more work: graph lowering reparses canonical JSON to recover an
already typed anchor; unit planning, coalescing and emission repeat size validation; sizing,
request hashing and wire preflight serialize or copy the same prepared values, and retries
clone SQL/bindings (`adapter.rs:200–238`, `immutable_window.rs:224`, `control.rs:380,464`,
`authorization.rs:583`, `coordination.rs:103`, `retry.rs:203`). Prepare and transfer established
properties at this internal boundary while keeping independent receiving validation and exact
wire/resident admission. Their timing share is unmeasured.

The pattern recurs across trust boundaries:

- Cold backing verification first scans actual compiler records, then fetches backing again per
  membership (`compiler.rs:6905–6952`).
- Terminal captured-row verification rebuilds contributor sets per row and queries holds by
  dereferenced semantic fields (`control/product_recovery.rs:168–279`;
  `admission/candidate.rs:233`), despite a known exact expected payload.
- Original completion knows its complete layout work set but performs separate
  layout/validation/hold observations per layout (`originals.rs:753–775`).

These are not all interchangeable checks. Cold admission must independently observe actual
installed content; terminal candidate comparison must detect conflicting nominal candidates.
The common opportunity is bounded set-shaped observation and prepared exact correspondence,
not removal of independent checking.

**Correction — Proposed.** Define a precise prepared installation packet over canonical bounded
input, distinct targets, parent dependencies, holds and output membership. Validate packet
structure once before effects, retain native commit-sensitive dependencies, and bulk-install the
packet under one owned operation. Compare bounded row packets with compact immutable
blocks/extents, particularly for nongraph evidence relations. Logical relations and identities
need not change merely because their physical ownership is grouped.

The new private packet may require unique exact guard/target entries and reject ambiguous
duplicate declarations before mutation. Current malformed-packet first-match behavior is not a
functional requirement. A typed, constrained packet can permit direct indexed lookup or references
between entries without implementing universal `Value` equality indexes.

**Legitimate counterexamples.** An unlocked array read cannot replace required scalar
`FOR UPDATE` protection. Missing/forbidden observations do not retain positions. Equal-content
replay still needs complete payload agreement; a digest, `IGNORE` or upsert is insufficient. New
holds still exclude retirement. Compound/numeric equality must be preserved wherever the
admitted packet actually supports it.

**Closure.** Select the packet/physical contract, then challenge missing/equal/conflicting rows,
complete parents, changed incidence, originals, held/retiring targets, exact nominal alternatives
and atomic rollback. Independently verify membership and full logical reconstruction. Measure
the complete operation only after the route works.

<a id="F06"></a>
### F06 — Every small physical window repeats the complete host recovery lifecycle

**High · FP-03, FP-07; DP-19, DP-21; A4 violated.**

Normal execution creates an exact request, persists a host pending envelope, atomically applies
payload plus outcome/reference, persists terminal knowledge, issues a separate settlement
transaction, then removes the envelope (`authorization.rs:696–1060`; `recovery.rs:560–790`).
`signal.rs:114–186` performs durable file/directory synchronization for two writes and removal.
This machinery repeats for each small physical window.

These operations protect real uncertainty. Their **granularity** is not inherently required.
The application already has durable attempt/contribution identity and operation-owned native
progress. `history.rs:397–412` and its page commit path demonstrate a narrower exact-page
progress/replay pattern, although history compaction cannot simply be reused as a universal writer.

**Correction — Proposed.** Compare operation- or segment-scoped host discovery with native
atomic page progress. One durable owner can name the operation and unresolved pages; each
committed page records the exact payload/progress outcome transactionally. Final completion
waits for all accepted pages, EOF and joined descendants. Recovery reconciles exact original
page identities and fences absent effects, while host terminal knowledge precedes release of
the last retaining witness.

This changes the ordinary protocol; it is not permission to defer arbitrary errors, release
references on success alone or rerun unknown production payloads under fresh identities.

**Legitimate counterexample.** A crash immediately after remote commit but before local
acknowledgement must remain recoverable. A late packet cannot cross freeze/close or history
collection. Interrupted segments must remain discoverable after host loss without scanning
guessed filenames or assuming process absence proves native finality.

**Closure.** Specify interruption points and retained root/progress invariants before
implementation. Exercise crash/lost acknowledgement before and after page commit, partial
refusal, cancellation, era/cut races, absent fencing and last-reference release. Preserve the
original Unknown outcome independently of later cleanup settlement.

<a id="F07"></a>
### F07 — Terminal cleanup pages re-enter complete attempt cuts

**Medium · FP-07; DP-19, DP-20; A4 violated.**

For non-attempt effect owners, `authorization.rs:878–879` calls `native_incidence` on the owner.
Cleanup rows include their origin attempt in incidence targets; `native_touch` then calls a
complete64-stripe attempt cut (`schema.surql:168–187`, `:305–326`). Page-oriented cleanup can
consequently repeat a whole-attempt exclusion transition.

The existing complete freeze, close, admission and maintenance cuts are meaningful positive
cases. They must not be removed merely because the same helper is expensive.

**Correction — Proposed.** Give terminal cleanup/history mutation an explicit hazard contract:
which closed producer state it relies on, what exact page scope changes, which collectors/
recovery owners contend and which predicates still need writes. Then use the narrowest
sufficient page exclusion, rather than obtaining production eligibility or recutting unchanged
terminal production through incidental incidence.

**Closure.** Exercise delayed production, cleanup restart, collector races, era changes,
retained outcomes and interrupted pages. If the existing full cut is retained, identify the
concrete hazard requiring it on every page; otherwise version and remove the obsolete path.

<a id="F08"></a>
### F08 — Optional product retention is on required downstream readiness

**Medium · FP-01, FP-03, FP-07; DP-09, DP-19; A4 violated.**

`workspace.rs:3470–3474` awaits product retention after local completion but before coherent
output handoff. `workspace_products.rs:371–397` can independently prepare and retain a compound
product. The recursive capture records thirteen retention phases totaling118.866s, overlapping
other work.

Retention is useful where it avoids future computation. It does not automatically need to
delay current consumers of an already completed producer. The decision should compare
retention lookup/preparation cost with the work actually reusable.

**Correction — Proposed.** Separate required completed-input readiness from optional retained-
product preparation where lifetimes permit. Keep the actual completed owner protected for a
retention child, and join that child at the appropriate enclosing boundary. Permit effect-free
decline before submission; once retention effects are accepted, their failures and uncertainty
remain real obligations.

**Legitimate counterexample.** A separately retained product requires an independent lifetime
and complete boundary. It cannot borrow an attempt that is released before retention finishes.
A preparation failure after effects begin cannot become an ordinary cache miss.

**Closure.** Demonstrate downstream progress before unrelated retention completes, with success,
decline, accepted failure, cancellation and source-drop cases. Compare reuse benefit and
complete-operation cost without assuming every empty result is cheap or every cached result useful.

<a id="F09"></a>
### F09 — Definition verification discovers the whole database although publication needs a declared subset

**Medium · FP-06, FP-07; DP-09, DP-10; A4 violated.**

`definitions.rs:95–111` inventories all database functions, analyzers and tables, then issues
sequential `INFO FOR TABLE` queries and normalizes their definitions. `verify_expected` checks
only the required subset. Both publication setup and seal call this route
(`native_publication.rs:117`, `:345`).

The pattern also occurs in selected backup and restore comparison. Installation catalog capture
and `realization.rs` are related but have different contracts: whole-installation identity can
legitimately need whole inventory. A single universal subset shortcut would be wrong.

**Correction — Proposed.** Derive required definition addresses from the trusted blueprint/
epoch; inspect only their necessary metadata, and retain an operation-owned verified context
where administrative exclusion protects it. Keep cold verification and exact engine/schema/
function/analyzer comparison. Backup must read its required comparison metadata within the
same external snapshot as its data. A narrow INFO API is a qualification question, not assumed.

**Legitimate counterexample.** A plain epoch hash does not prove installed definitions.
Privileged changes, service-generation replacement and definition-affecting maintenance invalidate
retained protection. Unrelated executable epochs should neither broaden the necessary verification
set nor be silently treated as the same pinned epoch.

**Closure.** Add unrelated tables/functions and show ordinary required verification does not
acquire their per-table details. Change a required definition and require refusal. Exercise
source-generation/context changes and the selected-backup snapshot boundary.

<a id="F10"></a>
### F10 — Public role construction loses its prepared expectations before final reconciliation

**Medium · FP-03, FP-07; DP-10, DP-23; A4 violated.**

`native_publication.rs:268–290` reads admitted canonical entities/assertions in128-record calls.
`loader.rs:558–675` lowers and flushes role/endpoint writes. At seal, staging invokes
`reconcile_owned`, which rereads, decodes and lowers canonical content to reconstruct the same
expected roles (`reconciliation.rs:43–214`; `staging.rs:358–361`).

Fresh **actual** row comparison is essential. Reconstructing unchanged expected roles is a
separate question. `PreparedSearch` already retains opaque, protected expected runs while
performing fresh actual reconciliation (`derived_search.rs:2450–2564`).

**Correction — Proposed.** Retain bounded/spillable role expectations under the actual admitted
source, staging owner, derivation, client, resource pool and pin. Carry necessary canonical
family/envelope assurances under their preserved premises. Continue fresh actual role/root
reconciliation and final guarded marker publication. Cold restore and independently supplied
claims retain independent reconstruction.

**Legitimate counterexample.** Expected IDs alone cannot reveal extra actual roles. Actual
enumeration must originate from selected sources, not the expected list. Equal concurrent
publishers can have different physical staging realizations and still need exact convergence checks.

**Closure.** Compare ordinary prepared reconciliation and cold reconstruction against missing/
extra/changed roles, parallel positions, external endpoints, altered source/derivation/pin,
concurrent winner and root-incidence changes. Do not transfer opaque preparation through serialization.

<a id="F11"></a>
### F11 — Search lowering repeatedly scans correspondence that can be prepared once

**Medium · FP-07; DP-10, DP-14; A4 violated.**

`derived_search.rs:2066` and `:2084` scan all links per source parameter. `lower` traverses
SearchWindow twice, loading `Basic` again (`:2106`, `:2151`), and scans all companion bindings
per window part (`:2163`).

The two passes have a real dependency: occurrence rows must refer to installed lexical/vector
values. That does not require rebuilding all correspondence or rereading every unchanged input.

**Correction — Proposed.** Prepare charged exact indexes by source parameter and `(window,part)`.
Retain or spool the minimum immutable correspondence across the installation barrier; compare
that with bounded reacquisition where retention would cost more. Preserve all alternative links,
occurrence identities, dependencies and channel witnesses.

**Closure.** Exercise competing parameter names, multiple bindings, repeated text/vector values
with distinct occurrences, empty primary sets, spill and failure. Confirm retained expectations
still undergo fresh actual reconciliation. No global search cache is implied.

<a id="F12"></a>
### F12 — Borrowed observations accidentally fork authenticated SDK sessions without owning their checked closure

**Medium; finality uncertainty is material · FP-02, FP-05, FP-06, FP-07; DP-15, DP-19;
G5 unresolved for these helper lifetimes.**

`control/held.rs:55`, `:505` and `:599` construct `Arc::new(client.clone())` from `&Surreal`.
This is not an `Arc` clone. Vendored `Surreal::clone` creates a new session UUID
(`third_party/surrealdb/src/lib.rs:406–419`); the gRPC lifecycle FIFO attaches a session and
replays its authentication/use log (`grpc.rs:840–856`, `:1736–1760`).

The new client is passed to `NativeReader::private`, whose operations default to
`owns_session=false` (`reader.rs:269`, `:3227`). Reader close drains queries, but checked session
close is conditional on ownership (`:1367–1399`). SDK Drop merely enqueues session detach
(`lib.rs:422`). Production singleton calls include staging and compiler completion/retention
paths; the window wrappers are predominantly test callers.

This establishes extra session/replay/lifecycle work and a missing explicit closure owner at
the helper boundary. It does not establish corrupt publication, an outstanding query after
drained EOF, permanent session leakage, or the timeout's cause.

**Correction — Proposed.** Borrow the actual `Arc<Surreal>` or an existing read owner. When an
independent session is genuinely required, create it explicitly under an owner that joins its
closure. Preserve per-reader query drainage independently.

**Closure.** Trace the production singleton callers; verify no incidental session fork, or
exercise checked intentional child closure and detach failure. Establish the helper's actual
finality contract before treating its close result as complete session finality.

## Combined correction and alternatives

The changes should compose around three boundaries.

First, computation consumes protected, sufficient inputs. F01 and F03 remove unnecessary
acquisition and repeated preparation without relaxing semantic dependency declarations. F02
decides the required question/evidence product. F04 transfers the resulting canonical ownership
efficiently. It should not create a second cache, a second semantic classifier or a second
row-conflict policy.

Second, native installation consumes meaningful packets or compact immutable units. F05 changes
physical expansion; F06 changes recovery granularity; F07 narrows terminal cleanup hazards. They
are related but independently closable. A larger window still repeats recovery; a segment journal
still pays per-row physical expansion; eliminating scalar reads alone leaves both.

Third, publication consumes admitted content and retained derived preparation. F09 scopes
executable verification, F10 retains role expectations and F11 prepares search correspondence.
These optimizations preserve fresh actual reconciliation and the final staging-root/incarnation
guard. F08 and F12 constrain their actual lifetime and readiness composition.

Suitable existing library capabilities are already available. The [library inquiry](../evidence/2026-10-17_catalog-publication-performance/library/README.md)
records exact pinned source and Context7 corroboration. Decisive source supports bounded
record-ID reads, bulk ordinary/relation insertion, transactions and explicit lock constraints.
The patched SDK already multiplexes requests over HTTP/2; a session pool is not a prerequisite.

For small bounded native `Value` packets, ordinary Rust maps with a deliberately constrained key
contract are simpler than converting everything into Arrow/DataFusion. DataFusion is appropriate
for already-columnar grouping, first-use reduction, ordered reconciliation and spillable local
relational preparation. Its SQL null/equality and duplicate semantics are not automatically
native object equality. Petgraph SCCs and the BDD kernel remain appropriate for their actual
semantics; neither substitutes for Summary transfer composition.

The simplest credible target is therefore existing owners plus narrower input/preparation
contracts, consuming output transfer, a precise packet/segment protocol and operation-owned
expected runs. Compare compact physical blocks where their retained state and access needs
warrant them. A new workflow engine, generic cache, proof-certificate service, retry framework or
backend replacement is not justified by these findings.

### Scheduling improvements must follow real independence

Prerequisite-ready compilation and shared call topology are already implemented. The older broad
handoff gate is gone. Prefix delivery can overlap independent preparation and persistence, but
the current recursive fixture has one Summary frame. Kernel parallelism would instead need
independent ready SCC/body work and a declared deterministic admission policy. Global work/member
allowances, vocabulary extension and proof ordering currently affect bounded prefixes; simply
running all work in parallel would change refusal/exhaustion behavior. Base's ordered kernels,
Completion's first non-normal statement and Enriched's growing context premises also impose real
dependencies. Separate immutable preparation and ready computation from canonical state admission;
do not add worker caps or weaken semantics. The output inquiry records the corresponding positive
cases and places where private independent jobs currently wait for an entire window.

## Change, growth and failure scenarios

| Scenario | Expected boundary and assessment |
|---|---|
| Add an attributed evidence family | One semantic declaration plus genuine extraction/analysis behavior; codecs and consuming contracts follow. It must not automatically impose one native object/hold/effect lifetime per logical row. F04/F05 matter as kind count grows. |
| Add a qualified Summary question or proof form | Question-domain, qualification and evidence owners change deliberately. F02 prevents an incidental enumeration strategy from silently defining the new public question set. |
| Substitute row packets with compact blocks/segments | Change native physical addressing, selection, ownership and recovery together. Preserve logical identities, all logical families, exact view eligibility and independent reconstruction. This is a real mechanism substitution, not a type-compatible shortcut. |
| Large body with many statements | Current Completion rebuilds syntax per statement. Shared immutable body preparation is credible; mutable proof state remains independent. F03. |
| Many roots sharing dependencies | Current no-op scans, cohort reacquisition, per-root local queries and repeated candidate hold scans amplify work. Exact selected universes remain necessary. F01/F05. |
| Guarded recursion/high-degree callsite | Proof alternatives, conditions and questions can grow differently. Bound and represent them separately; do not erase unfinished suffixes or invent Exact convergence. F02/F03/F05. |
| Crash after native page commit | Retain exact operation/page outcome and discoverable recovery ownership; absence alone does not prove nonexecution. F06. |
| Freeze/close/retirement overlaps delayed effects | Preserve conflict addresses, incarnations, complete cuts where required and actual executor drain. Narrow terminal cleanup only under its explicit hazard contract. F05–F07. |
| New publication while an old reader follows evidence | Keep complete publication/definition pins through references, resources and continuations. Retained preparation cannot transfer by hash or descriptor alone. F09/F10. |
| Detached backup or hostile restore | Same-snapshot export and independently validated actual incoming/installed content remain. Imported definitions and runtime control records never activate authority. |

## Architectural judgments and gates

A1–A3 describe the inspected semantic ownership, not an endorsement of the current physical route.

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | **Satisfied, scoped** | Meaningful operations have identifiable model, compiler, native and publisher owners. Pure finite kernels can be checked without unrelated services. Physical packet and lifetime changes legitimately cross their owning boundaries; findings identify the leakage that must be removed. |
| A2 Encode domain meaning explicitly | **Satisfied, scoped** | The inspected model governs attribution, exact roles, coverage, proof/condition semantics, completion, admission and publication. PairOutcome's current meaning is explicit but costly; that is not automatically a missing-model MUST violation. A revised question domain remains Proposed. |
| A3 Extend through composition | **Satisfied, scoped** | Existing completed inputs, pure kernels, guarded effects, admission capabilities and staging/pin owners supply credible composition seams. Replacements must use these meanings rather than invent parallel orchestration or assurance authorities. |
| A4 Fit execution to workload | **Violated** | F01–F11 establish avoidable broad reads, repeated preparation, retained copies, per-row physical expansion, overfine recovery and public reconstruction. Protective limits and the900s timeout do not establish feasibility. |

| Foundation | Verdict | Scope and reason |
|---|---|---|
| FP-01 Separation of concerns | **Satisfied, scoped** | Semantic, compiler, native and publication responsibilities have coherent owners. F08 identifies an execution dependency between readiness and optional retention. |
| FP-02 Stable contracts | **Unresolved** | Semantic substitution contracts are explicit, but F12 leaves the auxiliary session's ownership and checked-close contract unsettled. |
| FP-03 Composition | **Satisfied, scoped** | Existing capabilities compose through meaningful contracts. F04/F08 identify unnecessary physical barriers, assessed under A4. |
| FP-04 Domain model and authority | **Satisfied, scoped** | The inspected model governs consequential distinctions and operations; costly explicit semantics do not automatically establish an A2 defect. |
| FP-05 Explicit structure | **Unresolved** | F12 creates an authenticated child session outside the helper's explicit session-close ownership. |
| FP-06 Local reasoning | **Unresolved** | Domain kernels have bounded dependencies, but F12's apparent borrowed observation conceals an additional session lifetime whose finality cannot be established from the helper contract. |
| FP-07 Execution fit | **Violated** | F01–F11 establish unjustified amplification across acquisition, computation, persistence and publication. |

These foundation judgments leave A1–A4 and the gates unchanged: A1/A3 concern their explicitly
bounded semantic scenarios; F12's physical lifetime remains unresolved. DP-10, DP-16 and DP-19
supply material correction obligations; their status is not rescued by correct output.

| Gate | Verdict and evidence boundary |
|---|---|
| G1 Authority | **Pass, scoped source assessment.** No new competing writable semantic authority demonstrated. |
| G2 Semantic fidelity | **Pass, scoped source assessment.** Required distinctions remain explicit; proposed pruning or factorization must preserve them or change the contract deliberately. |
| G3 Validity | **Pass, scoped source assessment.** Exact selected assurance, typed construction, independent admission and guarded completion remain meaningful enforcement points. |
| G4 Hidden behavior | **Pass for inspected semantic operations.** Acquisition, native writes and publication effects are explicit. F12 concerns accidentally expanded effect ownership, not a demonstrated hidden semantic rewrite. |
| G5 Consistency/recovery | **Unresolved for F12 auxiliary-session finality.** Main publication/Unknown safeguards are substantial; the helper's actual fork/close composition is not established. Recursive native finality also remains unresolved operational evidence, not a newly inferred protocol failure. |
| G6 Transformation/reuse | **Pass for inspected current contracts; recursive runtime qualification remains open.** Same-kernel differential comparison is useful but not an independent algorithmic oracle. Replacement routes are not accepted by this verdict. |
| G7 Truthful claims | **Pass at this stated boundary.** No speedup, full recursive result, whole Catalog qualification or adoption is claimed. |
| G8 Library leverage | **Pass, scoped.** Appropriate libraries and native bulk capabilities are present; remaining A4 defects concern their composition. No inspected library supplies the entire required semantic/recovery capability as a drop-in replacement. |
| CI-G1 Fidelity | **Pass, scoped source assessment.** Provider relations, qualification, unknown and heuristic distinctions remain explicit. |
| CI-G2 Evidence closure | **Pass, scoped source assessment.** Same-generation proof/source closure and pinned serving remain required; no new broken claim was demonstrated. |
| CI-G3 Evaluation integrity | **Pass for inspected separation; limited oracle independence.** Production and ordinary replay share the finite algorithm. Source-specific known answers independently challenge selected semantics; their recursive run was not reached. |

The bounded decision is **Revise**, independently warranted by A4 and reinforced by unresolved
auxiliary-session finality. The enclosing publication/Analysis architecture needs revision for
the selected workload. This is not a rejection of its semantic model or a qualification verdict
for the broader product.

## Rule impacts

These are proposed changes, not applied authority. Existing ADRs remain immutable; planning must
present selected directions and route successor decisions through the owning sections.

| ID | Rule affected and proposed change | Dependent findings; consequence if retained |
|---|---|---|
| <a id="RC01"></a>RC01 | Current dependency/loader coupling in semantic execution and §15.1/§15.11: distinguish complete semantic dependencies from sufficient metadata and actual read demand. | F01. If retained, broad no-op acquisition remains and F01 stays open. |
| <a id="RC02"></a>RC02 | Summary question/trace contract in behavioral analysis and ADR-0180-related Summary refinements: either preserve the full logical relation through lossless factorization or explicitly separate questions/coverage from exhaustive trace. | F02. Keeping semantics permits physical compaction; semantic pruning remains disallowed without a replacement contract. |
| <a id="RC03"></a>RC03 | Borrowed canonical output and all-window delivery convention: permit consuming charged output and next-ordinal private delivery before unrelated jobs finish. | F04. Keeping the barrier retains simultaneous results; completion/public visibility requirements remain unchanged. |
| <a id="RC04"></a>RC04 | Fixed row-oriented native realization in ADR-0145/0178 and storage §6.1: select a precise prepared packet and, where justified, compact block/extent ownership. | F05. Keeping row realization permits set reads and coalescing but leaves intrinsic per-row machinery. |
| <a id="RC05"></a>RC05 | ADR-0178's per-effect pending envelope and ordinary settlement protocol: permit operation/segment-scoped host discovery with exact native page outcomes/progress. | F06. Keeping the current protocol leaves repeated durable sidecars and settlement; local lookup tuning cannot close F06. |
| <a id="RC06"></a>RC06 | Generic terminal incidence causing complete attempt cuts in ADR-0178/storage lifecycle: define narrow terminal cleanup/history hazards separately from production eligibility cuts. | F07. Keeping full cuts requires a concrete per-page protection benefit; otherwise F07 remains. |
| <a id="RC07"></a>RC07 | Optional retention before completed-output handoff, including ADR-0174 preparation/lifetime decisions: permit separately owned retention after required readiness. | F08. Keeping ordering preserves the critical-path dependency and requires its benefit to be justified. |
| <a id="RC08"></a>RC08 | Public definition verification realization in storage §6.1–§6.2: scope verification to required definitions and permit protected operation-local reuse while preserving cold/snapshot checks. | F09. Keeping unconditional broad discovery preserves unrelated-DB growth cost. Whole-installation identity remains separately scoped. |
| <a id="RC09"></a>RC09 | Public role reconciliation in ADR-0156/storage publication: carry opaque protected expected roles/necessary established canonical properties into fresh actual reconciliation. | F10. Keeping full expected reconstruction leaves this repeated work; independent cold reconstruction remains mandatory either way. |
| <a id="RC10"></a>RC10 | Current observation helper effect contract: borrow an actual client/read owner or explicitly own and join a deliberate SDK session fork. | F12. Keeping accidental forks requires their checked finality to be established and extra lifecycle cost justified. |

F03 and F11 can initially be corrected within existing immutable-preparation and exact-output
contracts. If a remedy changes qualification, selected universes, error semantics or retention
lifetimes, expose that additional rule impact before implementation.

## Open decisions and material uncertainty

<a id="I01"></a>
**I01 — Admission assurance preparation and executed-property reuse.** Determine which
independent failure class each acquisition, revalidation and execution covers. Compare
operation-owned selected-capability preparation separately from reuse of an executed whole-input
property when earning assurance. `admission/scoped.rs:309–321` acquires capabilities before memo
lookup; reuse revalidates original capabilities at`:372–378`. `admission.rs:1594` excludes
ordinary memo skipping when `earn=true`, and scoped evidence rejects `Execution::WholeInput`
(`scoped.rs:100–103`). Removing `!earn` would not create the missing executed evidence.
Any reused property must bind exact ordered requested/resolved roles, complete immutable
dependency views, model/invariant/factory, profile, ledger/frontier, physical validity context,
original live pin and resource/operation owner. Successful computation alone, nominal descriptor
equality or a digest does not establish those premises. Preserve independent cold/incoming/
installed receiving admission, complete-empty applicability, cancellation/invalidation and
failure-atomic evidence publication. This is a **Proposed investigation**, not an established
defect or permission to bypass admission. A selected change to what qualifies as executed
admission evidence needs its own rule-impact assessment.

<a id="I02"></a>
**I02 — Server execution and resident amplification.** Existing captures identify atomic apply
as a substantial bucket in the bounded physical control. They do not separate server parsing,
planning, UDF evaluation, record/index operations, commit or server resident state. Host ingress
reservations cover client buffers, not all server evaluator/transaction state. That uncertainty
limits quantitative attribution and replacement selection; it does not negate source-established
amplification. Investigate these components if the selected remedy depends on their relative
importance. A new timing or environmental campaign is not a prerequisite for the structural fixes.

<a id="I03"></a>
**I03 — Complete recursive output and its consumers.** The failed run establishes at least77,824
acknowledged PairOutcome rows, not final cardinality or disposition distribution. Establish the
complete question, refusal, proof and explanation obligations before deleting or pruning trace
detail. Retain F02's distinction between lossless physical factorization and a deliberate semantic
contract change. The recursive comparison and source-grounded assertions remain uncompleted.
Static consumer analysis can settle the required contract without a historical benchmark.

<a id="I04"></a>
**I04 — Packet, block and segment realization.** Choose physical units together with their access
and lifecycle obligations: selected membership, nominal conflict detection, point/range lookup,
graph adjacency where required, evidence hydration, complete-empty/absence meaning, originals,
export/restore, retention and retirement. Compare ordinary row packets, compact immutable blocks
and operation/segment recovery using total machinery. No candidate is accepted merely because it
reduces effect count; none must retain the present per-row or per-window protocol. Existing
indexes have real readers; removing them requires replacing their access obligations. The
existing full schema and physical data may be regenerated in a later authorized design-phase
cutover, rather than retaining obsolete compatibility paths.

## Verification, disposition and next decision

The current evidence establishes the structural diagnosis. It does not qualify replacement code.
Reviewer builds, tests and service actions were **not_run**, by the read-only assignment.

The ordinary Summary comparison reacquires and prepares inputs independently, but invokes the
same underlying semantic kernel (`scoped_summary.rs` versus
`summary_production::{produce,produce_prepared_scheduled}`). Preserve that valuable
adapter-equivalence control and label it accurately. Keep source-grounded known answers for
recursion, guards, no-base cycles, proof accounting and witness closure. The recursive control
must eventually reach those assertions; the current timeout establishes none of them.

The next decision belongs to the coordinator and operator through plan creation: choose the
required Summary question/evidence contract and native packet/segment boundary, then compose
the independent acquisition, preparation, transfer and publication corrections around them.
Consequence priority favors F01/F02/F05/F06 and the F12 finality uncertainty. Prerequisite order
is different: output meaning and packet/recovery contracts precede optimizations that depend on them.

Until scheduled, **F01–F12 and I01–I04 are Deferred in this source review**, with this trigger: creation or
revision of the authorized publication-efficiency implementation plan. That plan becomes the
sole mutable disposition owner when it adopts them. No new register is needed.

Closure should combine source traces, minimal revealing correctness/failure controls and
complete-operation measurement after implementation. Preserve normal concurrency and retained
semantic guarantees. Contract-preserving changes keep their semantic assertions; deliberately
changed contracts need explicitly revised independent expectations, never relaxed assertions
merely to pass. Full Catalog/assembled qualification, public publication performance,
real-library differentiation and operator adoption remain separately scoped.
