# Independent comparative audit: efficiency alignment

> Retired-source links pin the recorded committed baseline `88f33bc0`; working-tree differences, where reported, retain their original evidence limits.

**Decision: Revise.** The original review correctly identifies substantial avoidable work in
library-context's physical realization. Its central FP-07/A4 judgment survives independent
inspection. Several diagnoses need narrowing, and several proposed remedies need stronger
consumer, recovery and evidence contracts before they are suitable for implementation planning.
Six additional findings extend its coverage.

The architecture has useful semantic boundaries: exact contributions and views, explicit coverage
and refusal, model-owned identities, and separate publication, selection, pinning and recovery.
The execution problem is chiefly how these operations compose: small writes repeatedly cross the
same control boundary; selective demands become broad reads; warm reuse copies memberships;
immutable-domain preparation repeats; and some retained objects lack complete release routes.
These are structural findings, without a claim about measured latency, throughput or memory savings.

The corrective direction is owner-local work removal, followed by explicit decisions about reuse,
fencing, ranking and retained derivations. The original review sometimes makes those larger
redesigns prerequisites for improvements that can proceed independently. Conversely, removing
checks or moving observations is not safe merely because repeated work is visible. The audit
preserves the guarantees each correction must continue to establish.

## Scope, baseline and method

| Field | Audit scope |
|---|---|
| Reviewer and date | Fresh principal `design-reviewer`, with root integration/publication; 2026-10-11 |
| Tier and purpose | **Design / Target**; independent comparative assessment, not implementation acceptance |
| Subject | Product and development loop; docs publication tooling excluded as a review subject |
| Original | [Efficiency-alignment review](design_review_efficiency-alignment-codebase_2026-10-11.md), findings **O-F01–O-F50** and rule impacts **O-RC01–O-RC18** |
| Original source | `88f33bc0dd8bc67ef9291115c3a487b5f54bcb57` |
| Audit committed baseline B | `2b05c6c239fb6569472751faf126f5e39f822853`; the intervening commit contains the original review/evidence, with no product source delta |
| Concurrent source | Separately frozen working overlay at **2026-10-11 11:08:39 UTC**, with hashes and exact bytes; later context recorded separately |
| Standard | [Core 3.3](../design_principles/core/design-principles.md), [heuristics 1.0](../design_principles/core/efficient-architecture-heuristics.md), code-intelligence 1.5 and [repository binding](../design_principles/binding/library-context.md) |
| Functional target | Version-pinned API/evidence discovery and correct use, with precise support and uncertainty; [product owner](../../design/sections/api-and-evidence-product.md) |
| Evidence mode | Static source/contracts, pinned library evidence and attributed existing receipts; light service-free probes permitted, but none needed for the asserted structural conclusions |
| Execution | Product builds/tests, fixtures, SurrealDB queries, services, environment sync and real-library compiles **not_run**, as agreed; documentation checks establish publication only |

Source observations below are **Implemented, statically inspected on 2026-10-11**; library
interfaces are **Interface-checked** where identified. All corrective designs remain **Proposed**.
Historical probe results retain their original dates and limits; this audit did not remeasure them.
Source line numbers refer to B and can be recovered with `git show B:path`; live links are navigation,
not a claim that concurrent main remains byte-identical.

The [evidence index](../evidence/2026-10-11_efficiency-alignment-audit/README.md) records the
baseline, capture method, independence, complete comparison matrices and publication receipt.
The root read the original principal, coordinator handoff, method/reconciliation ledger, inquiry
briefs and probe limits. Handoff opinions were leads. Supporting inquiries covered native contracts,
compiler/reuse, and fresh breadth; the fresh-breadth reviewer formed candidates before reading the
original. A separate fresh principal reread decisive source and formed this combined judgment.

The matrices cover **every original finding and rule impact**:
[native contracts](../evidence/2026-10-11_efficiency-alignment-audit/native-contracts.md),
[compiler and reuse](../evidence/2026-10-11_efficiency-alignment-audit/compiler-and-reuse.md),
[serving and development](../evidence/2026-10-11_efficiency-alignment-audit/serving-and-development.md).
[Fresh coverage](../evidence/2026-10-11_efficiency-alignment-audit/fresh-coverage.md) records additional
items and inspected strengths. These are supporting evidence, not competing verdicts.

Existing ADRs, pins, plans and operating preferences describe the design being judged. They are
not additional judgment criteria. Proposed rule changes are listed below for subsequent plan
creation; no rule, implementation plan or original review is changed by this audit.

### Concurrent changes bearing on the findings

The frozen overlay strengthens current-context and negative/companion-witness eligibility for
derived serving rows and batches selected-root/state enumeration in charged 128-root windows.
Preserve these contracts in F02/F03/F08. They reduce some crossings without closing the full-state,
copying-attachment, repeated-lowering or global-effect findings. Their external acceptance remains
separate from this audit.

Two later small changes were inspected as publication context: an exact backup/restore control's
60-second × 15 timeout allowance, and six coarse manifest phase observations. The former changes
neither concurrency nor selection grain; the latter retains operation order, results and manifest
fields. Neither closes a finding. Because implementation fingerprints cover source bytes, the
logging change illustrates F12's invalidation distinction, without establishing a measured cost.
During publication, native recovery also gained first-owner dependency traversal and coarse capture
phase observations, with corresponding storage-owner prose. This does not change the finding that
cold recovery already follows logical closure at B; copying attachment remains a separate operation.
These later implementations receive no audit acceptance claim. Exact late bytes are retained in
`raw/publication-context.json` and its snapshots. Later STATUS/plan receipts are contextual updates,
not new audit qualification. Minor parallel edits do not reopen the baseline or change its course.

## Responsibilities and what must survive

The functional target requires meaningful answers about exact library versions, inputs and evidence.
A content-equivalent result, an exactly attributed result and an admitted physical realization are
related but different objects. Much of the original's proposed optimization is sound only when it
preserves that distinction.

| Responsibility | Owner and consequential contract |
|---|---|
| Semantic identities, relations, conditions, coverage and analytic meaning | `lctx-model::domain`; types and operations govern value, provenance, fidelity, availability and selected method |
| Captured configuration and provider observations | Acquisition and `cpg-extract`; exact analyzer family/context, source-role correspondence, joined work and explicit refusals |
| Dependency-bounded computation and admission | `cpg-core`; exact consumed inputs, preparation lifetime, product eligibility and effect ownership |
| Persistent realizations and retirement | `lctx-surrealdb`; physical incarnation, membership, ownership edges, fencing and recoverable effect disposition |
| Publication | `lctx-publisher`; readiness, seal, immutable manifest and retained reachability |
| Serving | Model request contracts plus `lctx-serving`; selected pin, eligibility, ranking, hydration, omissions and continuation |
| Python transport and embedding | MCP adapters; pending work, cancellation and native-worker lifetime are distinct from coroutine lifetime |
| Development evidence | Run/verification/storage owners; child outcome, capture health, executable observation and cleanup remain distinct facts |

The inspected implementation deserves credit for exact contribution specifications, full-value
collision checks, selected membership, absent/virtual/corrupt distinctions, explicit coverage and
partial outcomes, and declared finite-kernel methods. Publication, selection, reader pins and cold
recovery have separate responsibilities. Selected backup follows logical dependency closure;
restore independently admits reconstructed state. Repeated enforcement is not automatically
competing authority, and replaying one producer is not automatically an independent oracle.

## Change scenarios and architectural judgment

The workload premise is an evolving pinned library catalog, retained releases, repeated unchanged
or locally changed compiles, bounded selective queries, and concurrent development on one operator's
host. Size and skew vary: many small artifacts, sparse selections over retained history, high-degree
owners, and many independent callables are legitimate inputs. No representative corpus distribution
or deployed resource envelope was measured.

| Scenario | Credible responsibility and observed consequence |
|---|---|
| Many small artifacts | Byte windows should cross persistence in useful groups; chunk-local windows currently multiply control effects. F01 |
| A request supplies a small mandatory ID set | Selected verified keys should drive reads; the current filter route can broaden this into whole-view payload work. F02 |
| Unchanged provider inputs | Retain a qualified result with exact fresh attribution; the current pipeline reruns providers and copying attachment adds work to later reuse. F04 |
| One configuration sibling changes | Only genuine dependencies should invalidate; sibling/provenance identity can propagate unrelated changes. F05 |
| A new provider or optional method extends the domain | Preserve explicit vocabulary/availability and complete dependency keys; avoid creating every intermediate union and per-scope non-request solely because it has a semantic name. F05/F07 |
| A retained manifest is served after a lowering implementation changes | Current derivation identity/eligibility or an explicit generation cutover must govern substitution. Ordinary fresh compilation and retained-manifest reuse are different cases. F06 |
| A sparse pin shares a vector cohort with much retained history | Ranking and membership policy must remain explicit; scoring the whole cohort first can inspect more candidates than the current eligible-only route. F08 |
| Two MCP calls wait on embedding | Pending external work should not unnecessarily exclude unrelated native demand; running-thread ownership must still survive cancellation. F09 |
| Repeated compile and retirement | Named consumers need release routes for products and binding incidence, without deleting warm/live state. F18/F19 |
| One compiler edit or exact test rerun | Dependency and target selection should keep unrelated analyzer/build work out of the local loop. F12/F13 |

| Architectural judgment | Independent verdict |
|---|---|
| **A1 — Separation and local change** | **Violated narrowly.** Provider/composition dependency and fingerprint closure obstruct isolated development and propagate unrelated changes. F04–F07/F12 |
| **A2 — Domain model and authoritative realization** | **Unresolved at ranking and retained derivation policy; supported in inspected semantic contracts.** O-F38 does not establish a general competing membership authority. F06/F08/F20 identify the narrower decisions. |
| **A3 — Composition** | **Satisfied in inspected semantic interfaces.** Typed completed inputs, finite domain operations and effect adapters compose usefully. Physical amplification does not establish that all semantic composition is defective. |
| **A4 — Execution fit** | **Violated.** Confirmed per-item effects, broadened reads, copying reuse, repeated preparation and incomplete lifecycle integration burden supported workloads. Their existence does not depend on a measured speedup. |

### Relevant foundation and supporting-rule verdicts

These judgments apply to the inspected scenarios, not every implementation of each principle.

| Foundation | Verdict and scope |
|---|---|
| **FP-01** | **Violated narrowly:** independently changing provider/composition mechanisms remain coupled. F04–F07/F12 |
| **FP-02** | **Unresolved:** retained derivation substitution and exact/ANN ranking need complete consumer contracts; inspected contribution, coverage and effect contracts are otherwise supported. F06/F08 |
| **FP-03** | **Satisfied** in inspected semantic interfaces: no universal workflow engine is needed to compose the existing operations. |
| **FP-04** | **Unresolved narrowly:** ranking ownership and retained derivation identity need reconciliation; facts, coverage, nominal identity and evidence have executable semantic owners. |
| **FP-05** | **Violated locally:** tokenizer construction loses its selected-byte validation premise; retained product/binding lifetimes lack complete release transitions. F10/F18/F19 |
| **FP-06** | **Violated narrowly:** isolated compiler controls inherit unrelated heavy provider dependencies. Typed finite kernels and explicit outcomes remain strengths. F12 |
| **FP-07** | **Violated:** avoidable crossings, selection broadening, repeated preparation, copying reuse and retained incidence have independent examples in F01–F08 and F11–F19. |

Relevant supporting-rule calibration is similarly bounded:

- **Satisfied:** DP-01/02/04/05/07 for inspected authority, typed distinctions and identity/relationship
  contracts; DP-12 for explicit bounded iteration outcomes; DP-13/14 for examined library composition;
  DP-18 for inspected effect declarations.
- **Violated locally:** DP-03 at selected tokenizer construction; DP-10 at repeated preparation and
  scalar crossings; DP-15/22 at the conflicting implemented HNSW-route claim; DP-19 at incomplete
  retained-product/binding release integration. The lifecycle finding alone does not establish
  inconsistent committed output and therefore does not itself fail G5.
- **Unresolved:** DP-08/09/11 for retained derivation and ranking substitution; DP-20 for proposed
  combined admission/resource alternatives; DP-21 for corpus-origin attribution and evidence-time
  substitutions; DP-23 for proposed cold-check removal without complete witnesses; DP-24 for
  ranking/derivation contract cutovers. Proposed remedies are not counted as deployed violations.

For the code-intelligence profile, **CI-02/03/04/06/08 are satisfied** in sampled typed fidelity,
relationship, coverage and partial-outcome contracts; **CI-07 is satisfied** for examined routing
to established graph/compute capabilities; **CI-11 is satisfied** within inspected admission and
hydration boundaries. **CI-01/10 remain unresolved** at modified corpus attribution and complete
hermeticity coverage; **CI-05/09/13 remain unresolved** at ranking and retained derivation policy;
**CI-12 remains unresolved** beyond sampled evaluation separation. This does not certify unexamined
producers or protected evaluation populations.

### Correctness and fidelity gates

Gate judgments are independent of A4. A useful efficiency diagnosis does not justify every gate
assigned to it, and a proposed unsafe shortcut is not evidence that the current implementation
already executes that shortcut.

| Gate | Scoped judgment and decisive reason |
|---|---|
| **G1 Authority** | **Pass in inspected routes.** Quota/SUM representations, repeat checks and membership SQL do not by themselves prove independently mutable unreconciled facts. The suggested catalog-alias counterexample is not reachable through the inspected producer. |
| **G2 Semantic fidelity** | **Unresolved narrowly.** Ranking policy and revision-versus-modified-source attribution require decisions; no general collapse of unknown into absent was established. |
| **G3 Validity** | **Fail narrowly.** Verified tokenizer bytes can differ from the bytes subsequently loaded, violating the selected-assets premise. F10 |
| **G4 Hidden behaviour** | **Pass in inspected declarations.** Tokenizer loading is declared I/O and identity follows loaded bytes; this is not a claim of complete analyzer hermeticity. |
| **G5 Consistency and recovery** | **Pass in inspected publication/recovery routes.** Effect disposition, fences, pins and independent restore are real strengths. Missing retirement integration is not demonstrated unsafe publication. |
| **G6 Transformation and reuse** | **Unresolved for retained-manifest derivation.** Current derivations over old provenance need a complete substitution contract. Ordinary compilation already fingerprints native source. F06 |
| **G7 Truthful capability claims** | **Fail narrowly.** Implemented-route prose claims HNSW while served ranking uses the exact route. Proposed advantages and pending qualification are not themselves false claims. F08 |
| **G8 Library leverage** | **Pass for examined choices.** Existing libraries are used meaningfully; the sorter has a real conflict-validation/bounded-state contract. Its replacement remains a comparison, not an established unjustified bespoke implementation. |
| **CI-G1 Fidelity** | **Unresolved** at corpus-origin fidelity and beyond sampled attribution contracts. |
| **CI-G2 Evidence closure** | **Pass within inspected admission/hydration boundaries.** Frozen eligibility changes and their external acceptance remain separately attributed. |
| **CI-G3 Evaluation integrity** | **Unresolved beyond sampled separation.** Public-byte/private-reference routes were inspected; protected evaluation populations were not opened. |

## Findings and corrective contracts

Audit IDs **F01–F20** are local to this document; **O-Fnn** always means the original review.
The supporting matrices preserve each original obligation, including confirmed observations that
do not need their own new principal finding. Consequence-based priorities below are not a work
schedule. All remedies are **Proposed** and **unscheduled**: the trigger for disposition is the
user's reconciliation during implementation-plan creation. No existing plan is silently updated.

### Finding groups do not merge closure obligations

The twenty principal IDs group the comparative argument. Plan reconciliation must retain original
source-review IDs and their separate closure evidence. In particular:

- **O-F05:** unused intent payload and redundant hashing can close independently of intent/outcome protocol redesign.
- **O-F13:** terminal-result recovery/reuse and abandoned-root reclamation remain separate; declining reuse does not close reclamation.
- **O-F20:** lookup/retention crossings, quota/accounting, and payload encoding/canonicalization have distinct corrections.
- **O-F35:** fixed executable identity, pin-owned preparation, membership memoization and page-local hydration have different lifetimes and keys.
- **O-F48:** service-generation identity reuse and test/evidence receipt policy are different consumers.
- **Audit F09:** MCP demand admission and CLI/test signal integration have independent closure obligations.

Closing one representative instance does not close its entire group.

<a id="F01"></a>
### F01 — Native effects use unnecessarily small physical units and shared coordination

**High · A4 · FP-07 · DP-10/19/20 · O-F01–O-F07, O-F12/O-F14.**
The original is right about the common installation write (`control/authorization.rs::ERA_GUARD`,
B:89), separately ensured anchors/payloads/holds, chunk-local original windows and repeated ownership
transfer. `control.rs::run_transaction` retries immediately and indefinitely; its future can still
be externally cancelled. An ambiguous effect needs its original recoverable disposition.

The native owner should first fuse useful bounded work: remove unused intent bindings and duplicate
internal hashing, combine row/payload/hold effects where atomic reachability permits, and window
original chunks across a bounded iterator. Preserve collision/value checks, request identity,
incarnations and ownership. Do not construct an all-corpus vector. Larger transactions can increase
retry cost, so fewer crossings alone do not establish a throughput benefit.

Read-only ordinary fencing is a separate protocol decision. Its predicate inventory must include
retirement, incoming holds, backup/drain, era cuts, history pagination, outcome references and
successor-era lineage; exact-object locks already present deserve credit. Separate current-era
maintenance authority from admission-open. Revision allocation and reconciliation remain owned.
Retirement already charges and resumes high-degree roots; it does not require one effect for all
outgoing edges. Bounded sequential fusion and provider-local overlap need not await a new fence protocol.

**Closure:** trace each fused effect's atomic reachability and ambiguous-outcome recovery; establish
that any new fencing scheme protects the named predicates. Remove crossings without weakening
those contracts. Engine conflict frequency and performance remain unmeasured.

<a id="F02"></a>
### F02 — Selective compiler requests broaden into membership and payload work

**High · A4 · FP-07 · DP-10/14 · O-F21/O-F22.**
[SupportedFilter::driver](https://github.com/paul-heyse/library-context/blob/88f33bc0dd8bc67ef9291115c3a487b5f54bcb57/crates/lctx-surrealdb/src/compiler_provider.rs) (B:848–864)
recognizes atomic fields, while real consumers issue mandatory `id IN` requests. The resulting
path can read the complete membership run and hydrate payload addresses. Multi-owner explicit-key
lookup additionally probes keys against retained history before filtering owners.

The provider/native boundary should carry mandatory IDs and verified key projections. Preserve
AND/OR semantics, empty-view validation, absent/virtual roots, nominal conflicts and the distinction
between key demand and payload demand. Verified membership runs can serve selection without first
redesigning every sparse lookup. Bounded owner×key lookup is an alternative for few owners; an
unbounded Cartesian expansion is not.

**Closure:** selective callers use a verified selective path and retain exact selected-view/error
semantics. Source and focused countercases can establish work-shape improvement; actual native
plans and owner-count distributions remain unresolved.

<a id="F03"></a>
### F03 — Admission and seal repeat reconstruction, but cold assurance still has a job

**High · A4, constrained by G3/G5/G6 · DP-03/09/23 · O-F08–O-F10/O-F15.**
Canonical batches are re-decoded/canonicalized; `state_identity_selected` (compiler B:4168–4207)
reconstructs, sorts and hashes completed state; seal repeats state and derived reconciliation.
The original's per-row backing-query diagnosis is incorrect: `next_batch(1)` is local iteration;
`admit_backing_row` buffers rows/bytes for `validate_backing_batch` (B:6256ff).

Carry a nominal checked canonical batch where its premises hold, compute unchanged state once,
and remove exact duplicate lowering/checks. Keep validation of actual stored rows, references and
extras. Repeating the same lowering is not an independent semantic oracle, but checking persisted
results against an independently admitted source remains useful.

Compositional identities or durable validity premises need coverage of every identity-bearing row,
validator/producer implementation, physical incarnation and cold ingress. Binding revision alone
is insufficient. Once-per-attempt batched hashing is a smaller alternative. Cold import/restore
and audit are separate uncertainty boundaries, not automatically redundant work.

**Closure:** identify what failure each retained check detects and what protected premise permits
an omitted check. Changing completed-state identity or durable witness semantics is an explicit
format/contract decision; a local memo remains legitimate if that decision is declined.

<a id="F04"></a>
### F04 — Provider reuse needs complete capture keys and an inexpensive retained attachment

**High · A1/A4 · FP-01/02/07 · DP-09 · O-F13/O-F16/O-F17.**
`compile_facts` always executes offered providers. Later hits copy memberships into attempt-owned
contributions and reinstall views. `product_request` omits the captured binding present in
`ContributionSpec` identity. Captured provider configuration already exists; reuse must actually
key it, not assume an uncaptured external effect.

The compiler should declare eligible captured-input provider results and complete context/spec
keys. Reusable completed outcomes are not a promise that a new resource-constrained run produces
identical partial results. Native attachment should validate the receiver before effects and hold
the exact retained physical incarnation. Existing cold recovery already follows logical closure;
the original attempt-only objection is overstated. Existing-view installation needs a durable
completion fact, not a row-count inference. Copying attachment can still save analyzer work, so
by-reference attachment is an efficiency enabler rather than an absolute prerequisite to provider reuse.

Terminal completion and valid reusable completion remain different states. Under revised RC03,
a terminal **unadmitted source attempt** may supply a candidate after a new admission-for-reuse
operation validates outcome, specification, provenance, physical content and required closure under
recovery ownership. This does not require that source attempt to have been admitted previously.
Poisoned or corrupt candidates must remain distinguishable from ordinary misses.

**Closure:** eligible unchanged inputs avoid provider execution; exact-spec warm attachment avoids
N-sized membership copies while preserving fresh attribution, lifetime and cold recovery. Reclamation
of abandoned roots remains separately necessary even if recovery reuse is declined.

<a id="F05"></a>
### F05 — Identity, availability and cache policies create distinct forms of excess work

**High for warm/edit loops · A1/A4 · FP-02/07 · DP-08/09 · O-F18–O-F20/O-F23/O-F24.**
Exact contributor/sibling identity propagates changes; every product includes profile; providers
form intermediate unions; uniformly unrequested methods materialize per-scope work. Portable cache
hits also cross lookup/retention boundaries and repeat encoding. These are related but not one
interchangeable defect.

Split configuration dependencies first where their consumers are already separate. Broader value
cutoff must distinguish relation values from exact provenance and attachment. Invocation receipts
embed source digests, so a generic refreshed-spec attachment cannot relabel receipt-bearing rows.
Profile neutrality must hold across actual dependencies and availability, not just branch-free code.

Demand-bound union installation must preserve the first explicit empty completed relation and any
real intermediate consumer. Compact domain-total `NotRequested` must update frontier identity,
assessment and scope-query consumers together; missing, partial and no-scope states remain distinct.
Batch cache crossings and remove repeated encoding before assuming an opt-in policy is universally
better. Quota `SUM` and admission accounting have different roles; their coexistence does not prove G1.

**Closure:** demonstrate output-specific substitutability and receipt fidelity; preserve explicit
empty/non-request semantics; close cache crossings, accounting and encoding independently. Cold/edit
savings and expensive warm-kernel reuse are competing policy considerations, not measured outcomes.

<a id="F06"></a>
### F06 — Retained-manifest derivation needs a current implementation contract

**High decision priority · A2/G6 unresolved · DP-09/19, CI-13 · O-F11.**
The original overstates the identity omission. Ordinary compilation's producer fingerprint includes
native source through the `cpg-core` dependency closure and propagates through producer inventory
into manifest content. Changed ordinary compilation therefore already changes relevant identity.

A retained/imported/restored manifest intentionally carries old provenance. Applying a changed
current publisher/lowering implementation to it needs a current derivation key and eligibility
contract, or an explicit generation cutover. A new publication handle alone does not separate old
and new derived rows sharing canonical dependencies. Coordinate this decision with any narrowing
of the broad fingerprint in F12.

An unchanged-publication fast path should separately verify current definitions, exact manifest
and retained reachability. Avoiding original-byte reads in publication does not establish a whole
unchanged compile that reads no originals.

**Closure:** retained old manifests under changed lowering have unambiguous derived identity and
eligibility; existing pins are preserved or explicitly quiesced/retired. Establish this before
claiming safe shortcut or general derivation substitutability.

<a id="F07"></a>
### F07 — Stable preparation is repeated across owners and inner loops

**Medium–High by active consumer · A4 · FP-07 · DP-10/13/20 · O-F25–O-F29/O-F31–O-F34.**
The original correctly finds repeated expected-domain indexes, provider path/span searches,
artifact-use lookup, BDD lowering, Structural traversal preparation and embedding preparation.
Locked Pyrefly supports the shared joined pool bypassed by dependency waves. Locked Ruff supports
observation-aware checking. These capabilities provide concrete routes, not speculative libraries.

Use invocation/input-owned preparation with charged lifetimes. Fuse Ruff traversal only with exact
fork parity, including diagnostic multisets and noqa; preserve multiple artifact roles per path.
Fuse RECORD SHA-256 into capture, but do not infer immutability from private/read-only placement.
BDD preparation must preserve effective support after reduction, including cancelling expressions.
Structural work must bound high-degree adjacency preparation before the arc cutoff; sharing maps
alone does not fix collecting/sorting all outgoing edges per seed.

Embedding preparation already deduplicates texts and flushes by batch/budget: requests are not one
per root. Batch distinct ready work without retaining rich payloads merely to fill a batch. Optional
analytics shortlists, owner caps and bipartite alternatives may change exact best-window top-k or
community method; they need explicit policy/identity, not a transparent-optimization claim.

**Closure:** preserve canonical facts, ambiguity, refusal/partial states, effective support,
deterministic bounds and declared method. Provider-local preparation/pool improvements do not
require native protocol redesign. Default-off/behavioral-only work has lower immediate priority.

<a id="F08"></a>
### F08 — Serving needs distinct preparation lifetimes and one explicit ranking policy

**High · A2/A4, G7 fail narrowly · FP-04/07 · DP-09/10, CI-05 · O-F35–O-F38.**
Fixed executable identity, pin-fixed library/classification preparation, membership checks and
page-local hydration currently repeat at different grains. Vector serving hydrates an eligible
frontier and uses exact scoring over eligible documents, followed by Rust-side tier/witness ranking,
while implemented-route prose describes HNSW. The original is
right to demand reconciliation.

Choose exact or approximate ranking as a consumer contract, then place preparation at its actual
validity lifetime. Preserve selected pin/epoch, absence and current-context premises, relation-qualified
dependencies, omissions and continuation. Eligibility must precede result admission and tier/cap
counting; pure scoring may move only if ordering, failures and isolation remain equivalent.

Cohort-first scoring is not unconditionally cheaper: many retained releases can make the shared
cohort much larger than a sparse eligible pin. Route choice must acknowledge that losing case.
The HNSW reference has a real numerical/shared-representation control role, including full-value
projection checks; it does not exercise the served candidate path. Preserve an independent oracle
when changing the route, rather than replacing it with the same production query.

O-F38's suggested catalog-alias counterexample is unsubstantiated through the inspected producer:
canonical alias mapping applies to `Place`, outside package/release/input-distribution/catalog-member
discovery. Final hydration already checks exact membership and canonical/nominal identity. The
physical query-plan question remains unresolved; this is not a general membership-authority failure.

**Closure:** one discoverable ranking contract governs the actual route; independent references
cover its meaning. Preparation keys retain complete selected-context premises. Compare sparse and
dense eligibility when choosing physical order without promising a measured advantage.

<a id="F09"></a>
### F09 — Pending MCP demand and signal integration need separate corrections

**Medium–High · A4 · FP-03/07 · DP-19/20 · O-F39/O-F42.**
Two Python end-to-end admission slots can both be occupied while embedding waits, excluding unrelated
native demand. These slots are not a duplicate native CPU-thread budget. The inspected native bridge
correctly retains ownership until worker completion and drains before session close.

**MCP admission** should separate demand classes or preparation/execution capacity while retaining
finite pending work and ownership through actual worker completion. **Signal integration** should
connect CLI entrypoints to cooperative cancellation, drain and permitted abandonment. Tests/fixtures
bypassing that CLI need their own integration; neither change substitutes for the other.

**Closure:** unrelated MCP demand progresses during blocked embedding without losing worker/drain
ownership. CLI and test orchestration separately establish graceful cancellation and truthful
forced/timed-out outcomes. No universal “SIGTERM leaves no open holds” guarantee follows.

<a id="F10"></a>
### F10 — Tokenizer construction must consume the bytes whose selection was verified

**Medium, correctness · G3 fail · DP-03/21, CI-10 · O-F44.**
`LocalTokenizer::load_verified` verifies selected SHA-256 values and then calls a loader that reads
the paths again. The loaded bytes can differ. Contrary to the original diagnosis, identity does
follow the actual second-read bytes: this is a selected-assets validity gap, not unchanged identity
or hidden pure behavior under G4.

Construct from the verified buffers, preserving preprocessing, special-token and template contracts.
The locked tokenizers 0.23.2 API supports `from_bytes`; newer serialization documentation was not
transferred to this pin. The project already uses the byte constructor on its loading path.

**Closure:** the exact bytes satisfying selected hashes are the bytes supplied to tokenizer/config
construction. A focused source trace or revealing replacement-between-reads control settles this
without a real model/service run.

<a id="F11"></a>
### F11 — Sorting can remove small-input I/O without sacrificing bounded state

**Medium · A4 · FP-07 · DP-13/20 · O-F40/O-F41.**
`OrderedRows` eagerly creates a temp directory and `finish` flushes even a small final set to disk.
The original understates the small-input opportunity. Its pairwise merge also deliberately bounds
live heads and transient decoding; an all-runs merge could lose that guarantee.

Keep a lazy in-memory tail for fitting inputs; select a bounded merge fan-in from actual reservation,
row and descriptor constraints. Compare DataFusion sorting plus streaming adjacent-key conflict
validation against the bespoke route at the real adapter boundary. Neither library adoption nor
unbounded k-way merging is automatically the best complete design.

The 64-GiB envelope is not RSS/allocation enforcement or an aggregate host guarantee. Per-process
capacity discovery still duplicates capacity across processes. This does not establish the original
host-pressure causality; any shared admission decision needs its own causal evidence.

**Closure:** small fitting inputs avoid disk while large inputs preserve ordering, exact conflict
behavior and bounded live state. No job/thread limits or capacity-policy change is authorized here.

<a id="F12"></a>
### F12 — Dependency and profile boundaries amplify the development loop

**High–Medium · A1/A4 · FP-01/06/07 · O-F43/O-F45–O-F47.**
Compiler controls inherit heavy provider dependencies; `producer_fingerprint.rs` walks declared
local dependencies and complete source roots. A feature gate alone does not narrow that walker.
Source/package counts are not measured link cost or binary size. Per-root INFO volume is real,
but no runtime dominance was established.

Separate contract, adapter and composition owners where a real consumer benefits. A crate split
or sound fingerprint-root change can narrow identity; coordinate with F06 so derivation code does
not disappear from ordinary identity coverage. Grouping fixture controls can remove repeated
compiles, but changes nextest selection/reporting grain; one observation does not establish that
all assertions have become shared.

`cargo check` does not perform ordinary target machine-code generation/linking. Check plus release
tests are not two equivalent optimized builds. Existing BC3 already owns candidate local-profile
qualification; reconcile with it rather than creating another track. Preserve debug/overflow and
release acceptance distinctions. A shared ordinary profile does not authorize deleting warm trees.

**Closure:** a local control avoids unrelated analyzer/build work while preserving exact target,
assertion and identity coverage. Profile adoption follows BC3's actual qualification boundary;
per-root logging can be corrected independently.

<a id="F13"></a>
### F13 — Tooling should reuse owned facts without changing their evidence meaning

**Medium · A4 and evidence validity · FP-06/07 · DP-21 · O-F48–O-F50.**
The original correctly identifies repeated service/test identity work, broad exact-case builds and
foreign-registry failure coupling. Its cache-grain claim is too narrow: `CaptureOwner` can span
invocations within a run. Main run receipt/log completion also precedes the global sidecar scan;
not every form of completion is lost if an unrelated descriptor is corrupt.

Reuse service validation only within the protected borrow/service-generation lifetime and before
administration. A daemon/path/stat cache must bind the validated file and generation and invalidate
on replacement. This is separate from test receipt policy. A digest of a mutable path at later
export cannot recover bytes observed at completion. Choose retained completion evidence, retained
immutable artifacts, or explicitly missing digests; label export-time observations honestly.

Exact reruns should project recorded package and Cargo target kind/name, preserving profile,
features, filters, environment and prerequisites. `package::name` is not universally an integration
test. Refuse unsafe narrowing when target metadata is absent/ambiguous. Known-owner completion
should use exact IDs/indexed paths; global retirement still needs to expose unrelated corruption.

**Closure:** historical evidence claims remain time-correct, exact-case selection narrows the proper
build target, and foreign descriptors cannot block unrelated known-owner sidecar completion.

<a id="F14"></a>
### F14 — Summary discovery repeatedly scans unrelated calls and support

**Additional · Medium, behavioral profile · A4 · FP-07 · DP-10.**
[summary_production.rs](../../../crates/lctx-model/src/domain/execution/summary_production.rs)
(B:1185–1234, 1310–1334, 1942–1968) nests support/Entry searches, scans calls per proof and scans
seeds/branches per SCC. Many independent singleton SCCs still pay for unrelated components.
O-F31's condition lowering and O-F32's Structural traversal do not cover this owner.

Prepare owner/call/support indexes once per immutable invocation and follow matching relations.
Preserve deterministic SCC order, semantic progress, Pareto proof alternatives and explicit residuals.

**Closure:** sparse independent-callable and alias cases retain canonical results while unrelated
components cease multiplying discovery work. No new global summary service is needed.

<a id="F15"></a>
### F15 — Automatic synthesis repeats joins for every public candidate

**Additional · Medium · A4 · FP-07 · DP-10.**
[automatic.rs](../../../crates/lctx-model/src/domain/synthesis/automatic.rs) (B:199–267) scans member
frames, analytic inventories, ranks and community memberships per candidate. Prepare these indexes
once per admitted-input lifetime, preserving duplicate-slot ambiguity, configured selections,
absent analytics, ties and community/budget outcomes.

Do not stop once the brief budget is full: complete decision records have consumers (B:340–355).
This is separate from optional analytics and from the already compact documentary synthesis path.

**Closure:** complete decisions and selected output remain equivalent while candidate joins use
selective lookup instead of repeated relation scans.

<a id="F16"></a>
### F16 — Ruff correspondence scans canonical events for each binding

**Additional · Low–Medium · A4 · FP-07 · DP-10.**
[ruff_lexical.rs](../../../crates/cpg-extract/src/ruff_lexical.rs) (B:201, 228–249, 281) repeatedly
searches canonical events/scopes and allocates hit vectors although site/name are available keys.
This is distinct from O-F26's parser/checker traversal and O-F28's artifact-use lookup.

Build source-event buckets by site with exact-name disambiguation and canonical scope/observation
sets. Preserve unique-site fallback, ambiguous-name and unlocated outcomes.

**Closure:** multi-name and unsupported-location correspondence remains identical while each binding
no longer scans the entire event population.

<a id="F17"></a>
### F17 — Profile status repeatedly decodes unchanged completed samples

**Additional · Low–Medium, opt-in tooling · A4 · FP-07 · DP-10.**
[compile_profile.py](../../../scripts/compile_profile.py) (B:1171–1202, 1222–1253) calls
`sampled_report(readonly=True)` per unit. On a cache miss it hashes input/tool and executes
`perf report`, then returns before writing the reusable report receipt.

Ordinary status should use bounded metadata, request decoding explicitly, or consume a result
retained under the existing report owner. Preserve capture completion, exact reader/input identity
and unknown labels.

**Closure:** unchanged repeated status does not repeat decoding; changed input or reader invalidates
reuse. The diagnosis establishes duplicate work, not its measured duration.

<a id="F18"></a>
### F18 — Retained native products lack a complete owner release route

**Additional · Medium · A4 · DP-19/20; adjacent to O-F13/O-F17.**
`compiler.rs::retain_product_identity` (B:1496–1526) creates per-request/per-attempt product roots
with contribution holds. Admitted close releases attempt holds; ordinary publication retirement
nominates publication roots. No owning route to retire obsolete product roots was found through
inspected creation, close, retirement/history and scoped deletion searches. Generic retirement
exists: the gap is integration, not absence of a primitive or a measured leak.

Give retained products named consumers/lifetimes and an explicit release/nominating route. Preserve
warm reuse, exact views, publications, backups and all live holds. Nominate abandoned roots before
releasing the last owning hold. By-reference attachment does not retire distinct obsolete specs.

**Closure:** obsolete products can reach retirement through their owner while retained/live content
remains protected. Neither age/size nor this finding authorizes deletion.

<a id="F19"></a>
### F19 — Attempt binding incidence obstructs terminal history compaction

**Additional · Medium · A4 · DP-19.**
`bind_inner` (compiler B:4072–4079) creates attempt-specific binding rows. Inspected close/cleanup
does not remove them; `history::eligible` requires no bindings and its phases have no binding
phase. A bound terminal attempt cannot meet that condition through the examined lifecycle.

Blind deletion loses a consumer: [selected_backup.rs](../../../crates/lctx-publisher/src/selected_backup.rs)
(B:180–188) selects a stored binding by logical view. Establish a canonical descriptor or an
authoritative retained representative for cold consumers, then release redundant attempt incidence
and compact eligible attempts with crash-safe ownership.

**Closure:** backup/recovery works after redundant binding release, and terminal attempts can reach
the existing eligibility condition. No retained-history volume was measured.

<a id="F20"></a>
### F20 — Corpus origin must distinguish a named revision from modified captured bytes

**Medium, fidelity · G2/CI-G1 unresolved · DP-21, CI-01 · O-F30.**
[storage_acquisition.py](../../../scripts/storage_acquisition.py) `check_source` (B:309–318) checks
HEAD while origin reports the revision. Captured-byte identity remains exact. The unresolved
contract is whether origin means exact tree content or a base revision plus modifications.

Compare selected captured files with the named tree or explicitly label modified attribution.
A blanket clean-tree requirement rejects legitimate unrelated edits and remains race-sensitive.

**Closure:** modified selected inputs cannot be mistaken for the exact named tree, while unrelated
dirty work does not invalidate legitimate capture. This is an attribution correction, not an
efficiency cause.

## Material corrections to the original statements

The original is broadly right about the work shapes. These corrections change its explanation,
remedy or gate strength; they do not erase the confirmed structural observations.

| Original claim or proposed route | Independent correction |
|---|---|
| O-F09's per-row backing validation | Local `next_batch(1)` is not a native read per row; backing projection is already buffered/batched. Full-state reconstruction and repeat seal remain. |
| O-F11's general derivation-identity omission | Ordinary compilation fingerprints native implementation through producer inventory. Retained-manifest/current-derivation substitution is the narrower gap. |
| O-F17's attempt-only cold closure objection | Recovery already follows logical dependency closure; exact physical incarnation and installed-view completion remain obligations. |
| O-F12's high-degree retirement concern | Existing retirement charges edge units and resumes bounded pages; all fanout need not fit one effect. |
| O-F38's possible catalog-alias counterexample | The inspected producer's alias mapping is Place-specific and does not reach those catalog predicates; final hydration checks membership and identity. Query plans remain unexamined. |
| O-F44's unchanged identity/G4 reasoning | Identity follows loaded bytes. The defect is loss of the selected SHA-256 validation premise: G3. |
| O-F20's duplicated accounting as authority failure | Planning SUM and fenced quota admission have distinct roles. Duplication alone does not establish G1. |
| O-F36's cohort-first efficiency rationale | Sparse eligibility over a large shared cohort is a legitimate losing case. Preserve error and tier semantics as well as final order. |
| O-F33's shortlist/bipartite/cap alternatives | Exact rescoring does not prove omitted best-window candidates are harmless; owner caps and bipartite methods may alter communities. Require explicit method policy where semantics change. |
| O-F39's capacity framing | Python slots bound end-to-end pending demand, not native CPU threads. The liveness diagnosis remains. |
| O-F40's small-input qualification and merge remedy | Finish spills even the small tail; all-runs merge may lose existing bounded heads/descriptors. |
| O-F47's duplicate optimized build rationale | `cargo check` is not ordinary codegen/linking. Profile reuse is real but already belongs to BC3 qualification. |
| O-F48's cache lifetime and delayed hashing | Cache ownership can span invocations; hashing a mutable path later cannot replace a completion-time observation. |
| O-F49's binary-ID projection | Recorded target kind is needed; a bare `package::name` does not always select `--test`. |
| O-F50's completion consequence | Main receipt/log completion precedes the foreign-registry scan; known sidecar completion is the narrower affected operation. |
| O-F20's RC19 reference | The original table has RC18; retain that ID. |

Other important qualifications are retained in the complete matrices: expected-domain admission
already uses a compact projection; Pyrefly reparse evidence is narrower than every class-containing
module; analytic refusal is frame-local; embedding dedup/batching already exists; and private file
placement alone does not make later capture checks redundant. The known operation-map V02 error
is not propagated: TyFlow is behavioral-only, outside catalog extraction.

## Rule impacts for plan reconciliation

Audit **RC01–RC18** correspond to original **O-RC01–O-RC18**, with independent corrections below.
Each original-ID link supplies its exact rule quotation and recorded location. These are decision
proposals, not approved changes. The six additional findings require owner implementations, not
an additional standing rule or review register. Current disposition remains with subsequent plan
creation; existing scheduled source-review obligations retain their current owners.

| Audit ID · current rule/location | Proposed change and dependent audit findings | If the rule is kept |
|---|---|---|
| <a id="RC01"></a>**RC01** · [installation coordination, ADR-0145/SA8](design_review_efficiency-alignment-codebase_2026-10-11.md#rc01) | Conditionally support narrower ordinary fencing after the complete predicate/recovery inventory, with maintenance authority and revision allocation explicit. F01 | Shared conflicts remain; bounded fusion, local overlap and recoverable retry improvements are still possible. Do not place all of them behind redesign. |
| <a id="RC02"></a>**RC02** · [pure-stage attachment restriction](design_review_efficiency-alignment-codebase_2026-10-11.md#rc02) | Permit eligible captured-input Acquisition/Extraction reuse with full capture/context keys and outcome policy. F04 | Facts still re-execute; other preparation/persistence reductions remain useful. |
| <a id="RC03"></a>**RC03** · [admitted-source recovery rule](design_review_efficiency-alignment-codebase_2026-10-11.md#rc03) | Permit terminal unadmitted candidates only through admission-for-reuse under recovery ownership. F04 | Reuse may remain excluded; abandoned-root reclamation remains necessary. |
| <a id="RC04"></a>**RC04** · [exact-view/value identity](design_review_efficiency-alignment-codebase_2026-10-11.md#rc04) | Separate value substitution from provenance/evidence identity; receipt-bearing rows need exact attribution or regeneration. F05 | Split configuration outputs where already independent; broad value cutoff remains unavailable. |
| <a id="RC05"></a>**RC05** · [profile identity at every contribution](design_review_efficiency-alignment-codebase_2026-10-11.md#rc05) | Keep attempt/publication profile explicit; remove contribution profile only after proving neutrality of actual dependencies and availability. F05 | Current profile-keyed products cannot share hits merely because an algorithm is branch-free. |
| <a id="RC06"></a>**RC06** · [independent reconstruction/cold admission](design_review_efficiency-alignment-codebase_2026-10-11.md#rc06) | Remove duplicate derivation first; carry protected validity only with complete native witnesses. F03 | Exact duplicates can still go; retain meaningful stored-row and cold-ingress reconciliation. |
| <a id="RC07"></a>**RC07** · [completed-state format v3](design_review_efficiency-alignment-codebase_2026-10-11.md#rc07) | Consider versioned composition of validated completion digests. F03 | Compute ordinary identity once with better local batching; omitting backing validation still needs a separate sound premise. |
| <a id="RC08"></a>**RC08** · [per-scope non-request coverage/frontiers](design_review_efficiency-alignment-codebase_2026-10-11.md#rc08) | Compact invocation/domain-total non-request with frontier, assessment and query migration. F05 | Physical stage fusion remains possible, preserving existing scoped rows and distinctions. |
| <a id="RC09"></a>**RC09** · [publication handle v2](design_review_efficiency-alignment-codebase_2026-10-11.md#rc09) | Define current derivation identity/eligibility or explicit generation cutover for retained manifests; handle-only addition is insufficient. F06 | Ordinary changed fresh compilation is already fingerprinted. Retained-manifest shortcut/substitution still needs a decision; do not claim all republishing is impossible. |
| <a id="RC10"></a>**RC10** · [approximate HNSW route statements](design_review_efficiency-alignment-codebase_2026-10-11.md#rc10) | Choose one ranking contract; exact is a viable near-term option, with numerical reference consumers explicitly retained or retired. F08 | Qualify an architecture satisfying the retained approximate and frozen-result requirements; this audit does not prove only one possible ANN design. |
| <a id="RC11"></a>**RC11** · [eligibility before ranking](design_review_efficiency-alignment-codebase_2026-10-11.md#rc11) | Distinguish result/cap admission from pure score evaluation order, preserving failure/isolation/tier semantics. F08 | Eligible-only scoring remains; CPU/preparation improvements proceed independently. Neither choice guarantees lower total work. |
| <a id="RC12"></a>**RC12** · [ordinary workers/resource defaults](design_review_efficiency-alignment-codebase_2026-10-11.md#rc12) | Defer policy choice pending causal evidence; aggregate admission needs an actual owner. F11/F12 | Per-process envelopes remain no host-wide memory guarantee; remove repeated work independently. No caps change here. |
| <a id="RC13"></a>**RC13** · [no per-attempt database](design_review_efficiency-alignment-codebase_2026-10-11.md#rc13) | **Not recommended.** Physical isolation trades away sharing/reuse/lifecycle benefits; contention alone does not establish necessity. F01 | Continue owner/attempt isolation inside the shared store. |
| <a id="RC14"></a>**RC14** · [build/test profiles and compile checks](design_review_efficiency-alignment-codebase_2026-10-11.md#rc14) | Reconcile with existing BC3 candidate-profile qualification, correcting the check/codegen explanation. F12 | Preserve current release-before-BC3 boundary; do not describe checks as another equivalent optimized build. |
| <a id="RC15"></a>**RC15** · [executable digest evidence](design_review_efficiency-alignment-codebase_2026-10-11.md#rc15) | Choose completion observations, retained immutable artifacts or explicitly missing digests; protect any exact-generation cache. F13 | Keep truthful completion-time evidence; delayed mutable-path hashing is not an equivalent substitute. |
| <a id="RC16"></a>**RC16** · [revision-scoped original storage](design_review_efficiency-alignment-codebase_2026-10-11.md#rc16) | Content-keyed blobs with artifact layout references, shared holds and coherent backup/recovery/retirement. F01 | Duplicate physical bytes remain; cross-chunk windowing still reduces crossings. |
| <a id="RC17"></a>**RC17** · [mandatory independent cold admission](design_review_efficiency-alignment-codebase_2026-10-11.md#rc17) | Optional durable premise keyed by logical inputs, validator/producer implementation and protected physical incarnation. F03 | Local memo remains the only carry; the observation is true even if durable reuse is declined. Do not withdraw the diagnosis. |
| <a id="RC18"></a>**RC18** · [always-enabled portable cache](design_review_efficiency-alignment-codebase_2026-10-11.md#rc18) | Conditional opt-in policy after comparing cold/edit cost and expensive warm reuse; repair crossings/encoding independently. F05 | No policy change is needed once useful physical repairs land; evidence must not presume every warm cache is harmful. |

## Heuristics, priorities and prerequisite order

H2/H8 explain native grain and selective reads: preserve semantic distinctions while physical
windows express useful sets of work. H5/H7/H9 require semantically sufficient inputs and access
paths; they also expose cohort-first search's losing case when later filtering increases work or
changes error exposure.

H12–H14 support invocation-owned preparation and selective reuse, including lookup and retention
costs. They do not permit provenance relabelling. H18–H20 distinguish duplicate reconstruction from
meaningful cold admission: carry validity only while its actual premises remain protected and
retain independent protection where uncertainty enters.

H21–H25 govern pending-work admission, fencing, retries and recovery ownership. They support
narrowing unnecessary ordering while preserving ambiguous-effect disposition and running-worker
drain. H26–H28 apply to diagnostics, receipts, builds and retirement: auxiliary machinery needs a
consumer, full lifecycle alternatives matter, and useful local work removal need not await an
enclosing redesign. These are qualitative applications, not an additional checklist or cost model.

Priority and prerequisite order are different:

1. **Resolve narrow correctness and contract problems early:** selected tokenizer bytes (F10),
   ranking claims/policy (F08), retained-manifest derivation (F06), and origin meaning (F20).
   Their urgency does not make them prerequisites to every efficiency correction.
2. **Proceed with independent work removal:** bounded effect fusion/intent cleanup, mandatory-ID
   reads, owner-local preparation, a no-spill small sorter path, and known-owner receipt lookup.
   None categorically requires replacing the global fence first.
3. **Make reuse sound and cheap:** complete capture keys and reusable-outcome eligibility precede
   provider reuse; exact held incarnations and durable view completion precede by-reference attachment.
   Copying attachment may serve an intermediate useful provider-reuse route.
4. **Make broader model/lifecycle decisions explicitly:** provenance/value cutoff, compact non-request,
   durable premises, content blobs, retained product release and binding compaction affect different
   consumers. Do not combine them solely because each reduces repeated work.
5. **Increase overlap only with owned pending work and recovery:** provider-local pool use and useful
   extraction/I/O overlap have narrower prerequisites than a complete fence redesign. Preserve drain,
   retry disposition and bounded admitted state; do not hide defects with arbitrary concurrency caps.
6. **Use existing development owners:** BC3 owns profile adoption; exact reruns and compiler boundaries
   can improve independently. Optional/behavioral kernels and diagnostic tools follow active consumer
   value rather than becoming prerequisites to catalog work.

The next consequential design decisions are therefore plural: current derivation and ranking
contracts, complete captured-result eligibility, and the chosen scope of native fencing changes.
The original's single fence-first sequence is unnecessarily restrictive. A smaller safe correction
can close its own obligation while a wider alternative remains Proposed.

## Strengths, remaining uncertainty and disposition

Independent inspection supports the original's preservation emphasis. Additional strengths include
petgraph SCC scheduling with deterministic order, Summary semantic-progress/Pareto separation,
compact documentary synthesis with later selected hydration, batched embedding dedup/refusal,
a retained evaluation subprocess, native-thread drain ownership, selected coherent backup, independent
restore, archive validation, exact completed server-descriptor reuse and consumer-aware worktree
storage. These are constraints on remedies, not reasons to dismiss confirmed amplification.

The additional pass also found two smaller opportunities, retained without new principal obligations:
a metadata-shaped freshness snapshot could avoid copying all Rust bodies if Cargo/Hakari discovery
and concurrent-change detection remain equivalent; cold-backup hashing might be fused with writing
to shorten stopped-service work. Neither has measured importance. Per-call HTTP client construction
was already acknowledged by the original and is not a missed finding.

No exhaustive phantom-safety proof, engine plan, conflict share, representative library volume,
peak live memory, compile/link cost or speedup was established. Complete transfer/consequence
semantics, every lexical/search lowering path, full Python middleware/resource behavior, server-side
gRPC scheduling and all migration bodies remain unexamined. Installed vLLM source was sampled at
the declared custom version; runtime performance/fairness was not qualified. Protected gold/heldout
inputs, operator databases and the shared service were untouched.

**Final judgment:** independently **Revise** for the supported scope. A1 and A4 violations stand;
A2 needs the named contract decisions; inspected A3 composition is a strength. Narrow G3/G7 failures
and unresolved fidelity/reuse boundaries remain, without turning every efficiency issue into a
semantic gate failure. All 50 original findings and 18 impacts have comparative dispositions in the
supporting matrices; this principal adds six owner-specific efficiency findings and preserves
separate closure obligations.

This is a dated architectural audit, not release qualification or an implementation plan. The user
will reconcile the original and audit during plan creation, choose dispositions and approve any
rule changes there. The original review and evidence remain unchanged. Documentation publication
results and preservation checks are recorded in the linked evidence folder; product execution is
**not_run** under the agreed mode.
