# Compiled-product reuse contracts

**Review date:** 2026-10-09. **Purpose:** bounded independent change/target review of the
GR0/GR2/GR5 contract checkpoint, followed by a bounded inspection of integrated compiler
reuse. **Baseline:** `85e8b9cb` plus the concurrent, uncommitted implementation in the shared
working tree. Source anchors below describe the revisions inspected during this review;
line numbers can move as concurrent implementation continues.

The review applies design principles core 3.3, efficient-architecture heuristics 1.0,
code-intelligence profile 1.5, and the repository binding. Its conclusions are static
source assessments. They do not establish Tested or Measured claims, assembled plan
qualification, real-library performance, or operator adoption. The coordinator owns
decisions, finding disposition, and acceptance evidence in the active plans; this review
retains the finding identities and their source rationale rather than a second mutable
closure table.

## Scope and initial decision

The initial checkpoint covered portable product identities and manifests, workspace
capture/replay, native cache ownership and persistence, immutable preparation pins,
coalesced preparation, and service/session shutdown. It read the reuse plan §§3–4 and the
owning model, core, native-store, and serving consumers. Subsequent inspection covered
the integrated Local/Base/Body/SourceCall products, selected normalization products,
content domains, and the named SCC schedule cutoff. It did not review every compiler
operation, qualify general incremental scheduling, or inspect operator databases.

The initial decision was **revise the native unknown-write lifecycle before accepting
GR2**. GR5 had a concurrency-fit obligation concerning detached wait owners. The initial
semantic admission and provenance architecture was sound in the inspected normalization
routes, subject to preserving the distinction between cached data and consumer authority.
Later integrated inspection found a material public/private binding gap (F03) and a
canonical selected-product decoding gap (F04). These require correction before reuse can
be accepted at those boundaries. Later source corrections to F01/F02 are described below
as inspected implementation changes, not functional closure.

## Domain model and responsibility

| Owner | Contract assessed |
| --- | --- |
| `lctx-model::compilation_product` | Framed request and value identities, roles, exact prefix dependencies, complete section inventory, and successful products that accelerate computation without granting admission. |
| `cpg-core::workspace_products` | Binding to exact current inputs, typed canonical bodies, fresh ingress, and capture of the operation's singleton completed contribution. |
| Normalization and model admission | Candidate domains, complete outcomes, predecessor predicates and current premises that mint fresh consumer authority. |
| `lctx-surrealdb::product_cache` | Native persistence, equality/conflict behavior, capacity, leases, unknown completion, and retirement. |
| `lctx-serving::preparation` | Exact physical viewer pin, coalesced initialization, retained budget, cancellation, and close/drain. |
| Service and Python session | Request admission and preparation shutdown before native session invalidation. |

The semantic object is a derived compilation product, not a completed source contribution
or a serialized admission witness. An exact source view supplies current provenance and
coverage. A product supplies reusable data whose eligibility follows from current
dependencies and whose use must still satisfy the receiving model's predicates. Serving
preparation similarly belongs to the exact immutable physical pin, rather than merely to
a semantic snapshot identifier. The inspected changes introduced no generative or new
evaluation path.

Relevant source owners are [compilation_product.rs](../../../crates/lctx-model/src/domain/compilation_product.rs),
[workspace_products.rs](../../../crates/cpg-core/src/workspace_products.rs),
[workspace.rs](../../../crates/cpg-core/src/workspace.rs),
[product_cache.rs](../../../crates/lctx-surrealdb/src/product_cache.rs), and
[preparation.rs](../../../crates/lctx-serving/src/preparation.rs). The governing work is
the [reuse plan](../../plans/graph-compilation-reuse-and-invalidation-plan_2026-10-09.md)
and the [persisted-graph coordinator](../../plans/persisted-graph-execution-plan_2026-10-07.md).

## Private replay and necessary semantic revalidation

Private indexes and behavioral hints are data. A matching content identity does not make
them authoritative, and serializing a previously verified owner would not solve that
problem. The meaningful question is whether the receiving operation can establish its
required predicates against complete current premises before using the candidate.

The initially inspected normalization routes did this deliberately. Receiver
`prepare_view` checked independent candidates and complete outcomes. Event `prepare_view`
rebuilt admission from the current complete domain without requiring diagnostic producer
evaluation. Binding `prepare_view` checked enumeration and event/receiver applicability.
Normalization reuse reconstructed owners with current output and source providers. These
are necessary semantic checks, not an obligation to reproduce every producer's diagnostic
work. The initial anchors were receiver `prepare_view` around line 784, event
`prepare_view` around line 251, binding `prepare_view` around line 1315, and normalization
`reuse.rs` around line 35.

That distinction is valuable under H18–H20: keep necessary assurance while avoiding
producer replay that grants no additional consumer guarantee. It depends on retaining
the full candidate/outcome domain, current dependency ownership, and fresh budgets. The
initial generic private callback had no active private consumer in the checkpoint scope;
later behavioral consumers do. Their reconstruction validates hints against current
premises, but that alone does not bind separately cached public rows to the reconstructed
owner. F03 identifies the resulting gap. Neither a hash nor a private-owner reconstruction
is a substitute for this final binding.

## Findings

### F01 — Unknown native completion could outlive ownership and accounting

**Responsible component:** `lctx-surrealdb::product_cache`. **Initial severity:** acceptance
blocking lifecycle defect. **Principles:** DP03, DP19, DP20; FP04/FP05; assurance G5/G6.

At the initial checkpoint, `insert_owned` held the local lifecycle and stage filesystem
locks, but an unknown completion could return with a preserved stage while dropping those
locks (`product_cache.rs`, initially lines 195–223). Reaping treated an unlocked stage as
an orphan and reclaimed its accounting. Raw chunk creation, initially lines 258–262,
had no native generation/stage fence. The native loader's `write_failure` distinction
explicitly included remote unknown completion rather than rollback (`loader.rs`, initially
around line 677).

Consequently, a late final entry could become lookup-visible after unsuccessful
reconciliation, and late chunks could recreate state after cleanup or retirement without
remaining accounted ownership. An exact readback can acknowledge a complete observed
product. An absent readback cannot prove that an outstanding native mutation has become
terminal. The initial lost-ack control injected failure after complete publication; it
did not establish behavior of an actually delayed mutation.

The proposed correction had two separate duties. Visibility needs an atomic local
generation/stage/key/digest acknowledgement receipt created only after complete exact
native readback. Mutation and capacity need continued ownership or transactional native
generation and point-stage fences on every write. A receipt alone is sufficient to keep an
unacknowledged late entry invisible in the supported local-server lifecycle; it does not
make late writes harmless to quota or retirement, and it does not promise remote rollback.
Quarantine accounting must survive unless a transactional fence establishes that the
late effect can no longer mutate the retired or cleaned stage.

**Follow-up source inspected on 2026-10-09.** The corrected implementation uses version 2
native quota headers; transactionally guarded generation checks with `FOR UPDATE`; native
quota reservation; point-stage checks for chunk and final-entry publication; fenced stage
cleanup and entry deletion; and generation-changing retirement. Publishing an entry moves
the reserved state from stage to entry without releasing quota. Cleanup deletes the
stage and its unreachable chunks while releasing its reserved quota under the same native
fence. Retirement clears derived state and resets quota while changing generation.

Lookup requires the exact local acknowledgement receipt. The receipt is persisted through
a temporary file, file synchronization, atomic persistence, and directory synchronization
after full native equality readback. Entry deletion removes its receipt. Retirement
removes acknowledgement, entry, and stage assets under exclusive lifecycle/admission
ownership while retaining the gate inode. These source changes address the initial
visibility, late-mutation, and accounting defects architecturally.

The follow-up also identified two correction details. Incompatible native state must be
reset only during explicit installation, with the stored canonical lease directory
checked before reset; ordinary connection must fall through rather than repair it.
Orphan injection must charge the native quota it later releases. The latest inspected
source checks scalar version and directory before reset (`product_cache.rs`, around
lines 97–107), and the orphan control increments native `used` transactionally. The later
transaction anchors were reserve around 291, chunk around 298, publication around 313,
cleanup around 339, and retirement/assets around 374–383.

**Required closure evidence:** actual delayed native chunk mutation after cleanup, late
unacknowledged entry publication, and reserve/publication after retirement must establish
invisibility, fencing, and accounting. The coordinator was adding these controls; the
reviewer did not execute them. Inspected correction is not a Tested closure claim.

### F02 — Detached preparation wait owners lacked explicit bounded admission

**Responsible component:** `lctx-serving::preparation`. **Initial severity:** unresolved
concurrency-fit obligation. **Principles:** FP07, DP10, DP20; A4; H14/H17/H21.

Initially `load` registered an insertion owner and spawned a detached task per waiter
before Moka elected a shared initializer (`preparation.rs`, initially lines 86–117).
Initializer query/CPU admission happened inside `try_get_with`. Payload and capture
budgets bounded some retained state, but the detached erased future, Tokio task, Moka
waiter, and owner metadata did not have an explicit admission charge. Canceled requests
could therefore leave a population of wait owners for close to drain independently of
the one useful shared initialization.

The correction should bound this lifetime before spawning while retaining the intended
property that request cancellation cannot prematurely destroy a shared initializer or
its physical pin. This was an uncertainty in the design's resource bound, not evidence
that Moka's coalescing itself was incorrect.

**Follow-up source inspected on 2026-10-09.** `InsertionOwner` now retains an erased
reservation until terminality. The load path first registers against the closing fence,
uses a fast cache `get`, and reserves 4096 bytes before spawning for insertion metadata.
If that optional cache admission fails with Resource/Limit, it invalidates optional
retention, runs pending maintenance, and retries the reservation. This avoids having an
unused retained value prevent fresh preparation. The latest inspected load anchors were
around lines 89–99.

Close sets the closing fence and closes queued query/CPU semaphore admission. Submitted
work drains cooperatively; close waits for insertion owners and active requests, then
invalidates retained values and runs final maintenance. Borrowed preparation retains its
budget and pin through eviction. Request-admission suspension releases its query/CPU
slots while awaiting preparation, avoiding a waiter holding the slot needed by its
initializer. These changes address the original uncharged wait-owner uncertainty in
source. They do not establish a measured metadata bound or cancellation performance.

**Required closure evidence:** focused canceled-waiter, queued-close, borrowed-lease, and
optional-retention admission controls. Such controls exist or were being added in the
shared tree; the reviewer did not execute them.

### F03 — Revalidated private owners were not bound to cached public rows

**Responsible components:** `cpg-core::workspace_products`, Local/Base/Body/SourceCall
reuse, and final semantic admission. **Severity:** acceptance blocking semantic defect.
**Principles:** DP03, DP08, DP09, DP19, DP23; FP04; A2; G3/G6.

The integrated generic `reuse_product_checked_async` callback received `Arc<ProductPrivate>`
only. Public sections were checked separately for schema and canonical form. Local,
Base, Body, and SourceCall callbacks reconstructed private owners using current premises,
but the public rows replayed afterward were not compared with those owners before
ingress. Relevant consumer anchors were `local_semantics.rs` around 256–283 and
`semantic_execution.rs` around 265–279, 659–672, and 1094–1109. Their implementations in
`semantic_execution/reuse.rs` rechecked current frames, expressions, body roots, and
source events. This is useful semantic work; its output must govern the public product
that will actually be used.

`Produced<T>::borrow` checks premises and output-domain subsets, not content equality
between private owners and public rows. `bind_actual`, `bind_base`, `bind_body`, and
`bind_source_calls` gathered public outputs and retained the private owners without this
comparison (initial inspected anchors around 440–461, 525–542, 923–941, and 1333–1356).
Where a later consumer calls `ProducedEvaluations::actual`, it does check the private row
against the public row (`production.rs`, around 144–158). That is late and conditional on
consumption: it can turn a bad cache hit into an operation failure rather than a fresh
fallback, and an unused row need not reach that check.

A concrete counterexample is a canonical `ExpressionEvaluation` with its semantic key,
owner, and source/operand digests preserved but its non-key `boolean_value` changed. A
corrupted public product can carry a newly consistent product digest while retaining the
original private hints. Private reconstruction can mint the genuine owner for the old
row, while the altered public row enters the workspace. Final `admit_semantics` selects
Admission; `base_invariants` is DiagnosticReplay and is skipped there (`workspace.rs`,
around 971/1065; model selection around 648; execution `records.rs` around 263–290).
Admission fidelity for `ExpressionEvaluation` checks frame definition, qualification,
expression kind, source ownership/context and ordered source/operand digests, but does
not establish `boolean_value`, release, or status (`execution/fidelity.rs`, around
612–678). Thus the inspected final admission does not supply the missing binding.

This is not an argument for restoring full diagnostic producer replay. The correction is
to make the reuse admission see and bind the complete public inventory to the freshly
revalidated private owner, or reconstruct the public rows from that owner. Compare exact
rows and inventories before any cached ingress, including omissions and additional rows.
Final admission remains necessary for its own predicates.

**Required closure evidence:** valid-hash, schema-valid, canonical public-row mutation
with unchanged private hints must invalidate/fall through to fresh production before
ingress. Include unused changed evaluations, omitted/extra rows, and selected downstream
consumption. Concurrent corrections were still in progress; this review records the
source inspected on 2026-10-09 and does not infer their closure.

### F04 — Selected-product decoding normalized invalid stored inventories

**Responsible components:** normalization `kernel_products`, native body codec, and typed
row canonicalization. **Severity:** product-contract defect. **Principles:** DP08, DP09,
DP11, DP23; G6.

`entity_hit`, `callable_hit`, and `aspect_hit` in `normalize/kernel_products.rs` checked
Complete outcome, empty private payload, and a section's JSON count before decoding in
`TRANSFER_ROWS` windows (initial anchors around 54, 147, and 221). The native projected
builder selected declared schema fields and ignored extra object fields
(`projected_arrow.rs`, around 96–111). `decode_bodies` canonicalized each window before
encoding (`codec.rs`, around 147–150). Generated record decoding does validate that the
stored identifier equals the derived semantic identifier; missing ID validation is not
the defect.

Typed `Rows::insert` coalesces equal duplicates and rejects unequal ones
(`normalized/rows.rs`, around 167–176). Equal duplicates spanning transfer windows could
therefore pass per-window decoding and be coalesced, leaving the stored section inventory
different from the returned typed rows. Window reordering was also normalized rather than
rejected. Extra fields could disappear through projection. Canonical outer product JSON
and matching digests do not establish canonical typed bodies or global uniqueness.

The correction should compare stored body representations with the canonical typed
re-encoding and enforce global relation ordering, uniqueness, and count across windows,
using the shared canonical/hash path rather than a second test-only validator. A selected
product that violates these rules must miss/invalidate before emission.

**Required closure evidence:** extra-field bodies, duplicate rows spanning `TRANSFER_ROWS`,
and globally reordered windows must fall through before emitting cached results. Workers
were implementing this correction; its completion and functional evidence were not
established by this inspection.

## Preserved strengths and integrated boundaries

The initial cache compares complete canonical products for the same key; it does not
silently ignore conflicts. Operation capture uses the singleton completed contribution,
rather than prior cumulative contributions. Native `contribution_view` checks the
completed contribution/spec identity and logical descriptor and registers its singleton
view. Replay uses registered inputs, typed writes, ordinary completion, and current
descriptor/spec/outcome checks. These preserve source ownership when the product itself
has been admitted correctly.

Preparation keys include the semantic and physical `SnapshotHandle`, operation
definition, preparation kind, canonical roots, typed inputs, prefix/order, and incoming
fields. The prepared value owns its key, real resource budget, and physical lease. Moka's
approximate weight is not the sole budget authority. The inspected Python/session close
ordering drains shared active work, closes service preparation, then invalidates the
native client. Source/library fit supported using the existing coalescing cache rather
than a bespoke election protocol; this review did not repeat the coordinator's earlier
pinned Moka/SurrealDB documentation verification.

The finer selected domains inspected in `PreparedRootBatch::content_domain` run fresh
discovery against current scope memberships, include contract/prefix/order, exact source
row tokens, and explicit Present/Absent/Virtual root outcomes. Missing requested members
have explicit absent content identity. Native content tokens are restricted to immutable
registered views with ordered keys and membership ownership. Unsupported realizations
fall through. This supports negative-domain and matching-row invalidation in the inspected
scope program; it is not a certification of all compiler discovery completeness.

The SCC product is a credible named value cutoff. Its identity includes full nominal
vertices and typed arc/source/target topology, including isolated vertices and internal
edges. `summary_schedule::from_components` checks complete vertex inventory, uniqueness,
nonempty sorted components, strong connectivity, and a canonical condensation order that
rejects a nonmaximal split cycle. Current summaries rebuild provenance. This supports the
named invocation-SCC schedule; it does not establish general scheduler value equality.
The inspected dependency graph's additional traversal was used for trace logging rather
than a general incremental scheduling authority.

Optional resource failures deserve the same boundary discipline. Selected domain/hit
paths and generic private-validation reservation had Resource/Limit fallthrough in the
latest inspected source. Two further amplification edges were returned to the
coordinator: generic replay could acquire an additional ingress reservation after writer
setup and prior section effects, and optional product capture could propagate resource
errors from contribution-view scanning rather than abandon retention. All mandatory
cached replay resources should be admitted before irreversible ingress; optional capture
must not turn already successful compilation into a resource failure. These observations
are bounded integration obligations, not a claim that unknown native failures can be
swallowed. Concurrent fixes were not presumed from worker intent.

## Assurance and execution-fit judgment

The following records the **initial checkpoint assessment**, preserving its original
assurance boundary rather than overwriting it with later implementation status.

| Gate | Initial assessment and limit |
| --- | --- |
| A1 | Satisfied for the inspected explicit derived-product and physical-preparation models. |
| A2 | Violated by F01 lifecycle ownership; inspected normalization admission/provenance otherwise satisfied. |
| A3 | Satisfied for the bounded composition examined. |
| A4 | Unresolved for canceled wait-owner amplification and unknown native completion. |
| G1–G4 | Passed static inspection of the initial data/admission scope. This predates the integrated F03 consumers. |
| G5–G6 | Failed static inspection because of F01. F03/F04 subsequently exposed additional integrated obligations. |
| G7 | No positive qualification claim: the plan was unfinished and the initial proposed ADR template was not accepted implementation evidence. |
| G8 | Passed the inspected preservation and scoped-change boundary. |
| CI-G1/CI-G2 | Passed for inspected attributed inputs and typed/admitted reuse routes, with broader answers outside scope. |
| CI-G3 | Not applicable to an unchanged evaluation path; no retrieval-quality claim made. |

Execution fit is qualitatively favorable where stable preparation and exact products
amortize repeated work (H12/H14), and where unnecessary producer replay is separated
from admission (H18–H20). F01 violates bounded lifecycle and capacity expectations
(H21/H23–H25); F02 exposed combined waiter state (H17/H21). Source corrections improve
those fits. F03 demonstrates why fewer semantic checks cannot be chosen merely because
content identities match: the authority boundary must bind the data consumed. F04
demonstrates why canonicalization must reject invalid stored representations at admission,
rather than silently changing their declared inventory. No new rule, register, cost model,
or additional review machinery is recommended.

## Evidence and handoff

Static source, type, plan, role, and principle inspection was sufficient to diagnose the
listed contract defects and assess proposed/source-inspected corrections. Review evidence
was obtained with scoped `rg`, source reads, and dirty-tree inspection. No separate probe
folder was necessary.

**Verification:** `cargo check`, affected functional controls, and `just qualify` were
**not_run by this reviewer**, explicitly outside the delegated boundary. No native fixture
was provisioned, no operator state was inspected, and no performance was measured.
Existing or newly added test source is not an executed result. The coordinator retains
functional evidence and disposition in the active plans and must not promote this static
review to whole-plan qualification.

The initial review required revision of F01 and resolution of F02's admission bound.
Follow-up source inspected on 2026-10-09 contains concrete corrections to both. Integrated
inspection requires public/private binding (F03) and exact canonical selected-body admission
(F04), with the optional-resource integration obligations above. Acceptance belongs to the
coordinator after those boundaries are implemented and their minimal revealing controls
are established.

## Follow-up source assessment — 2026-10-09

This second bounded follow-up inspected HEAD
`85e8b9cb9bf8a2941a2daea48f3ccaf258523879` plus the stable integrated dirty source after
the coordinator's corrections. It preserves the historical findings above. Its conclusion
is **source correction of the four reported mechanisms is established; functional closure
is not assessed, and one optional-capture error-conversion issue remains**. No Cargo,
functional control, formatting, native provisioning, or operator action was run.

For F01, native format 3 separates the immutable generation header from authoritative
mutable `product_quota:current`. Reservation reads quota with `FOR UPDATE` and updates it
in the same transaction as stage creation. Every stage/chunk/publication/deletion mutation
uses the generation read fence; chunks and publication also require the point stage.
Cleanup removes the point stage and releases reserved quota transactionally. Retirement
changes generation and clears quota and derived native/local assets under exclusive local
lifecycle ownership. Exact local acknowledgement remains a prerequisite for lookup.
Splitting quota from generation permits independent stage/chunk effects without making
ordinary quota updates change the retirement identity. The inspected generation/stage
conflict mechanism addresses the original unknown-effect authority defect; only executed
delayed-native controls can establish the pinned native realization's behavior.

For F02, the inspected code retains the pre-spawn charged owner, fast lookup, optional
retention eviction/retry, closed semaphore queues, cooperative submitted-work drain, and
borrower-owned charges and physical pins. No additional substantive defect was found in
this bounded lifecycle inspection. This is not a measurement of the 4096-byte metadata
allowance or a cancellation throughput claim.

For F03, `UpperStage::reuse` explicitly returns Fresh for behavioral Local, Base,
Completion, and SourceCall; their direct producer paths no longer perform product replay.
Catalog-profile row reuse remains a separate branch. The portable product has no private
payload, and repository source search found no `ProductPrivate`, cached hint format,
`rebind_product`, or private-product setter. The prior semantic-execution hint module is
gone. Fresh computation mints the behavioral owners and their public rows together, so
the historical public/private disconnect is removed from those families rather than
treated as safe through matching hashes. This is consistent with the initial
first-principles distinction: when establishing full current equivalence repeats the
kernel, replay adds work and fresh production is appropriate. No claim is made that a
schema-valid arbitrary behavioral row is semantically admitted merely by content identity.

For F04, all three selected hit decoders call the shared
`workspace_products::validate_product_rows` before constructing outputs. It checks the
section inventory/count, decodes typed windows, compares exact canonical re-encoded bodies
with each original window, and retains one `RelationContent` across windows.
`Relation::hash_rows` rejects non-increasing identifiers globally. Together these reject
extra projected-away fields, reordered bodies, and equal duplicate identifiers across
transfer windows before selected output emission. The original lossy-normalization
mechanism is corrected in the inspected source.

Generic replay now calls `prepare_replay` before mutation. All decoded typed batches and
their retained budget are prepared before registration/writer/native ingress. The cache
lease and cached-byte charge are dropped only after this preparation succeeds. A
preparation Resource/Limit refusal returns to fresh production without invalidating an
otherwise valid product. This addresses the earlier late cache-only allocation concern;
mandatory native ingress remains an owned operation with ordinary failure semantics.

The outer `retain_product` wrapper now abandons capture on direct Resource/Limit errors.
However, the inspected `retain_product_owned` still converts a streaming error through
`stream.try_next().await.map_err(ModelError::codec)` (line 165 in the inspected revision).
`compiler_provider::scan_batches` wraps `ProjectedBuilder::release`, `push`, and `finish`
errors as `DataFusionError::External` (lines 669/672/677/684). Those methods can refuse
resource reservations during polling. `ModelError::codec` then flattens that typed refusal
to Codec text, so the outer Resource/Limit branch cannot recognize it and optional capture
can still fail an already successful producer. Preserve/classify the typed external model
cause at this conversion, while continuing to propagate unknown native/ownership failures.
Merely wrapping the DataFusion error in `ModelError::Cause` does not make the existing
`primary()` recognize Resource/Limit. This remaining integration issue was returned to
the coordinator; it has no separately claimed functional result here.

The selected `retain_entity`, `retain_callable`, and `retain_aspect` paths encode already
owned typed rows after optional allowance admission; they have no corresponding streamed
capture conversion. `analysis_graphs.rs` also has a stream-to-codec conversion, but that
path loads mandatory fresh topology rather than optional retention. No further occurrence
of this optional-capture defect was found in those bounded related consumers.

Follow-up source anchors:
[native generation/quota/acknowledgement](../../../crates/lctx-surrealdb/src/product_cache.rs),
[preparation load/close](../../../crates/lctx-serving/src/preparation.rs),
[reuse eligibility](../../../crates/cpg-core/src/compilation.rs),
[fresh Local](../../../crates/cpg-core/src/local_semantics.rs),
[fresh behavioral execution](../../../crates/cpg-core/src/semantic_execution.rs),
[prepared replay/canonical validation/capture](../../../crates/cpg-core/src/workspace_products.rs),
[selected decoders](../../../crates/cpg-core/src/normalize/kernel_products.rs),
[stream error conversion](../../../crates/lctx-surrealdb/src/compiler_provider.rs), and
[global relation content order](../../../crates/lctx-model/src/domain/model.rs).

The following SHA-256 receipts identify the inspected dirty source exactly; they are
source receipts, not qualification evidence. Paths are relative to `crates/`.

| Source | SHA-256 |
| --- | --- |
| `lctx-surrealdb/src/product_cache.rs` | `99fa0bfd7568f4d26907c0dabcec60c3c287d7628671805fda66b4f3aa1fb3c2` |
| `lctx-serving/src/preparation.rs` | `2b3911b3f6d6c46def563ef8f367a47e7c9dafdbb2470bf0c5a61aa356ee5c84` |
| `cpg-core/src/workspace_products.rs` | `2b6ad53909db0473f710c3da9964c1f5574604270caf26f72f6886df6764906d` |
| `cpg-core/src/normalize/kernel_products.rs` | `29c9181332ec7f9235a16ae3216299897a609d4de04406dee2761a2be0429122` |
| `cpg-core/src/compilation.rs` | `684b2c9e2baa622557b3c72f046fb054a478f88c9e3ded0b4f0c61ca987b2b73` |
| `cpg-core/src/local_semantics.rs` | `68af706849b2ff5e2475c4eb5b032dee697f45593fe9271199bae4d17008022e` |
| `cpg-core/src/semantic_execution.rs` | `b71558e0f70e0b92564489aad1be040bfc370e06f7bf3db96847ae92b5074c88` |
| `lctx-model/src/domain/compilation_product.rs` | `8da2f2d7fce3f264fa48b41386532979ca7ca9a89447f4ea7bb862c838238e49` |

The coordinator owns current dispositions and the functional evidence needed to close
F01–F04. This follow-up changes no historical finding ID or historical assurance judgment.

## Follow-up of capture and retention corrections — 2026-10-09

This further source-only inspection uses the same HEAD
`85e8b9cb9bf8a2941a2daea48f3ccaf258523879` with subsequent concurrent corrections. The
coordinator reported pressure failures and teardown waits against the earlier preparation
source. Those are coordinator-reported results, not tests run by this reviewer, and the
replacement bodies had not yet been functionally established at this inspection.

The optional streamed-capture conversion described in the preceding follow-up is now
corrected in source. `product_capture_error` recovers a directly wrapped
`DataFusionError::External(ModelError)` as the original typed model error; other errors
retain an owned cause. Resource/Limit can therefore reach the optional-capture refusal
branch without flattening. Completion/unknown native effects remain fatal. The new unit
control examines both refusal and unknown completion, and a native pressure control
examines survival of completed output; neither was executed by this reviewer.

Preparation now holds a replaceable `Mutex<Cache>` retention generation.
`release_optional_retention` swaps and drops the complete optional cache owner. A load
retains a clone of its election generation through initialization and drops that clone
before resource retry. Active initializer clones and external prepared-value `Arc`
borrowers remain owned and charged. Close first fences requests, closes queued admissions,
and drains insertion owners/requests; it then replaces the optional cache and waits for
external value leases. No mutex guard crosses an await in the inspected accessors.

This addresses the earlier assumption that logical invalidation plus maintenance implies
prompt destruction. The pinned local Moka 0.12.16 source supports the owner-drop mechanism:
`future/base_cache.rs::Inner::drop` flushes deferred garbage and its owned cache store drops
remaining values in place; `BaseCache` clones retain the shared inner. This was a local
source inspection, not an independently executed library probe. Replacing a generation
under pressure can cause another election for the same physical pin while an older
initializer is still live, but it does not release that owner's budget or lease. This
bounded amplification is charged through the insertion-owner and preparation budgets;
no measured byte-exact allowance or throughput claim follows.

Two remaining edges were returned to the coordinator:

1. **Preparation captures precede the retention retry.** `hydrate` reserves
   `viewer-preparation-layout-capture` and `viewer-preparation-closure-capture` directly
   before entering `load` (inspected lines 183 and 193). A small exact key can succeed
   while the larger capture reservation fails because of unborrowed optional retained
   values. Neither `load`'s insertion retry nor its initializer retry is reached. These
   required capture reservations need the same release-optional-retention/retry policy,
   preserving live layout/root/borrower charges. The inspected pressure controls exercise
   insertion and initializer admission, not this pre-load capture boundary.
2. **Typed uncertainty must reach the semantic fallback guard.** Generic reuse now
   propagates a predicate/preparation error when `!permits_storage_cleanup()` or
   `has_committed_effect()` before invalidation or fallback. That guard is correct for
   typed completion errors. However, current normalization admission queries convert
   DataFusion query/stream errors through `ModelError::codec`: receiver roots around
   67/70/71; event roots around 163/164 and 194/197/198; binding roots around 289/292/293.
   Shared hydration in `consumed_rows` has related conversions. A native provider's
   typed unknown error can consequently reach the new guard as Codec text, which permits
   cleanup and has no committed-effect metadata. The native store records uncertain
   failures and should prevent eventual successful publication, but that does not
   establish the intended immediate propagation before valid cache invalidation/fresh
   fallback. Preserve the source cause through these live callback boundaries, or consult
   the native owner's failure/terminal state before interpreting rejection as a cache
   miss. This is a bounded integration obligation for current-premise checks, not a
   request to reintroduce diagnostic replay.

Additional source receipts for this inspection, relative to `crates/`:

| Source | SHA-256 |
| --- | --- |
| `lctx-serving/src/preparation.rs` | `b925b3e0c02a231a512aa99bdc93139ca990b237caa03cc3777160f0dd16fefb` |
| `cpg-core/src/workspace_products.rs` | `89f5b571c03d2c63a1ab9f63934cdf592e26f003a8beeb481f1482245a44a330` |
| `cpg-core/src/normalize/admission.rs` | `d7dd522bb0eea68aea8f11021a93e13d5efd343d3665737044b638c69a711606` |
| `cpg-core/src/consumed_rows.rs` | `e4fa11f98a07f422456b417960876227a23e2d27793100ce826d50ffa8e75785` |

The capture-conversion and cache-owner corrections are source-inspected implementation
changes. The two remaining edges and repaired controls still require coordinator
disposition and functional evidence. No historical F01–F04 judgment or finding ID is
overwritten by this follow-up.

## Final bounded source follow-up — 2026-10-09

The final inspection of the two preceding integration obligations used HEAD
`85e8b9cb9bf8a2941a2daea48f3ccaf258523879` plus the corrections identified below.
**Verdict: both obligations are corrected in inspected source; no remaining substantive
blocker was found within this bounded contract review.** This verdict does not supersede
the historical findings, establish functional closure, or qualify the whole reuse plan.

`PreparedCache::reserve_capture` now retries a Resource/Limit refusal after replacing the
optional cache owner. Both pre-load layout/closure capture reservations and insertion
owner admission use it. Borrowed layouts, active initializer generations, keys, roots,
and prepared values keep their real charges. The revealing pre-loader pressure control
reserves capture bytes with insufficient free space until unborrowed optional retention
is released. Its source exercises the previously missed boundary; this reviewer did not
execute it.

`sql::model_error` preserves provider model errors through External and Context wrappers,
and retains shared DataFusion errors when their shared owner cannot be unwrapped.
`sql::product_fallback_allowed` walks model causes, shared model causes, and error source
chains. It rejects uncertain completion and committed effects and inspects every element
of a DataFusion Collection, rather than relying on that type's first-only `source()`.
Generic reuse applies this guard before invalidation or fresh fallback. The normalization
admission, scoped admission, callable-scope, and consumed-row stream paths inspected now
use this typed conversion. This establishes the intended boundary: a failure to observe
current premises cannot be reinterpreted as proof that a disposable product is corrupt.
The shared-unknown-wrapper control covers the non-unique shared-error case in source.

Some consumed-row setup-only conversions still use Codec: selected-provider scan/plan
execution around 1174/1177 and 1267/1270, and query `execute_stream` around 1380/1524.
These were returned to the coordinator as an error-preservation limitation. In the
currently inspected native adapter, `NativePartition::execute` defers native read setup
to stream polling, where the corrected conversion preserves its typed errors. Consequently
those remaining setup conversions do not demonstrate a current unknown-native bypass
and are not an additional bounded acceptance blocker. No general guarantee for future
setup providers is inferred from the current adapter's deferred execution.

The coordinator reported a passed release compile check for `cpg-core` and `lctx-serving`
including tests (2.18 seconds), and a repaired 39-test functional selection building with
the compiler cache matrix. Those are coordinator-reported state: the repaired functional
results were pending when this follow-up was written. Reviewer verification remains
**not_run** for Cargo and functional controls. Root retains acceptance and evidence
ownership.

Final inspected dirty-source SHA-256 receipts, relative to `crates/`:

| Source | SHA-256 |
| --- | --- |
| `lctx-serving/src/preparation.rs` | `0b04216c52a580110fb479c39660b9781d669a046e22762fcad1ab8d96fac50b` |
| `cpg-core/src/sql.rs` | `41b2b368ba1746cf1cb49e3ccbe6d88b5aef3c9360b705264515d2212bbed46b` |
| `cpg-core/src/workspace_products.rs` | `4030d49a5cd2116b59de54d134a01c4411def0f6f0dbcf6ac5c5cf08a0c2a751` |
| `cpg-core/src/normalize/admission.rs` | `5dc6528c742b90c17c16a9e23a856c19bac21759d91660893704d4639d67fea6` |
| `cpg-core/src/scoped_admission.rs` | `81a846baff01f44b05f2c1884cd54d000c44c09882481794f6c59228dc6db8c6` |
| `cpg-core/src/normalize/callable_scope.rs` | `8fbb3898b603317a30c142b64dfcc4c0cb34862bf34b6dd3420ca73c2eebe206` |
| `cpg-core/src/consumed_rows.rs` | `cecf822f5ee7d079d09eb1e2dc95efa5eb74ba49ac7fbd1b7c3ba212f434139c` |

## Bounded coordination metadata follow-up — 2026-10-09

The coordinator subsequently identified a steady-state capacity obligation: full-key
entry-lock filenames were created even by misses, and per-nonce stage-lock filenames
survived until retirement. Native byte quota alone did not bound that filesystem lock
metadata. This extends F01's capacity/ownership rationale; it does not create another
mutable disposition authority.

The inspected coordination version 4 correction uses separate fixed sets of 4096 entry
and 4096 stage lock stripes. Each path derives from the first three hexadecimal digits
of a content hash of the complete entry key or staging nonce. Full native keys, staging
identifiers, exact value comparison, receipts, and point-stage fences retain their
independent meaning. Collisions only share synchronization. Misses and unknown reserve
attempts therefore create at most these 8192 stable lock paths, including attempts that
never produced a native stage. The prior proposed per-nonce unlink scheme would have
needed reconciliation of such local-only orphan files; fixed stage stripes avoid that
obligation.

Stripes are not unlinked during ordinary eviction or stage cleanup. They are removed only
by exclusive admission/lifecycle retirement or incompatible explicit installation, after
native generation reset. Ordinary connection never migrates coordination. This preserves
cross-process inode ownership through live borrows and avoids splitting a live lock
between an unlinked old inode and a replacement path.

Insertion reads an existing entry under the shared entry stripe. Corrupt replacement
releases that pin before attempting eviction and refuses retention if a borrowed stripe
prevents removal. Final publication uses a nonblocking exclusive stripe attempt: a
collision with an external borrow returns optional refusal and cleans its reserved stage
through the existing native fences, rather than waiting on a lease its caller might own.
Capacity inventory verifies native entry ID/key agreement before attempting deletion.
Stage owners use shared stage stripes; reaping attempts exclusive ownership and
conservatively retains a colliding live stage. All native accounting and late-effect
fences remain in force.

**Production source verdict:** the bounded stripe correction preserves the inspected
borrowed/cross-process lease contracts and addresses miss/unknown-attempt lock metadata
growth. No production blocker was found in this bounded assessment. Hash collisions do
not equate products or authorize a hit; conservative collision refusals can reduce reuse,
without changing fresh compilation semantics.

One concrete control repair remained in the inspected source:
`native_subprocess_entry_lease` still opened `entry_<fullkey>.lock` around line 511, while
production now opens the hashed stripe. The parent capacity control passed that full key
to the child. Consequently it no longer established cross-process ownership of the
production lock and required the shared stripe helper or exact production path before
rerun. The new native collision control correctly constructs two distinct full keys with
one stripe, holds the first borrow, requires the second insertion to refuse without
waiting, then checks both exact values after release. Neither control was executed by
this reviewer.

The previously noted DataFusion setup-only conversions in consumed rows were also
inspected as corrected to `sql::model_error`; integer/schema/I/O conversions remain Codec.
The coordinator reported a passed final release compile check of native/core/serving
including tests (3.18 seconds). The next functional target run was pending. These are
coordinator-reported verification state, not reviewer-run results.

This inspection retained HEAD `85e8b9cb9bf8a2941a2daea48f3ccaf258523879` and the following
dirty-source SHA-256 receipts, relative to `crates/`:

| Source | SHA-256 |
| --- | --- |
| `lctx-surrealdb/src/product_cache.rs` | `5e254787f46284f391153e338662c7c8070da9dec05647f572c35b3bbe577525` |
| `cpg-core/src/consumed_rows.rs` | `599597597c2567134da2fb3833cb439f49093dc0f638ea290c2be48a8469c7d7` |

Reviewer Cargo and functional controls remain **not_run**. This is bounded source
assessment, with the control repair returned to the coordinator before functional
closure; it is not a whole-plan qualification claim.

## Explicit incompatible-schema reset follow-up — 2026-10-09

This bounded inspection examined the latest version 4 explicit installation, which now
removes incompatible private tables and their obsolete field/index definitions before
rebuilding. It used HEAD `85e8b9cb9bf8a2941a2daea48f3ccaf258523879` and native cache source
SHA-256 `f60c86bc167c413afef8776a6133d12748ada744eea64c5e640c3fd4df4453b6`.

The normal ownership ordering is correct: installation canonicalizes the configured lease
directory and takes exclusive lifecycle ownership; scalar version/directory inspection
precedes destructive reset; a stored foreign directory is refused; ordinary connection
does not migrate. Removing table definitions solves the prior row-only reset's obsolete
required-field problem. The updated native reset control adds an obsolete required header
field and checks foreign refusal before same-owner rebuild. This is source inspection;
the reviewer did not run that control.

**Reset ownership blocker:** the removal list includes `product_header`, which contains
the only persisted directory-owner receipt. Filesystem lifecycle locks coordinate users
of that directory, but an installer configured with another directory has independent
locks. After installer A removes the header and before it recreates it, installer B can
observe no version/directory, treat the database as unclaimed, and create a header owned
by B. A crash after header removal extends this window. A directory comparison made
before removal does not prevent this concurrent or subsequent foreign claim. Preserve
native coordination ownership throughout reset: either retain the header's ownership
record/table while removing obsolete schema definitions, or keep a stable native owner
receipt outside the disposable tables and require it before any rebuild.

**Late-effect fence obligation:** the destructive DDL sequence also replaces retirement's
explicit point-header generation update with reliance on `REMOVE TABLE` conflict
semantics. Local exclusive locks drain local owners but cannot establish remote unknown
terminality. Mutate the established generation fence before destructive DDL, or supply
pinned source/control evidence that an old transaction's header read necessarily
conflicts across removal and recreation. The existing delayed-retirement controls exercise
the explicit update protocol; the incompatible-schema control does not yet exercise a
delayed transaction crossing DDL reset. A new generation after rebuild alone is not
evidence about a transaction that already read the old generation.

These are reset-specific extensions of F01's coordination and unknown-effect obligations.
The earlier bounded production-stripe verdict remains limited to its inspected revision;
it does not certify this subsequently changed reset sequence. The ownership gap was
returned to the coordinator as a substantive blocker. The reviewer performed no Cargo,
tests, provisioning, or operator action and changed only this review manuscript.

## Persistent coordination-owner correction — 2026-10-09

The version 5 source inspected at HEAD
`85e8b9cb9bf8a2941a2daea48f3ccaf258523879`, native cache SHA-256
`e9c564e9cf2810024bb7c63c6e634e9acb2dcf64b80b119bbc0d7aead54aad6e`, adds immutable
`product_owner:current` directory authority outside the reset table set.

**Ownership correction verdict:** the preceding foreign-directory reset blocker is
corrected in source. Installation rejects a persistent foreign owner before header DDL.
When adding the receipt to an older store, it checks the older header's directory before
creating ownership. Singleton CREATE cannot overwrite a concurrent winner; a losing
installer stops before destructive reset. The owner survives header removal, interruption,
generation retirement, and schema rebuild. Ordinary connection now requires that owner.
Same-owner recovery does not permit a second directory to claim an absent header.

The source also explicitly updates the old header generation before destructive DDL,
addressing the earlier reliance on table-removal conflict semantics for already executing
product transactions. The strengthened reset control covers obsolete required fields,
foreign refusal, an old delayed transaction, absent-header foreign refusal, and same-owner
recovery. Its delayed transaction was opened before the fixture's obsolete-field and
version updates, however: those earlier header point mutations already cause a conflict.
To reveal the reset's own fence, open/read that transaction after the fixture header
updates and before invoking reset. This control-isolation issue was returned to root;
no control was executed by the reviewer.

**Remaining unknown administrative-effect obligation:** the generation update and table
removals in `install` are separate bare native effects. An unknown table-removal result
can release local lifecycle ownership without proving remote completion. A later
same-owner recovery may rebuild the table before the earlier removal executes, allowing
the delayed administrative effect to remove current state. The generation update fences
old product transactions; it does not make each subsequent reset effect generation-bound.
Likewise, retirement's destructive native transaction follows a separate `current()` read
rather than checking its expected generation inside that transaction. If retry/recovery
after unknown reset or retirement is supported, fence these effects themselves against
the expected generation in native transactions, or retain quarantine that requires
confirmed native quiescence before recovery. An atomic guarded reset or a stable
generation authority outside the removed tables can supply that boundary. Do not infer
remote rollback from an unknown response.

The persistent owner resolves directory authority across reset, but does not by itself
establish administrative terminality. This remaining reset/retirement obligation is an
extension of F01 and was returned to the coordinator. Reviewer Cargo/native controls
remain **not_run**; root was compiling and running actual controls. Historical findings
and earlier source receipts remain intact.

## Administrative transaction and quarantine correction — 2026-10-09

This inspection uses HEAD `85e8b9cb9bf8a2941a2daea48f3ccaf258523879` and frozen native
cache source SHA-256 `e0efef1042fe8913a92eb96b673ab422c67ad6b8d88d47af64f02e25c57a2671`.

The persistent directory owner now also holds the administrative generation token.
`installation_body` checks that token with `FOR UPDATE`, rotates it, mutates the product
generation, and performs destructive DDL/current schema/header/quota creation inside one
native transaction. The owner table remains outside the reset set. Compatible explicit
installation also rotates the administrative token while preserving compatible product
generation and values. `RETIREMENT_BODY` checks and rotates that same token and checks the
expected product generation inside its destructive transaction. Reset and retirement
therefore share an administrative epoch; a superseded late administrative effect cannot
remove a subsequent install's current state.

Before submitting administration, `admin.pending` is created and both its inode and parent
directory are synchronized. Failure or unknown disposition retains quarantine. `current()`
returns false and read-only connection returns unavailable while that marker exists;
retirement of a quarantined instance names explicit installation reconciliation rather
than pretending the effect completed. Only confirmed fenced administration clears the
marker. Local generation assets are removed after confirmed reset/retirement, under
exclusive lifecycle ownership. This corrects the preceding administrative-effect and
read-only-resumption authority obligations in inspected source.

The actual pending-transaction control invokes the same production installation and
retirement bodies. It checks a newer rebuild/value before late DDL commit refusal, then
checks quarantine, read-only refusal, compatible reconciliation, late retirement refusal,
and preservation of the current product. The reset fixture's delayed transaction now
starts after its obsolete-field/version changes, isolating the reset fence. The subprocess
lease control uses the shared production coordination-path helper. The previously
reported control-isolation issues are corrected in source.

Root reports the final frozen 13-control native run **passed**, run
`20261009T150919.600Z-955ea5`, Nextest `92db742b-b9bc-4f9d-90c6-e5aaa85c7358`:
0.513 seconds of controls after a 36.25-second release build. Root also reported the prior
atomic-DDL/old-generation 13-control run passed, `20261009T150731.463Z-931937`. These are
coordinator-provided functional receipts; the reviewer did not execute Cargo or native
controls. They qualify those controls, not the whole reuse plan.

**Residual capacity-metadata obligation:** if reset/retirement commits but its acknowledgement
is lost, the local call can return before `remove_generation_assets`. Explicit recovery
then observes a compatible current header, rotates the administrative token, and clears
quarantine without reclaiming acknowledgement files for the retired product generation.
Repeated committed-but-unacknowledged administrations can accumulate such orphan receipt
files despite fixed lock stripes. After confirmed fencing and under exclusive lifecycle,
compatible reconciliation can remove receipts whose stored generation differs from the
current product header, preserving all current-generation receipts and products.
Alternatively, a durable pending-administration record can identify the cleanup owed by
the observed committed generation. The current delayed-administration control establishes
uncommitted transaction fencing; it does not exercise committed administration with lost
acknowledgement before filesystem cleanup. This bounded F01 capacity obligation was
returned to root without changing historical finding IDs or disposition ownership.

The administrative authority/fence corrections are sound in source and have the reported
focused native evidence above. The residual receipt-reclamation edge prevents an
unqualified claim that all filesystem metadata remains bounded across unknown committed
administration. Reviewer verification remains **not_run** and only this manuscript was
edited.

## Final receipt-reconciliation assessment — 2026-10-09

The narrow final correction was inspected at HEAD
`85e8b9cb9bf8a2941a2daea48f3ccaf258523879`, native cache source SHA-256
`27eabcb327aeee5221aa43405ead8d493f6e8e3cab11f8c08c015ca97a3da326`.

`read_acknowledgment` rejects envelopes exceeding 2048 bytes before reading/parsing and
treats absent or invalid envelopes as unacknowledged. Explicit installation now reads and
checks the current product header after the confirmed administrative fence, then invokes
`reconcile_acknowledgments` while retaining exclusive lifecycle ownership. The scan removes
invalid or non-current-generation receipts and preserves current-generation receipts.
Only afterward does it clear and synchronize the pending-administration marker. Cleanup
failure consequently leaves admission quarantined rather than reporting reconciliation
complete. Existing directory/owner/header checks and late administrative fences are
preserved.

The same native delayed-administration control now also commits the actual
`RETIREMENT_BODY` while deliberately omitting local asset cleanup, retaining the pending
marker and old acknowledgement. Compatible explicit recovery must remove that old receipt
and return no product. This is the revealing committed-but-unacknowledged case missing
from the previous delayed-uncommitted-transaction control; it exercises production
administration and reconciliation rather than a duplicate test-only algorithm.

Root reports the final 13-control native run **passed** on 2026-10-09:
`20261009T151353.336Z-db7117`, Nextest `7809d7fa-1890-444c-98aa-d1c9c32f46d5`,
0.532 seconds of controls after a 34.94-second release build. This receipt applies to the
final narrow correction and supersedes neither the older receipts nor their limits. The
reviewer inspected source and the control body, and did not run Cargo or native tests.

**Final bounded verdict:** the receipt-reclamation obligation is corrected in inspected
source and has the coordinator-reported revealing native evidence above. No remaining
substantive blocker was found within the assigned compiled-product ownership, admission,
canonicality, leases, capacity, administrative recovery, and shared-preparation contract
scope. This is not whole-plan qualification, a performance claim, or operator adoption.
F01–F04 retain their original IDs and rationale; their current dispositions and wider
acceptance evidence remain with the coordinator and active plans. Only this manuscript
was edited by the reviewer.

## SCC eligibility follow-up, 2026-10-09

**Implemented / acceptance in progress; static refinement.** The earlier SCC finding above
established a pure value contract and fidelity, not avoided computation. Subsequent execution-fit
inspection found that the actual restoration validator repeats forward/reverse component
traversals and canonical condensation ordering, then adds topology hashing, lookup and comparison.
The coordinator therefore keeps this consumer Fresh under beneficial-reuse eligibility. Each
prepared invocation graph retains one computed schedule, bound by a private nonserialized
materialization identity; a schedule cannot attach to a separately materialized graph with
identical keys or topology. Canonical complete nominal topology identity remains available for
other useful pure contracts. The active companions and coordinator own this disposition and
matching controls; no numerical speed claim or persistent SCC cutoff is established.

## Entity computational-demand follow-up, 2026-10-09

**Implemented / source-inspected; integrated runtime acceptance pending.** Candidate controls
exposed root-dependent `symbol_entity_resolutions` conflicts. The native/finite scope selects
complete reverse candidate families for virtual computational roots, while ordinary nominal
ports retain supporting lookup symbols. The former kernel computed resolutions for all selected
symbols, including dependencies without their candidate families. The correction adds typed
Symbol/SyntaxField/Public/Enumeration output demands and removes rootless scoped entry points.
Requested row presence, virtual port/type correspondence and missing selected premises still
fail closed. Parameter and field work follows the demanded symbol; Public resolves every exact
origin/context/module/name alternative but skips ancillary work. Enumeration preserves existing
qualification/support/run predicates. Full normalization preserves its prior refusal order.
The inspected mixed-cache path maps each surviving partition to its original root through the
ordered miss mapping, while domain identity continues to bind that root and complete membership.
Independent bounded source assessment found no substantive issue. Finite demanded-versus-full
normalization and actual native controls remain the active coordinator's acceptance obligation;
source acceptance alone does not close the failing candidate or whole-plan scope.

## Advertised callable-owner follow-up, 2026-10-09

**Implemented / independently source-inspected; runtime acceptance in progress.** After entity
demand was corrected, both native normalized frontiers completed normalization but refused
callable admission because nominally reached supporting callables contributed claims to the
requested owner's selection. The independent reviewer assessed the frozen correction at HEAD
`85e8b9cb` plus dirty source (`callable_normalization.rs` SHA-256
`18b8fca5daa404ea5a070b52f3b2a9a8a1d75ad9b209b12c043ab26b19c83111`).
Model-owned `CallableOwnerSelection` selects assessments by actual stored callable ownership,
decorators/evidence by those assessment IDs, and premises by stored evidence links. Canonical
row iteration keeps charged membership vectors sorted. It never filters by recomputed expected
IDs; all selected-owner contexts and extra/missing claims remain visible to independent equality.
Missing referenced premises refuse selection. Global orphan anti-joins remain unchanged and
prevent owner filtering from hiding unsupported roots or detached children/premises. Broad
source lookup and selected-family nominal closure remain intact. Borrowed views cannot outlive
the charged stored rows or membership selection; scratch membership releases after construction.
No material concern was found within this bounded correction. The reviewer ran no Cargo/tests
and changed no production source. Coordinator §9.1 owns actual runtime receipts and broader
acceptance; this source assessment does not qualify native artifacts or the whole plan.
