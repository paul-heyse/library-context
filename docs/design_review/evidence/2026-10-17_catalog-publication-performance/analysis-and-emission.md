# Analysis preparation and typed emission: bounded source inquiry

Date: 2026-10-17. Source baseline: `7f353fa1fffd081eb9f25003b61cbea9f74ded67`.

This note supplies evidence to the [principal target review](../../reviews/design_review_catalog-publication-performance-target_2026-10-17.md).
It is a bounded inquiry into Analysis preparation, recursive Summary production and typed output.
It is not an overall acceptance verdict or the current disposition ledger.
The functional target is the pinned API/evidence catalog plus explicitly requested behavioral semantics;
existing ADRs and physical row protocols are evaluated constraints, not the definition of correctness.

Method: read-only source and retained evidence inspection. No new builds, tests, services or probes.
Source observations below describe Implemented mechanisms at the baseline. Corrective designs are Proposed.
No runtime improvement is Measured by this inquiry.

## Evidence boundary and causal interpretation

The recursive control is the seven-function fixture in
[`scoped_summary.rs`](../../../../crates/cpg-core/tests/fixtures/scoped_summary.rs),
with the full Analysis frontier, 4 GiB and a 900 s boundary.
The retained `20261017T204229.825Z-e713d0` native log shows the Summary kernel returned before
the timeout during emission; nineteen 4,096-row PairOutcome batches were acknowledged (77,824 rows).
Those acknowledgements establish a publication prefix, not the complete result cardinality or proof mix.
The recursive semantic oracle was not reached. The ordinary control passed in approximately 874 s.
The `68c73f` physical 4,096-row probe passed in 15.714 s; it is not the recursive semantic oracle.

Retained timing places Summary start 595.618 s after initial native setup and Base at 96.679 s.
Nine cohort-membership captures total 45.093 s and unique-payload captures total 37.533 s,
but these intervals overlap. They cannot be added or assigned wholly to any source mechanism below.
The log owner is `build/runs/20261017T204229.825Z-e713d0/verify-evidence/87a497f550e24c70ac1c0b4cee8360e7/native.log`.
The recursive attempt is `d385e9d9ba754221907b49ec297a954bb9b879cd61b327f9fd7b209aa9d08018`.

That attempt records 12 calls, 13 bindings, 22 flow paths, 38 value-support rows, 16 local
contributions and 16 local alternatives; captured/model alternatives/model supports/source headers
are zero. These counts constrain explanations, but do not reveal final Summary proof/output counts.

## SO01 — Attempted pair trace and semantic question universe are coupled

The kernel pairs current caller/callee owner branches; the early filter constrains the caller
output to an occurrence, while exact binding compatibility is checked later during composition.
See `summary_production.rs:1738` and `composition.rs:984`.
Pair records distinguish Disjoint, Subsumed, Refused and Proven outcomes
(`summary_production.rs:2643`).

The consequence stage derives its question universe from all pair outcomes and residuals,
grouped by event, caller input, callee output and caller qualification
(`summary_consequences.rs:918`).
The later decision loop uses refusal causes and proven witnesses
(`summary_consequences.rs:1043`).
Thus a failed/no-op candidate can currently determine whether a question exists at all.
Early pruning is not justified solely because an individual pair cannot produce a transfer.

Useful semantic cardinalities are distinct: accepted proofs P, qualified finite questions Q,
closure questions K, their alternative-membership total ΣA, residuals R, and vocabulary V.
The attempted pair count is neither the transfer count nor automatically the necessary published count.
Its controlling recurrence is frontier growth: each accepted new branch changes later caller/callee
branch products and can enqueue further component work. The pair trace is the sum of the candidate
products actually visited as that frontier evolves, subject to filters and finite-work refusal.
It is not a fixed product of seven functions or twelve call edges. Exact final totals and the
disposition/proof multiplicities remain unobserved in the interrupted recursive emission.
Current proof derivations/discharges scale with P, conclusions with Q, and closure membership
with ΣA; these obligations do not independently require every attempted branch pair.

Synthesis does not directly consume PairOutcome: it consumes conclusions/proofs and generic
derivations (`synthesis/summary.rs:30`, `:77`).
Native request preparation consumes finite witnesses and path DAGs
(`native_requests/preparation.rs:769`).
Proof compaction therefore requires adapting real consumers, not merely removing trace rows.

Proposed alternatives: declare the finite question universe independently of pair attempts;
or retain a lossless factorized representation of candidate coverage, qualifications and refusal causes.
Then index/prune impossible combinations before constructing per-attempt records.
Preserve qualified endpoints, non-proven questions, exact binding/guard premises, Unknown/refusal
semantics, closure-member coverage and finite witnesses. Preserve all proof distinctions that a
consumer actually uses; do not assume every equal-cost proof is either necessary or redundant.

Closure: compare semantic question/conclusion/closure results under disjoint, subsumed, refused,
proven and residual-only cases. The existing all-family replay oracle couples acceptance to the
current physical trace; retain its fidelity checks while adding an independent semantic comparison.
Its shared production route is not an independent proof of the current question-universe design.

## SO02 — Whole-result retention and row emission multiply representation work

Summary owns all 44 record families and 14 vocabulary families. Its coordinator prepares jobs,
then `owned_work::ordered` joins all jobs before the first result is emitted
([`semantic_summaries.rs:305`](../../../../crates/cpg-core/src/semantic_summaries.rs#L305),
[`owned_work.rs:70`](../../../../crates/cpg-core/src/owned_work.rs#L70)).
Publication borrows the full result frame
([`semantic_summaries.rs:369`](../../../../crates/cpg-core/src/semantic_summaries.rs#L369)).
The generic emitter clones rows before handing them to output
(`producer_operations.rs:26`);
the workspace push validates/downcasts and locks the typed row buffer
(`workspace.rs:3253`).
Batch construction validates IDs again and sorts/deduplicates
(`record.rs:693`).

Consequently already-emitted source backing remains owned by the frame, other completed jobs
remain retained, and output buffering adds another live representation. For rich records,
the pre-push clone also precedes output reservation. PairOutcome itself is heap-free, so that
particular clone concern must not be used to explain its recursive emission volume.

Proposed: a consuming canonical-batch handoff with reservation ownership transferred into the
producer output, plus deterministic prefix publication as ordered jobs become available.
Retain attempt-owned partial effects, exact input identities, coherent completion, cancellation,
and joined failure/worker finality. Completion may not race ahead of failed or unjoined producers.
Bound both unfinished work and ready results by existing resource ownership, not arbitrary thread caps.

There is an existing `BatchWriter` with preallocation and cross-flush digest deduplication
(`batching.rs:25`, `:74`); compare it with the output
writer's pending vector before adding another framework (`workspace.rs:2876`).
Closure should include rich heap-owning rows, repeated IDs spanning flushes, a later failing job,
cancellation after a visible prefix, resource pressure, and deterministic replay across schedules.

The analogous typed-output survey covered normalization, Local, semantic execution, model
production, C0/C1, structural/analytic production, synthesis preparation, retrieval preparation,
final coverage and provider batch output. The recurring structure is buffered typed rows followed
by row-oriented emission; family-specific lifetimes and ordering differ. This is shared-interface
work, not a Summary-only optimization. Selected anchors: `local_semantics.rs:409`,
`semantic_execution.rs:131`, `retrieval_preparation.rs:1250`, `workspace.rs:3756` (provider batch output).

## SO03 — Repeated kernel preparation and global searches

Composition allowance traverses maps per pair; composition clones literal/segment maps for a
small overlay and performs guard substitution at multiple points
([`composition.rs:429`](../../../../crates/lctx-model/src/domain/composition.rs#L429),
[`:1033`](../../../../crates/lctx-model/src/domain/composition.rs#L1033), `:1068`, `:1138`).
Summary publication scans global branches per component, and proof-source aggregation repeatedly
searches IDs (`summary_production.rs:2108`, `:2189`). Consequence production scans origin
boundaries per question (`summary_consequences.rs:1087`).

Proposed: immutable base plus a small borrowed overlay, direct correspondence indexes and
separate cached guard computation from final qualified-evidence construction. Preserve exact
substitution vocabulary, source correspondence, qualification and budget ownership. A cache keyed
only by endpoints or a textual condition would be insufficient. These are source-level repeated
work opportunities; no measured dominance or need for a replacement solver is established.

## SO04 — Scheduling opportunities have semantic and deterministic limits

The upper compilation coordinator is already readiness-driven
(`compilation.rs:1350`, `:1511`).
There is no supported finding that it retains a retired whole-frontier barrier.
Remaining grain serialization appears in Local (`local_semantics.rs:164`), Base expressions
(`semantic_execution.rs:488`), SourceCalls (`:1336`) and Enriched (`:1813`), with whole-window
joins in several paths. Independent grains can be considered for bounded ordered pipelines.

SCC parallelism is conditional: Summary has a global progress meter (`summary_production.rs:2496`)
and bounded per-component worklists (`:2536`). Changing schedule can change which work reaches
the finite limit unless a canonical budget prefix is preserved or the semantic budget contract changes.
A fixture with one input/context frame also receives no benefit from frame-only parallelism.

## SO05 — Discarded rich-input prepasses precede selected acquisition

The semantic loader streams declared input rows through coverage admission; `load_ignored`
installs a no-op payload visitor
([`semantic_execution.rs:30`](../../../../crates/cpg-core/src/semantic_execution.rs#L30), `:94`).
The consumed-port owner tracks dispatched declarations and checks completeness, while streaming
starts from `SELECT *` with canonical ordering
([`cpg-core/consumed_rows.rs:37`](../../../../crates/cpg-core/src/consumed_rows.rs#L37), `:96`, `:246`).

Base discards evaluation/entry payloads at `semantic_execution.rs:222` and witnesses/accesses
at `:243`, before selected preparation/acquisition at `:291` and `:445`.
Completion does so at `:723` and `:743`; SourceCalls at `:1141` and `:1175`;
Enriched at `:1518` and `:1560`. Each later acquires selected kernel regions separately.
Expected-coverage admission explicitly allows skipped families
([`expected.rs:1160`](../../../../crates/lctx-model/src/domain/analysis/expected.rs#L1160));
its inventory is metadata/coverage rather than these rich payload families (`:15`).

Proposed: distinguish semantic dependency identity, metadata needed for admission and selected
rich inputs actually consumed. Retain exact completed handles and necessary coverage premises;
avoid a full discarded payload traversal merely to dispatch an inventory port.
Local is a positive nonmatch: its decoder vocabulary is broad, but its declared metadata acquisition
is narrow (`local_semantics.rs:61`, `:203`). Closure must prove that each removed scan contributes
no unique premise and retain late-failure behavior for streams whose contents actually supply results.

## SO06 — Base capture repeats first-use queries and limits reuse to outer windows

Each Base capture creates local membership/discovery backing
([`base_scope/capture.rs:44`](../../../../crates/cpg-core/src/base_scope/capture.rs#L44)).
For each input kind it finds active roots, then queries each root's IDs absent from earlier roots
(`:134–180`). This enforces the documented first-consuming-root policy (`:169`).
Selected assembly later uses exact membership against locally captured payloads (`:297`).

Proposed: compute `(kind, id) → minimum root ordinal` once, retain exact logical memberships,
and generate the same root-preserving native requests. This can remove repeated local membership
queries without changing the root-isolation policy. Separately evaluate union point acquisition
with exact local partition indexing rather than assuming physical root isolation is required.

Supporting payload reuse across successive outer capture windows is another opportunity, using
invocation-owned spillable exact backing. Preserve source/query/context/budget identities and
terminal failure/cancellation; incomplete acquisition must never become cached success.
Global computed adjacency is already cached (`consumed_rows.rs:590`, `:677`); native forward
discovery still participates in each capture (`:1743`). Do not label all frontier preparation repeated.
Native capture occurs once before local compute retries (`semantic_execution.rs:445–487`), so
cohort shrinking already reuses captured backing. Closure: overlapping/disjoint roots, common native
references, later-window reuse, acquisition failure and resource exhaustion.

## SO07 — Completion repeats immutable syntax preparation

Completion production calls statement completion for selected statements after selecting actual
earlier evaluations (`completion_production.rs:389–484`). That route constructs a fresh
`PreparedExecution` (`completion.rs:758–819`), rebuilding syntax indexes and validation
(`evaluation.rs:324–451`, `:1343`). Body completion constructs preparation again
(`completion_production.rs:573`; `body.rs:138`; `evaluation.rs:1264`). Proof-used evaluation
correspondence also uses repeated linear filtering (`completion_production.rs:498`).

Proposed: share immutable prepared syntax and indexed actual-evaluation correspondence per
admitted body/window, retaining separate checked requests and mutable proof state.
The prepared completion interface already exists (`completion.rs:821`). Base already shares
syntax per compute cohort (`semantic_execution.rs:472–503`; `evaluation.rs:1378`), and Enriched
builds one prepared execution per owner frame (`enriched.rs:311`) reused by completion (`:165`).
Preserve selected-view validation, ambiguous placements, actual tokens, owner/context identity
and budget ownership. Closure: multi-statement bodies, competing placements and missing actual proofs.

## Family coverage, preservation and priorities

The Summary record macro covers all 44 meaningful families (`summary_production.rs:432`):
terminal witnesses; capture witnesses/contributions; exception outcomes; symbolic alternatives;
origin boundaries/pair outcomes; runs/components/component members/origins/proof origins/residuals/
call members; control witnesses; costs/path routes/path witnesses/path contributions;
premises/witnesses/contributions/keys/alternatives/supports; influences/control supports/selections/
substitutions; claims/claim members/claim standings/claim proofs/claim proof members/refutation
coverage/conclusions/obligation sources; sources/subjects/propositions/derivations/derivation premises/
obligations/discharges. Vocabulary expansion covers 14 families (`:225`). None is assumed dispensable.

Existing strengths include separated proof progress and finite work accounting, petgraph component
scheduling, actual lower-owner tokens, lazy port/signature preparation, post-exhaustion endpoint
grouping, explicit solver release, and separate cost admission. Catalog compilation does not require
unrequested behavioral production. Preserve explicit Unknown, terminal and exception distinctions.

Several barriers are genuine: Base field candidates precede dependent read kernels and sealing
(`base_scope.rs:1421`, `:1544`); ordered body completion stops at the first non-normal statement
(`body.rs:138–220`); SourceCalls uses actual predecessor bodies (`source_call_records.rs:1233`);
Enriched context rounds add premises and skip completed contexts (`enriched_production.rs:787–837`).
Its construction token checks enumeration ownership (`model_construction.rs:19–47`). These are
qualified nonmatches to blanket parallelization, replay-removal or admission-bypass proposals.

Prioritize SO01's question contract and SO02's output ownership for the observed recursive emission
problem; SO05 independently addresses broad pre-Summary work. SO03, first-use indexing in SO06
and immutable sharing in SO07 can be evaluated locally. Cross-window retention and SCC parallelism
have stronger lifetime/determinism dependencies. No finding establishes that the Summary kernel
must be replaced. Physical batch handoff can preserve meaning; candidate/proof pruning requires
explicit semantic contracts and consumer adaptation. Existing behavioral-analysis rules requiring
attempt traces or all equal-cost proofs may need revision, with their surviving fidelity obligations
made explicit rather than weakened. The principal review owns the integrated judgment.
