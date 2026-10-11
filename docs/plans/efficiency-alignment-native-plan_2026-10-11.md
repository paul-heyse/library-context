# Efficiency alignment: native persistence and lifecycle

**Proposed, 2026-10-11.** Supporting plan for the new
[coordinator](efficiency-alignment-plan_2026-10-11.md), which owns baseline, rule decisions,
progress, finding disposition and combined completion. Existing plans were not read or superseded.
No product implementation or execution occurred during authoring.

Sources: original O-F01–O-F15/O-F17; audit A-F01–A-F06/A-F18/A-F19 and
[native evidence](../design_review/evidence/2026-10-11_efficiency-alignment-audit/native-contracts.md).
The operator approved revised RC01/03/06/07/09/16/17 on 2026-10-11 and retained RC13's shared-store
logical isolation. Approval establishes design direction, not safety evidence or implemented closure.

## 1. Target and boundaries

Retain shared immutable native content, exact model meaning, explicit ownership and independent
cold ingress. Change physical operation grain, installed-view completion, current derivation
eligibility and release transitions. Logical content identity, physical incarnation, semantic
admission, current publication and selected reader lifetime remain different contracts.

The current control/outcome protocol, full-value collision checks, bounded retirement and logical
recovery closure are foundations to preserve. The audit corrected two important premises: backing
validation is already buffered despite scalar local iteration, and recovery is not limited to the
current attempt's contributions. Current concurrent source also improves root batching and selected
context eligibility; it does not close the work below.

Native owns physical validity and held realizations. Compiler owns captured-request meaning and
reuse eligibility. Publisher owns coherent realization and seal. Serving consumes a handle's exact
realization, including negative/context eligibility; it does not reconstruct native ownership policy.

## 2. Packages and contracts

<a id="N01"></a>
### N01 — Fuse bounded effects without losing request recovery

**Sources:** O-F02/O-F03 and separate O-F05 payload/hash obligations.
`compiler.rs::ensure_physical_rows`, `loader.rs::insert`, `control/authorization.rs` and
`control.rs::run_transaction` are the executing boundaries.

Provide a native-owned operation installing anchors, immutable payloads and required ownership/
reference holds atomically where their reachability permits. Budget the combined SQL, bindings and
payload footprint. Compiler ingestion and Loader consume the same owner operation. Preserve late
collision rollback, contribution write fencing, retiring-object refusal and incarnation checks.

Compute the complete immutable request identity once on the trusted internal path. Public retained
requests still reject payload mismatch. Intent submission carries only the control bindings its SQL
uses. Retain durable intent/outcome machinery: draining, reconciliation and history consumers need it.
Its elimination is a separate alternative, not a closure requirement for these local corrections.

Add backoff and bounded observation to retry handling at the existing owner. Any cancellation or
retry stop returns the same request's recoverable disposition. A lost acknowledgement is not proof
that the effect did not commit. No fixed concurrency cap is introduced.

**Closure:** late collision leaves neither partial rows nor dangling new holds; lost acknowledgement
reconciles the original request; changed retry bytes refuse; insertion/retirement races have coherent
outcomes. This package can complete before N03's global fencing replacement.

<a id="N02"></a>
### N02 — Checked ingress, useful original windows and bounded producer progress

**Sources:** O-F04/O-F06/O-F07/O-F08. Carry nominal checked canonical batches from model construction
into native ingestion, qualified by relation/model/schema/encoding. Untyped, raw and imported input
must earn that validity. Do not decode/sort/canonicalize again solely because a checked batch crosses
an internal boundary.

Window originals through a bounded iterator spanning successive chunks. Hold charges until actual
acknowledgement; do not first collect the whole corpus. The compiler schedules dependency-ready
providers, while native supplies an acknowledged sink whose completion includes all accepted writes
and cleanup. Provider-local extraction may overlap native I/O without assuming improved store write
contention. Failure and cancellation prevent premature contribution completion.

Replace membership/hold statements held open across write loops with keyset pages over a protected
immutable source. Use an owned bounded spill capture only where paging cannot retain exact semantics.
Checked EOF, ordering, cursor progress and transport drain remain completion premises. A transaction
setting alone does not define the lifetime of an external backup snapshot.

**Closure:** checked ingress avoids duplicate canonicalization; raw conflicts still refuse; windows
cross chunk boundaries without exceeding combined charge; cancellation drains queued writes; shuffled
and interrupted paged streams neither omit nor duplicate members. Pair with C07 provider integration.

<a id="N03"></a>
### N03 — Replace global ordinary-effect coordination through a bounded safety decision

**Sources:** O-F01/RC01. The target removes the single installation-record write from independent
ordinary effects where equivalent protection can be established. It does not promise that a read-only
era comparison is sufficient.

Inventory the actual finite protocol predicates: admission, intent/payload, owner epochs, incoming
holds, retirement claim/finalization, backup admission, maintenance drain, era cut, history-page
eligibility, outcome references and successor lineage. Separate monotone revision allocation from
fencing. Preserve maintenance authority while new admission is closed.

Choose exact object/predicate guards where sufficient; use a finite admission/era coordination set
where cross-object predicates require shared conflicts. Read-only checks are eligible only when the
pinned engine and surrounding protocol establish protection. Finite stripes are an alternative to
one shared record, not a new general scheduler. Old effects never execute under a successor epoch;
successor recovery owns the predecessor's persisted remaining scope.

**Investigation gate:** inspect pinned transaction semantics and use focused eventual native controls
to settle the competing read/write cases. This is functional protocol evidence, not a benchmark.
If a predicate's protection remains unresolved, preserve its existing guard and record that replacement
as incomplete; N01/N02 remain executable. This fallback is not closure of the global-write finding.

**Closure:** era cut versus uncommitted intent, incoming hold/backup admission versus retirement,
outcome reference versus history deletion, and successor versus predecessor page replay are protected.
Record the selected protocol in the architectural owner/ADR before changing `ERA_GUARD`.

<a id="N04"></a>
### N04 — Hold exact retained incarnations and publish installed-view completion

**Sources:** O-F17; consumes compiler C02/C03 semantic contracts. Separate candidate discovery,
held inspection, receiver preflight and attachment. Acquire a hold against the expected incarnation,
not merely whichever incarnation currently occupies a key. Validate receiver requirements before
attachment effects. Mismatch can be a miss; corrupt content and unknown effects retain distinct errors.

Attach exact-spec completed physical contributions by reference without copying their N-row membership
into a new attempt contributor. Persist selected logical-contributor → physical-record/incarnation
correspondence at the view realization, with holds. Equivalent copies can exist; cold recovery must
validate their witnessed equivalence rather than use a process-global first logical match.

Publish a durable installation-completion fact only after the complete view descriptor, selected
contributor closure, exact membership, collision/backing checks and checked EOF are established.
Bind it to the view incarnation and membership realization/codec. Neither a descriptor nor count
agreement establishes completeness. Interrupted installation resumes/rebuilds before visibility.

**Compiler handoff:** C04 can multi-get existing addresses `H(view, relation, key)` under this fact.
A missing row proves absence only under completed installation. Retrieved relation/key/node/content
still receives semantic checks. Cold import/restore earns fresh completion and physical validity.

**Closure:** no copying hit path; preflight rejection has no attachment effects; fabricated cardinality,
interrupted prefixes, stale incarnations and missing/extra members refuse; backup/recovery retains
referenced content after source-attempt close. Shared retirement consumers migrate with the interface.

<a id="N05"></a>
### N05 — Transfer a publication staging root rather than every child hold

**Source:** O-F12. Create a durable discoverable staging root before child effects. Derived rows hold
under that root; preparation owns it through the attempt. Marker commit establishes publication-to-root
reachability instead of recreating every child hold.

Migrate Loader, references, search/statistics producers, seal, cleanup and retirement together. Nominate
an abandoned root before releasing its last owner. A committed marker remains reported as committed
if later readback or cleanup fails. Existing retirement already charges edges and resumes high-degree
roots; preserve that implementation rather than assuming an all-fanout transaction.

**Closure:** crash before the first child, mid-production and after marker commit; paged retirement;
shared child retained by another publication. Portable recovery reconstructs destination ownership
and never imports a source live attempt or marker as executable authority.

<a id="N06"></a>
### N06 — Version completed identity over the complete frozen closure

**Sources:** O-F09/RC07. Compose normalized logical content for all six completed families:
contributions, memberships, views, typed backing, bindings and aliases. Include explicit empty views,
selectors and relevant absence semantics. Existing output key/content digests alone omit parts of
this contract. Keep logical completed identity separate from installation-local physical integrity.

Completion closes and drains accepted producing work; native freeze closes all identity-bearing
mutation surfaces. Inspect generic mutation routes as well as compiler helpers. A local flag or
binding revision alone cannot justify skipping seal reconstruction. Version the contract and migrate
model, compiler, manifest, detached transfer and restore consumers together.

Equal imported logical content can retain logical identity while obtaining new physical incarnations
and fresh admission. While the new witness is incomplete, compute current v3 once with improved local
batching and preserve the remaining necessary recheck; mark only that smaller slice complete.

**Closure:** producer order does not alter content identity; alias/backing/binding/empty-view changes
alter the appropriate identity; late writes refuse; no state family disappears; restored content
cannot inherit source installation trust.

<a id="N07"></a>
### N07 — Ordinary seal consumes established admission and reconciles stored derivations once

**Sources:** O-F10/RC06. Require N06's protected freeze, drained effects, admitted logical identity
and finalization ownership. Lower each derived family once and compare actual stored rows once,
retaining missing/extra/reference/statistics detection. Verify the immutable definition epoch once
within its protected lifetime. Same-lowering replay is not an independent oracle.

Avoid repeated original readback only where admission protected the same original-byte incarnation.
Cold import/restore and explicit audit remain independent uncertainty boundaries. Preserve final
acknowledgement semantics and current-context/negative witnesses introduced by concurrent work.

**Closure:** missing, extra and changed derived rows are detected; changed premise between admission
and seal cannot pass; invalid imported backing refuses; committed markers survive fallible cleanup
reporting. The fast path is unavailable until its complete witness exists.

<a id="N08"></a>
### N08 — Bind current derivations and unchanged publication to one exact realization

**Sources:** O-F11/RC09. Ordinary fresh compilation already includes native implementation in producer
inventory. The target adds explicit current derivation identity for retained old manifests: references,
search and lexical/statistical lowering bind it into derived addresses, eligibility, publication and
trusted executable definitions. A handle-only hash addition is insufficient.

Within the new physical format, support separately named derivation realizations for genuinely live
pinned consumers. Every query/resource/continuation uses its handle's realization. Retire old realizations
when their named consumers release them; retain no historical generation merely for compatibility.
Cross-format migration follows a coordinated quiesce/rebuild/cutover, not new old-format readers.

The unchanged shortcut validates exact manifest/derivation/definitions, equal marker, reachability
and non-retiring incarnation, then acquires a guarded hold before returning the handle. Coordinate
fingerprint narrowing with C01/D02 so responsible derivation implementation remains in identity.

**Closure:** a retained manifest under two derivations never mixes derived rows; old pinned consumers
see their named realization or explicitly refuse after retirement; racing identical publication
converges; concurrent retirement cannot invalidate a successfully acquired shortcut.

<a id="N09"></a>
### N09 — Complete product, terminal-recovery and binding release lifecycles

**Sources:** O-F13, A-F18/A-F19. Keep four separate closure obligations: terminal-result reuse,
abandoned-root reclamation, obsolete retained-product release, and redundant binding incidence.

Products have named consumers and an owner release operation. Last-owner release first nominates
the root durably or transfers recovery ownership. Preserve warm reuse, views, publications, backup
and readers; age and size do not authorize deletion.

Attempt cleanup/close also nominates released native content that has no completed-product owner,
including failed, abandoned, incomplete-output and artifact-only attempts. Establish durable nomination
or explicit recovery handoff before releasing the last attempt hold. Reclamation is independent of
reuse eligibility: a rejected candidate still needs a safe release route. Recovery holds have an owned
terminal/expiry transition that rechecks borrowers/references and nominates remaining content; expiry
alone is not deletion authority. Preserve rows shared with a live contribution, view or publication.

A terminal unadmitted candidate enters admission-for-reuse under a recovery hold. Validate exact
capture/spec, reusable outcome, complete physical content and closure. Completion, poison, unresolved
effects and resource-partial output are not interchangeable. This does not require prior admission
of the source attempt; it requires current candidate admission.

Retain canonical complete binding descriptors, or an authoritative held representative, before removing
redundant attempt incidence. Key the complete binding meaning, not just view identity: requested/resolved
selectors and roles cannot be silently merged. Migrate selected backup's stored-binding lookup first.
History compaction becomes possible only when every actual consumer releases its incidence; binding
release alone cannot erase an attempt still needed by retained contributions.

**Closure:** interrupted nomination/release replays safely; no-product/incomplete-output attempts
and ended recovery holds reach reclamation without losing shared rows; obsolete products retire while
live peers survive; poisoned candidates refuse; backup works after redundant bindings release; fully
eligible terminal attempts compact. No sweep or collection is authorized by authoring this plan.

<a id="N10"></a>
### N10 — Share original bytes beneath artifact identity

**Sources:** O-F14/RC16. Use a full-content-addressed blob header `(content, byte length, format)`
with bounded chunks keyed by blob identity/offset. Artifact layout/header references the blob and
retains semantic revision/path identity. Blob ownership retains chunks. Do not introduce a second
chunk-content dedup layer without a demonstrated need.

Migrate ingestion, logical artifact-chunk projection/hydration, source retrieval, reconciliation,
selected backup, detached transfer, restore and retirement together. Reconstruct logical artifact
rows from layout/blob storage rather than recreating a serialized byte copy under each artifact.
Verify continuous ranges, lengths, full content and explicit valid empty originals.

**Closure:** unchanged bytes across revisions share storage; changed content obtains its own blob;
one revision's retirement preserves another holder; malformed ranges/hash/length refuse; interrupted
construction gains no completion. Portable recovery carries content and earns destination ownership.

<a id="N11"></a>
### N11 — Carry durable admission premises only across protected physical validity

**Sources:** O-F15/RC17. Extend premise identity with validator/factory and relevant producer
implementation, realization/codec and exact protected incarnations, retaining model/invariant,
profile, requested/resolved selectors, non-request state and exact dependencies.

Persist success only after all declared inputs were checked. Consumption acquires holds and validates
that the physical validity scope still exists. Absent, stale or failed premises trigger fresh checking;
corruption does not silently become a cache miss. Undeclared ambient-input invariants are ineligible.

Premises are installation-local trust metadata, not portable semantic content. Export/restore cannot
transport them as acceptance authority. Cold service recovery reuses them only under an explicitly
identity-preserving recovery contract; otherwise invalidate and recheck. N04/N06 supply the necessary
physical completion/protection before this reuse becomes available.

**Closure:** unchanged protected inputs reuse; validator changes, new incarnations, selector/profile
changes and portable restored records cannot reuse old authority. Durable digest labels do not replace
backing integrity.

## 3. Transition, investigations and integration

Current upgrade code names a particular source4-to-history-receipt-v5 transition, with history-specific
preflight and no data backfill. It is not a generic migrator for blobs, composed identity or derivation
eligibility. Implement each actual transition under the existing upgrade owner or use an explicit
project-store rebuild/cutover. A format-number bump alone is not completion. Preserve operator stores
and named live readers; installation/maintenance operations require their own execution authorization.

N01/N02 proceed independently. N03 gates only fencing replacement. N04 supplies compiler C03/C04.
N06 precedes the N07 fast path and N11 trust reuse. N08 precedes a general retained-manifest shortcut.
N05/N09/N10 share lifecycle surfaces but remain separately closable. Schedule consumer migration with
each producing change, not in a trailing catch-all phase.

Targeted cold-path inspection covered model recovery dispatch, selected backup, restore and upgrade.
Preserve independent destination admission and selected logical closure. The smaller cold-backup
hashing opportunity belongs here: fuse hashing with archive writing only if the exact completed bytes,
flush/finalization, ownership and independent restore checks remain equivalent; otherwise retain the
current cold route and record the unresolved optimization. Do not trade away cold coherence to reduce
a stopped-service interval. Its measured importance is an external follow-up.

Functional completion includes actual native conflict/recovery, selected backup/restore and retirement
controls for the changed contracts. Mocks cannot establish persistence or drainage. The coordinator
schedules these integrated boundaries once the relevant consumers have migrated. Performance and
real-library campaigns are separate; none of the proposed savings is Measured.
