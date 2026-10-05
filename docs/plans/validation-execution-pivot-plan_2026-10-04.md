# Validation execution pivot

**Implemented / Tested; assembled qualification composite passed, 2026-10-05.** Supporting design for the
[testing architecture coordinator](testing-architecture-pivot-plan_2026-10-04.md), which owns
F01–F04 disposition and combined acceptance. This document owns model validation registration,
physical preparation, acknowledged result reuse and the runtime/audit trust boundary.
It does not own test-run scheduling or the meaning of individual domain rules.

## 1. Current foundation and changes in responsibility

`lctx-model` owns canonical semantic definitions and complete ordered premises. `ValidatedModel`
resolves explicit relation references and records definitions/revisions in model identity. The
source-call header, invocation and run retain their references to one `source_call_replay` definition.
Generated derivation validation is also scoped and model-owned; unsupported partial models refuse.

`lctx-postgres::generations::validation_session` resolves acknowledged frames and exact conclusions.
Existing `validation_receipts` records complete binding contexts; `stage_read_checks` references them
as grant evidence. Publication, closure, stage reads and selection use this shared owner. Final
receipts also acknowledge held empty relations after seal/validation, without inventing producer
stage authority. `generations::audit` independently challenges body and aggregate digests, nominal
references, exact proof contexts and required pure/publication semantics without issuing grants.

Useful freeze premises already exist: ordinary completion takes exclusive relation locks before
its receipt/grant transition; lifecycle operations hold the generation lock; vocabulary closure
locks canonical relations and private deltas, merges append-only rows and revokes importer access.
Vocabulary prefix views filter `introduced_epoch` by the closed ordinal. Later legal appends change
the base table, but must not change earlier prefix meaning. Privileged administration can bypass
these rules. The transaction wrapper uses ordinary `begin()`, so transaction identity alone does
not establish a stable multi-query read.

The target retains the model as semantic owner and PostgreSQL as effect/receipt owner. It removes
repeated execution of equivalent obligations and routine physical revalidation of already
acknowledged immutable inputs. It strengthens closure so reuse has an enforceable premise and
places out-of-band integrity recomputation in explicit audit/repair.

## 2. Canonical obligations and complete bindings

Replace repeated relation-attached definitions with a bounded model declaration mechanism:
definitions are registered once as part of constructing the model, and relations/stage read uses
refer to their stable IDs. The model builder resolves all references and produces the validated
execution set. Do not introduce a runtime plugin registry or generic workflow engine.

Each definition supplies its identity/revision, required input roles, ordering requirements and
pure checker constructor. Required premises and references participate in the model digest, with
stable explicit encoding. Existing invariant semantics can retain their implementations; registration
changes ownership rather than moving semantic rules into the store. Definition revision changes
whenever validation meaning changes, even if its display name does not.

Model construction refuses an unresolved reference, conflicting definition, incomplete premise,
invalid required order or unsupported input mapping. Dependency closure includes every premise
of every referenced definition. A relation demanding a shared replay continues to demand it when
another referring relation is absent; no arbitrary third relation is an anchor. Supported finite
models must supply the actual complete replay inputs or refuse explicitly rather than silently
run a smaller check. Stage read sets deduplicate resolved exact bindings, not callback pointers.

A binding consists of the definition and its resolved input roles. At minimum it discriminates
store installation/generation, validated model and physical lowering identity, definition revision,
each ordered source relation receipt and closed prefix, source content/count identity, and semantic
profile/configuration affecting the answer. Definition inputs make additional dependencies explicit;
there is no ambient provider/version/configuration omitted from an answer that depends on it.
Model/configuration digests can represent this information without duplicating every field.
Content identity comes from acknowledged publication, not a fresh row scan on every lookup.

Two invocations with the same binding ask the same semantic question. A changed prefix, profile,
definition, physical layout, source authority or relevant setting is a miss/refusal according to
the operation, never a hit inferred from the same relation name. Resource limits that change
completeness or semantic outcome are binding inputs. Capacity for actual new work remains a fresh
admission question. An existing complete proof can eliminate replay work without charging fictional
replay, but it cannot grant unrelated consumer capacity.

## 3. Closure and trusted immutability

The selected normal-runtime contract trusts acknowledged store-owned immutability. It does not
promise detection of arbitrary superuser modification on every read. V1 must establish the premise
before V4 may use it; the implementation may strengthen current locking and grants where inspection
or revealing controls find a gap. Changing this trust promise requires the pivot ADR and edits to
semantic model, storage and validation owners.

For ordinary output acknowledgement, the owned transition drains in-flight writers, excludes new
writes, validates the candidate and computes canonical digest/count, records its conclusion and
revokes mutation capability before commit exposes the completed output. Define deterministic lock
order across involved relations/generation operations. An acknowledgement failure, cancellation
or ambiguous commit never makes a usable success token. Existing attempt poison/recovery semantics
remain authoritative and readers cannot observe a half-published proof.

For vocabulary closure, validation runs against the exact candidate prefix under closure locks.
It acknowledges the merged closed prefix, not the evolving canonical table. Later additions may
appear only at later ordinals; prior payloads, epoch membership and view definition must remain
stable. Candidate contributions are mutable until closure and cannot reuse a previous prefix proof
as proof of the candidate. Source proofs may be reused while the new candidate is checked afresh.

Audit the enforcement paths, not only grant statements: owner/role inheritance, service roles,
raw/prepared SQL queued before revocation, migration/administrative APIs, transaction isolation,
view replacement, cancellation and append/update conflicts. Production service roles remain
non-superuser. Do not expose a repair writer that modifies an acknowledged live source while its
trusted readers remain valid. Administrative repair quiesces readers, discards affected proofs and
rebuilds under a new generation/installation identity; no compatibility or historical runtime copy.

Physical schema enforcement replaces runtime checks only for facts guaranteed at that exact
boundary. Rust constructors do not protect arbitrary imported rows; PostgreSQL constraints do
not prove semantic derivation or coverage. Compile/schema/lowering controls independently challenge
key/reference/null/codebook encoding, while independent known answers challenge meaning.

## 4. Prepared validation and acknowledged conclusions

### Stable session execution

Resolve each required input frame once in the validation transaction. Retain READ COMMITTED under
explicit mutation-excluding locks and owned immutable source/prefix premises: every statement must
observe the same unchanged logical frames. Drain writers and finish candidate writes before
preparation, then hold the locks and installation/generation authority through validation and
acknowledgement. No later candidate mutation is permitted in that session. Consumers of already
closed sources can resolve their receipts without rescanning them for stability. Legal later
vocabulary appends must preserve the exact earlier prefix rows, membership and view definition.

If a frame remains mutable, strengthen its ownership boundary or select a stable snapshot for that
scoped case before sharing preparation. Do not merely change `transaction_on` to repeatable read:
advisory-lock/registry SELECTs may establish a snapshot before delayed writers drain. The delayed
writer control must show their committed rows are included in the acknowledged candidate.
Snapshot consistency cannot compensate for an incomplete candidate or an unexcluded writer.

Plan streams by exact frame and order. A canonical digest and checkers with the same order can
consume one charged ordered batch stream. Each checker keeps its own bounded state and finish
result; a checker cannot influence another's expected answer. Where orders differ, a scan cannot
be safely shared, or aggregate check state exceeds the admitted budget, use separate scans or
bounded partitioning. Avoid materializing the whole universe simply to make a reusable cache.
Preserve total ordering and exact physical conversion; wrong schema/decoding remains a refusal.

Memoize preparation and completed local results only inside that session. Failure/cancellation
drops buffers and does not record success. Charge live buffers, checker state and retained material
before allocation; ownership survives worker cancellation until actual work drains. Counters exposed
to focused controls count resolved frames, row scans, digest streams and checker executions; they
are diagnostic/test instrumentation, not a new public metrics service.

### Durable semantic reuse

Extend the existing receipt infrastructure to store complete acknowledged binding conclusions.
Read-check receipts remain grant evidence referencing the required conclusions; they do not become
a second authority for semantic truth. Publication/final validation establishes the definition set
required for its scope. Consumer admission derives requirements from the validated model rather
than a private list of validator names. Reuse the existing stored receipt owner, not a global cache.

Lookup resolves the exact binding and proves the referenced source/closure acknowledgements and
required complete conclusion are present. It performs current role/lease/capability/resource
admission. A miss executes validation only where the operation is authorized to establish a proof;
a strict read lacking required proof refuses rather than silently inventing coverage. Failed,
partial, interrupted, uncommitted or mismatched results never answer as complete success. Do not
persist negative results as an enduring rejection across mutable candidate repairs.

Proof schema/identity changes rebuild regenerable state. Invalidation follows new identities for
model/check/config/source/installation changes; deleting or retiring a generation makes its proofs
unusable under normal lifecycle admission. Cache reuse is not permission to use retired sources.
Local commit uncertainty and recovery follow the operation owner's existing acknowledgement rules.

### Operator audit

Provide an explicit audit operation over a specified generation/prefix/scope that independently
recomputes physical content identities and required semantic obligations under a stable read.
Prefer the existing generation/store command surface, with concrete scope and budget, rather than
a daemon or new evidence register. Audit reads and reports discrepancies; it never repairs them.
It can challenge injected privileged corruption, while normal consumers retain the stated trust
boundary. Repair is a separately owned quiesce/rebuild action, invalidating affected proof identities.
No recurring audit schedule or real-library run is introduced by this plan.

## 5. Implementation packages

| Package | Coherent change, inputs and migration | Verification revealing a wrong implementation |
|---|---|---|
| V0 — Classify obligations | Map each current validator/predicate to its semantic owner, premise and enforcement boundary. Remove structurally intrinsic/duplicated checks at their consumers; retain semantic replay and independent challenges. Update owner text and schedule the trust decision with coordinator P0. | Trace bypass-capable writers; malformed external rows fail at the declared boundary. A generated schema agreeing with itself does not establish domain meaning. |
| V1 — Complete freeze ownership | Strengthen `generations/{ddl,receipts,vocabulary,mod}.rs` and relevant lifecycle/repair paths so ordinary outputs and closed prefixes meet §3. Supply stable validation transaction entry and documented lock/read order. | Actual application/importer roles, inherited privileges and queued/in-flight inserts. A delayed writer commits while closure waits and its rows appear in validation. Cancellation/commit ambiguity creates no acknowledged success; legal later prefix append cannot alter an earlier frame. |
| V2 — Declare once, reference explicitly | Migrate `domain/model.rs`, `domain/stages.rs`, dependency closure, invariant declarations, `source_call_records.rs` and all factory/read-use consumers to §2. Model registration is coherent; one definition revision enters identity once and every required reference remains visible. | Missing definition/premise refuses. Any supported referring relation retains coverage. A source-call binding executes once, while malformed header/invocation/argument inventory still fails; different binding executes separately. |
| V3 — Prepare once under bounded stable inputs | Refactor publication, stage-read, vocabulary-close and final validation into session-owned preparation/execution. Stream compatible checks/digests together. Integrate V2; remove duplicate physical work only under the proved boundary. | Count one scan per compatible frame/order, one replay per binding; ordered/unordered/incompatible cases preserve answers. Tiny budgets and cancellation release owned state; candidate mutation cannot reuse stale preparation. |
| V4 — Reuse acknowledged proofs and expose audit | Change receipt schema, grants/lease consumers, `selection.rs`/`selection::admission`, strict native/serving readers and final/publication/closure consumers together. Routine reads reuse exact complete proof and do fresh admission; audit recomputes identities/checks. Rebuild affected stores after quiescence. | Repeated same-binding admission has no replay/rehash; changed source/prefix/revision/profile/config is a miss/refusal. Proof hit with inadequate current capacity refuses the new work. Privileged corruption is detected by audit; retirement/repair invalidates authority. |
| V5 — Delete obsolete validation paths | Remove unused factories, repeated predicates, old receipt format/readers, private name lists and tamper assertions against excluded routine-read promises. Enduring semantics/rationale live in model/storage/validation owners. | Static consumer audit and focused controls verify no legacy authority remains; new assembled qualification owns integration. |

V1 and V2 can develop independently with root ownership of model/store shared contracts. V3 needs
their implemented freeze/binding interfaces. V4 requires V1 revealing controls to pass, V3 behavior
and the receipt model; no persisted skip may precede that readiness. V0 applies incrementally to
each migrated boundary, not a blanket removal before examining bypasses. V5 deletions belong with
each replacement, with a final audit for stragglers.

## 6. Scenarios, acceptance and limits

A future relation needing source-call replay adds a reference to the existing obligation and its
premise mapping; it does not add another replay factory. A new semantic rule adds one definition,
references and independent positive/negative challenges. A later vocabulary prefix creates a new
binding without invalidating the earlier immutable prefix. A changed budget that affects semantic
completion rebinds validation; a new reader that needs memory still admits that memory now.
A physical layout/model change rebuilds acknowledged state instead of trusting old receipts.

Acceptance requires canonical complete coverage, actual writer exclusion/draining, stable prefix
meaning, exact proof dependency discrimination, truthful failure/partial/commit states, current
resource admission and explicit audit behavior. Use release compile/targeted model controls and
real disposable PostgreSQL 18 controls during implementation. Never replace the store with a mock
to claim these outcomes. Structural controls may deliberately inject through privileged access
to challenge lowering or audit, but that is distinct from a legitimate runtime mutation promise.

The supported trust boundary and reduction in duplicate work can become **Tested** after the
new controls pass. Improved wall time or memory remains **Proposed** until comparable measurements;
no baseline legacy suite run is required. Whole-series qualification and finding closure belong
to the coordinator, and test harness/gate execution belongs to the verification plan.
