# Efficiency alignment: native persistence and lifecycle

**Proposed, 2026-10-11.** Supporting plan for the new
[coordinator](efficiency-alignment-plan_2026-10-11.md), which owns baseline, rule decisions,
progress, finding disposition and combined completion. Existing plans were not read or superseded.
No product implementation occurred. The approved decision investigation was executed on 2026-10-11
at `439f3d2dd73135cfada43a58e90361a56a96668e`; selected mechanisms and finite evidence are below.
All product findings remain **Planned**. Design choices are Proposed until the architectural
owner/ADR is updated at implementation; finite design checks do not establish product acceptance.

Decision evidence: [native mechanisms](../design_review/evidence/2026-10-11_efficiency-design-decisions/native.md),
[finite probes and raw result](../design_review/evidence/2026-10-11_efficiency-design-decisions/raw/native-finite-results.json).

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

**Mechanism selected, Proposed, 2026-10-11.** Native `install_window(owner, mutation_permit,
ImmutableWindow) -> NativeWriteReceipt` plans anchors/payloads/roles/holds together and commits them
with the outcome in one guarded transaction. Budget SQL, binding, payload, guard and hold expansion
together; split only at owned reachability boundaries. `PreparedNativeRequest` carries the complete
digest once internally; public retained requests compare supplied bytes independently. Keep durable
intent/outcome, bind only used intent controls, and let CTL own cancellation-aware jittered retry
backoff/observation. A stopped retry reconciles the same request before returning its disposition.
Reject no-intent writes and replacement request identity on ambiguous acknowledgement. Compiler
ingress and Loader migrate onto this owner. Finite rollback/lost-ack countercases passed; native
collision/ack/retirement races remain future implementation controls.

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

**Mechanism selected, Proposed, 2026-10-11.** Model constructs nominal `CheckedCanonicalBatch`
qualified by relation/model/schema/canonical codec; raw/imported inputs independently earn it.
Native's `accept` transfers charge and returns an ingress sequence, while `await_committed` returns
the durable request/disposition; enqueue acknowledgement never means database completion. Finish
closes acceptance and drains accepted sequences/cleanup before contribution completion, including
cancel/failure. One flattening original iterator windows across chunks within combined charges.
Held frozen memberships use keyset pages with strict cursor/order/EOF checks and page drainage before
new writes; protected spill is only the fallback for sources without exact paging. Compiler C07
owns provider scheduling; native owns remote drain. Reject unbounded queues and whole-corpus
collection. Finite charge/cross-chunk/keyset controls passed; actual transport/provider controls remain.

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

**Mechanism selected, Proposed, 2026-10-11.** Use 64 format-versioned **write** admission stripes
(`H(operation) mod 64`), plus exact object/owner/history-target/lineage guards. Intent/payload update
their stripe; admission close/open and era cut atomically update all stripes and the marker, retaining
current-era maintenance while new admission is closed. Dedicated backup-gate writes protect the
global cold-backup predicate; selected logical backup retains transaction/pin semantics. Every hold,
outcome reference, contribution/binding/cleanup/effect incidence and successor insertion/removal
writes the exact predicate guard its collector checks. The evidence inventory covers every history
eligibility phase, not merely retirement. Only actual attempt/pin/backup/lifecycle owner types with
a named ordinal consumer retain monotone allocation. Ordinary installation effects use an owner
derived from installation identity and operation UUID, with its exact guard/local epoch created in
that operation's stripe transaction; they never allocate a global ordinal or update the installation
record per window. Retries retain this owner and its effect/reference/history incidence until terminal
cleanup. Replace cross-owner epoch ordering with
request-bound expected object incarnation/tombstone revision; retries never refresh expected identity.
Migrate these together under closed/drained format transition. Reject read-only era checks, stripes
without exact predicate guards and a global ordinary write. The coordinator owns native conflict
experiments; finite conflict-graph evidence establishes only the selected abstract protocol.

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

**Implementation gate:** the dedicated engine controls establish the selected write-conflict premises.
Exercise the complete application predicates and the 64-stripe cut with focused actual native controls
before replacing their guards. This is functional protocol evidence, not a benchmark. If implementation
cannot establish a predicate's protection, preserve its existing guard and record replacement as
incomplete; N01/N02 remain executable. This fallback is not closure of the global-write finding.

**Closure:** era cut versus uncommitted intent, incoming hold/backup admission versus retirement,
outcome reference versus history deletion, and successor versus predecessor page replay are protected.
Record the selected protocol in the architectural owner/ADR before changing `ERA_GUARD`.

<a id="N04"></a>
### N04 — Hold exact retained incarnations and publish installed-view completion

**Mechanism selected, Proposed, 2026-10-11.** `hold_expected` produces a `HeldCandidate` naming exact
physical record/incarnation and logical completion; compiler receiver preflight precedes attachment.
Exact-spec hits hold existing contributors directly. Native publishes `InstalledViewCompletion/v1`
only over the **view-scoped** frozen six-family dependency closure, selected logical→physical mappings, membership digest and
checked EOF/backing/extra-row validation. Physical `view_realization_id` hashes logical view, sorted
selected records/incarnations and member codec; direct addresses are
`H(native-view-member/v2, view_realization_id, relation, key)`. `HeldInstalledView` carries that
namespace/completion and holds through reads; missing rows imply absence only under it. Incomplete
installation remains privately rooted and resumes/rebuilds before visibility. Completed contributors
and their exact binding/alias/backing inventories freeze independently; unrelated attempt producers
continue. N06 later freezes/composes the complete admitted attempt, avoiding a C04/final-freeze cycle.
Reject count-only
completion and first-logical-match recovery. Compiler C03/C04, selected backup, native recovery and
retirement migrate together. Finite stale/prefix/same-count-substitution controls passed; native
attachment/cold-recovery controls remain.

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

**Compiler handoff:** C04 can multi-get addresses `H(view_realization_id, relation, key)` under this fact.
A missing row proves absence only under completed installation. Retrieved relation/key/node/content
still receives semantic checks. Cold import/restore earns fresh completion and physical validity.

**Closure:** no copying hit path; preflight rejection has no attachment effects; fabricated cardinality,
interrupted prefixes, stale incarnations and missing/extra members refuse; backup/recovery retains
referenced content after source-attempt close. Shared retirement consumers migrate with the interface.

<a id="N05"></a>
### N05 — Transfer a publication staging root rather than every child hold

**Mechanism selected, Proposed, 2026-10-11.** Create `PublicationStagingRoot` under the attempt before
any child; Loader/reference/search/statistics children hold beneath it. One guarded marker transaction
compares equal marker bytes and installs publication→root reachability with sealed root state. Release
attempt ownership afterwards; committed marker reporting survives cleanup/readback errors. Competing
equal publishers converge under the publication guard; nominate a losing/abandoned root durably before
last-owner release. Preserve existing paged retirement and lineage. Reject flat per-child transfer
and undiscoverable private preparation. Portable recovery rebuilds destination roots. Finite crash
prefix/shared-child cases passed; actual pre-child/mid-production/post-marker and paged retirement
interruptions remain controls.

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

**Mechanism selected, Proposed, 2026-10-11.** Model `LogicalCompletedIdentity/v4` composes ordered
family-tag/count/canonical-row-digest roots for contributions, memberships, views, typed backing,
bindings and aliases, computed once at protected freeze. Normalize physical pointers to complete
logical meaning, retain explicit empty views/selectors/absence, and reject conflicting duplicates.
Native freeze closes/drains producing work and all identity-bearing mutation permits, including generic
row, alias/backing/binding/descriptor/import routes; derived staging has a separate capability.
Persist family roots only after exact actual closure/backing checks under that inventory revision.
Seal composes the protected roots without repeated full-state/original reconstruction. Physical
completion/incarnation remains N04's independent installation-local witness. Reject XOR/digest-sums,
binding-revision-only freeze and custom incremental Merkle machinery. Model, compiler, manifest,
detached transfer and restore version together. All six finite family mutation/order controls passed;
native late-mutation and independent cold admission controls remain.

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

**Mechanism selected, Proposed, 2026-10-11.** Lower each derived family once into an owned bounded
or spilled expected stream, install it, then compare actual stored key/full-row/reference/context/
statistical content once, including missing and extras. Keep semantic admission independent from
this physical reconciliation. Verify trusted definitions once under their immutable epoch hold.
Ordinary seal consumes N04/N06/N10 validity and freeze; stale/absent witnesses require fresh checking,
and cold restore/audit keep independent admission. Reject repeating the producer as an oracle and
skipping checks before the witness exists. NP, Loader/reconciliation, derived search and definitions
migrate together. Finite missing/extra/changed detection passed; actual corruption/premise-change and
committed-marker-after-cleanup-failure controls remain.

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

**Mechanism selected, Proposed, 2026-10-11.** `DerivationIdentity/v1` includes responsible lowering,
lexical/statistics/vector/context code, model/physical codecs and trusted definition epoch. Qualify
every derived address/index/eligibility predicate, staging root, marker and handle with it. Same-format
named pinned realizations coexist; every reader/resource/continuation uses its handle's namespace.
`try_existing_publication` compares full manifest/derivation/definitions/marker and exact reachable
completed views/bindings, then acquires an expected-incarnation hold atomically under the retirement
guard. A hit performs no derived staging. Reject handle-only hashing, “latest” fallbacks and check-then-
pin races. C01/D02 narrowing preserves relevant native source. Serving, publication, backup/restore
and retirement consumers migrate together. Finite namespace/shortcut mutation controls passed;
actual two-derivation, identical publication and retirement races remain.

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

**Mechanism selected, Proposed, 2026-10-11.** Native `retain_product`/`release_product` give warm reuse,
view/publication/backup/reader/recovery consumers named incidence. Last-owner release atomically
nominates exact roots before incidence deletion; attempt close includes failed/incomplete/no-product/
artifact-only roots. `admit_terminal_candidate` checks reusable outcome/spec/capture/complete physical
closure under a recovery hold and earns current admission, independently of source-attempt admission.
Recovery terminal/qualified-expiry transition rechecks actual borrowers and nominates before release;
expiry never authorizes deletion alone. Canonical complete `BindingDescriptor/v2` hashes selectors,
roles/source/boundary/configuration/view and explicit absent/unrequested meaning; attempt incidence
releases only after authoritative view/publication/recovery holders exist. Migrate selected backup
to exact publication binding inventory first; HIST still checks all other attempt consumers. Reject
age/size collection, permanent product roots and view-only binding keys. Finite shared-root/binding
controls passed; interrupted release/recovery and fully eligible native compaction remain controls.

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

**Current correction, Implemented / source-inspected, 2026-10-14:** the selected route conservatively
recaptures original content with current exact owner observations. The former insertion-only predicate
proposal is not selected. Active incarnation, tombstone, exact layout/header/holds and complete bytes,
hash/range/EOF validation remain required; a refreshed guard alone never proves unchanged content.
[Catalog completion §6](catalog-completion-plan_2026-10-14.md#6-sole-disposition-and-current-checkpoint)
owns affected current Catalog/cold evidence. This does not close the wider N10 retrieval/recovery,
selected backup/shared retirement or equal-peer replay, intervening piece mutation and stale
incarnation/tombstone matrix. The original failed observations remain in this coordinator §4; no
specific changed guard field is claimed as their measured cause.

**Mechanism selected, Proposed, 2026-10-11.** `OriginalBlob/v1` header carries full-content hash,
length/format and earned completion; bounded chunks use `H(blob, offset)`. Revision/path-scoped
`OriginalLayout` references/holds the blob and preserves logical ArtifactChunk rows through hydration,
without another stored per-artifact body. Blob owns chunks; full-byte collision checks and contiguous
exact `[0,length)`/hash/EOF validation precede completion, with explicit valid empty headers. Migrate
ingress, projections/hydration, Loader/reconciliation/source retrieval, backing validation, all recovery
families, selected backup, detached transfer/restore and retirement together. Reject artifact-keyed
duplicate bytes and a second chunk-dedup layer. Finite malformed/empty/shared-revision controls passed;
actual interrupted/concurrent install, range reads, shared retirement and cold round trips remain.

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

**Mechanism selected, Proposed, 2026-10-11.** Compiler `AdmissionPremise/v2` hashes model/invariant,
explicit validator-factory and relevant producer implementation, profile/configuration, ordered input
roles/requested-resolved selectors/absent-unrequested state/exact logical dependencies, physical schema/
codec, installation/validity epoch and held record/incarnation/completion IDs. Factories declare all
dependencies; undeclared ambient input makes durable reuse ineligible. Native validates held physical
completion before use. Missing/stale/failed records recheck; corruption errors never become quiet
misses. Choose cold invalidation: host service owner changes physical-validity epoch under closed/drained
restart/privileged mutation; restore/import earns fresh completion and premises. Reuse spans compiler
processes within the same protected service epoch, not arbitrary daemon recovery. Reject declaration-
only and portable trust. Factory registration, W premise keys, native completion and service lifecycle
migrate together. Finite exact-key mutations passed; actual factory/incarnation/selector/restart and
corrupt-backing controls remain.

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

**Current cutover, operator-directed, 2026-10-12 (ADR-0161).** Migrate useful views into
current exact realizations where practical; explicitly regenerate unsupported design data from
pinned inputs. No legacy runtime code/data, opaque preservation store or historical compatibility
reader survives. The dedicated campaign's `library_context` namespace, databases `main` and
`validation`, contain disposable fixture outputs: regenerate both through current focused producer,
compiler, publication and serving controls. This is a regeneration disposition, not a claim that old
views were semantically migrated. The interrupted original upper fixture remains unknown in its
receipt and receives no retrospective acceptance.

The existing service owner must bind the exact campaign installation, two database names, installer,
interrupted run and attachment; acquire exclusive admission/borrower ownership; confirm actual child
cessation and cooperative joined daemon shutdown before replacing data; recreate current configuration,
users and schema; independently read back target installation and absent legacy definitions before
opening admission. Failure leaves the owned transition closed and resumable. No shared paused service,
operator database or client registration is affected. Unknown old effects cannot become a success
merely because processes exited. No old-revision fixture, historical semantic exporter or benchmark is
a prerequisite. Remove the transition-only converters/adapters and frozen historical fixture path.

For later actual retained product views outside this disposable campaign, the plan must name exact
current consumers and either their practical migration or pinned regeneration before replacement.
A format constant bump or a declaration of equivalence is insufficient. No implicit reset follows
from ordinary tests or this scope's completion.

N01/N02 proceed independently. N03 gates only fencing replacement. N04 supplies compiler C03/C04.
N06 precedes the N07 fast path and N11 trust reuse. N08 precedes a general retained-manifest shortcut.
N05/N09/N10 share lifecycle surfaces but remain separately closable. Schedule consumer migration with
each producing change, not in a trailing catch-all phase.

**Cold hash lowering selected, Proposed, 2026-10-11.** Hash each asset through the exact reader
supplying its tar member and collect the resulting inventory; append `recovery.json` after those
members. Restore already locates metadata by name rather than first-entry position. Use one
non-seeking archive writer that loops through short writes and hashes only bytes actually written,
including tar headers, padding and final trailer. Publish its digest/pointer only after tar close,
writer flush, file fsync and containing-directory fsync have all succeeded. Any short asset read,
unsupported seek, write/finalization/fsync error leaves an incomplete unreferenced archive, preserves
the original failure and still runs the exact service restart/recovery owner. Do not publish a
prefix hash or hash reconstructed logical contents instead of finalized archive bytes.

Preserve cold quiescence, exact source inventory/identity and post-copy coherence, private permissions,
no-overwrite admission, independent restored-asset hashing, generation/executable/coordination checks,
destination semantic admission and selected logical closure. This removes the asset prehash and final
archive reread from the proposed writer; it does not remove independent ingress checks. The
[finite tar/hash control](../design_review/evidence/2026-10-11_efficiency-design-decisions/cold_hash_probe.py)
passed six synthetic I/O groups on 2026-10-11. It establishes framing/hash/failure properties, not
actual service backup/restore acceptance. Implement inside `surrealdb_service.backup_service`, then
exercise the actual restore staging and durability faults with synthetic private assets before a
separately authorized dedicated-service round trip. The current service route remains in use until
that implementation passes; no new design alternative remains open and no stopped-service saving
is Measured. Cold backup timing stays external M02.

Functional completion includes actual native conflict/recovery, selected backup/restore and retirement
controls for the changed contracts. Mocks cannot establish persistence or drainage. The coordinator
schedules these integrated boundaries once the relevant consumers have migrated. Performance and
real-library campaigns are separate; none of the proposed savings is Measured.

## 4. Decision-investigation completion

**2026-10-11:** N01–N11 have selected mechanisms, rejected alternatives, concrete ownership/identity/
failure interfaces and adjacent consumer/migration obligations in this plan and the linked native
evidence. The finite synthetic groups passed; coordinator-owned engine and cold-path receipts retain
their own scope. Product implementation, integrated native controls, full transition qualification,
performance and operator adoption remain **not_run** by this worker. Every native product finding
remains **Planned**. No ADR, production change, service operation, commit, generator or repository
quality check was performed by this worker.
