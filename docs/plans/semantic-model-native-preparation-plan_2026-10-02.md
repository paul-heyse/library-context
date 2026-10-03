# Native preparation and refusal fidelity

**Proposed implementation plan · 2026-10-02.** This companion develops F01/F02 of the
[incremental review](../design_review/reviews/design_review_semantic-model-incremental-alignment_2026-10-02.md).
The [coordinator](semantic-model-incremental-alignment-plan_2026-10-02.md) owns sequencing,
current disposition and combined acceptance. Creating this plan implements no correction.

## 1. Baseline and intended result

Source inspection on 2026-10-02 used main `42551010` plus the preserved working tree.
`lctx-model::domain::native_requests` already owns checked Entry/context/path operations and
finite assessment. PostgreSQL still chooses structural correspondences, composes prepared
paths and reconstructs permitted formals. The intended result is one ordinary pure model
operation for that meaning, wrapped by the existing generation service for effects.

Authorities are [§15.12](../design/sections/semantic-model.md#section-15-12),
[§11](../design/sections/synthesis-and-serving.md#section-11), ADR-0085 and ADR-0114.
This realizes their ownership boundary; it introduces no new crate, general executor,
semantic frontier or native capability. Runtime support remains the existing finite model.

Reusable foundations include opaque Entry/native/rebase constructors, charged typed rows,
the prepared selection owner, shared invariant validators, epoch-aware canonical reads,
actual stored-row membership checks and the original generation guard. Selection’s pure
local preparation explicitly grants no repository admission; native preparation follows
the same distinction. These conclusions are static, not fresh functional qualification.

Decisive source routes:

- [inventory.rs:79](../../crates/lctx-model/src/domain/native_requests/inventory.rs#L79)
  and [ingress.rs:37](../../crates/lctx-model/src/domain/native_requests/ingress.rs#L37)
  establish exact context refusal causes.
- [native_service.rs:130](../../crates/lctx-postgres/src/generations/native_service.rs#L130)
  and lines 260–752 perform correspondence, replay and path assembly; lines 826–875
  reconstruct the public member/analysis/formal domain on each request.
- [evaluate.rs:14](../../crates/lctx-model/src/domain/native_requests/evaluate.rs#L14)
  replaces refused contexts with EntryValueUnknown; service lines 988–994 repeat that
  generic cause in section availability.
- [responses.rs:38](../../crates/lctx-model/src/domain/serving/responses.rs#L38)
  already carries the canonical assessment reason without reinterpretation.

## 2. Chosen contracts

### Exact refusal and availability

Retain the context constructor’s `ObligationKind` beside its derived Entry value. The
unexamined assessment consumes that cause rather than inventing one. Keep existing codes:
MissingEvidence=7, DefaultStabilityUnknown=32, IncompatibleContexts=44 and EntryValueUnknown=49.
No renumbering, replacement code or string interpretation is necessary.

Per-path reasons remain authoritative. A Partial native section uses the model-owned neutral
label `native_context_unavailable`, rather than assigning EntryValueUnknown to every cause.
NotRequested keeps its existing precedence and meaning. Available remains the current
success condition; this label change does not promote Unknown to absence or failure.

Preserve Unknown verdict, ExactOutcome::Unknown, Basis::Unexamined, original path/condition,
proof membership, zero admitted scalar assignments and no restricted-result identity.
Admitted contexts retain their current restriction/refutation behavior.

### Prepared public formal domain

Build the member/analysis domain once from borrowed selection data. The opaque result records
unique-owner, no-owner and ambiguous-owner outcomes and the allowed owner/formal pairs.
The route is public member → catalog callable → effective assessment → invocation variant →
slot → declaration/formal link. It is not the set of contexts that happen to pass native admission.

A legitimate defaulted formal remains requestable and yields its precise uncertainty.
Request-time model resolution rejects duplicate or foreign formals and nonunique ownership,
validates scalar values and returns the owner plus canonically ordered inputs. The service
preserves its current public Contract-refusal classification for these invalid requests.
Cursor construction uses the same normalized input ordering as the current route.

The prepared domain retains charged nominal indexes, not a second copied catalog. Borrow
the selection owner’s immutable classification rows during construction and return owned
IDs/outcomes. Keep selection/guard ownership in the store wrapper.

### Pure semantic preparation

The conceptual operation is:

```text
prepare_native(owned typed inputs, borrowed selection data, ResourceBudget)
    -> Result<PreparedNativeSemantics, ModelError>
```

Names and module layout may be local choices; ownership and observable distinctions are fixed.
The opaque output owns admitted/refused contexts, checked atoms, original finite paths,
coverage interpretation and the prepared public formal domain. No mutable builder lets the
store establish additional path meaning after preparation.

The operation owns structural occurrence/range correspondence, ambiguity handling, Entry
owner/formal/run selection, context indexing, path attachment, Local/private-continuation
replay, stability/rebase substitutions and Summary/continuation composition. It reuses
existing proof owners; it does not duplicate their algorithms or relax opaque construction.
Private continuation witnesses cite their actual stored premises, never an unstored witness ID.

Declare the current native and Extra inputs at this model boundary, including conditional
binding replay. The model exposes the initial input set and whether decoded initial rows
require the existing binding input/output set. Storage executes that hydration request;
it does not independently infer the semantic necessity of replay. Missing required input
refuses preparation. Relation, epoch and required stream ordering remain distinct.

The PostgreSQL wrapper retains receipt/content verification, shared invariant execution,
epoch-aware snapshot reads, actual-row membership, original guard, resource scopes, CPU
admission, deadline/paging/envelope work and drainage. It receives no new authority from
a locally prepared model value. Read and validate on the original leased connection,
then release the lease mutex before pure work while retaining guard and charged inputs.
Confirm that same guard before publishing the prepared owner; failure publishes nothing.
Use the existing retained CPU-work mechanism, not a detached task or replacement lease.

### Definition and identity

`native_requests::definition()` must capture inventory, formal resolution and all extracted
preparation sources as well as current assessment/proof sources. Today inventory.rs is absent
from its source list ([mod.rs:24](../../crates/lctx-model/src/domain/native_requests/mod.rs#L24)).
Keep native definition identity in cursor policy, and include prepared consumed-input bindings
at their existing identity owners. The global model source capture also rotates after model
edits. These changes deliberately require final-tree reconstruction; no old reader or historical
runtime copy is retained. Stored condition identity stays separate from request restriction.

## 3. Packages and removal obligations

| Package | Prerequisite | Delivered behavior and editing responsibility |
|---|---|---|
| N0 exact refusal | Existing constructors and response codec | Native/model owner changes all three refusal sites, unexamined assessment and neutral section availability; removes generic cause substitution |
| N1 prepared formal domain | N0 semantics fixed | Model owns one domain operation/index; service delegates resolution and removes repeated request-time owner/formal joins |
| N2 pure native composition | N1 and exact epoch/ordering input contract | Model owns every current preparation family and declared inputs; store retains admission/hydration/guard wrapper; removes Extra/context selection and semantic helpers from the store |
| N3 integration and owner clarification | N2 current consumers migrated | Update affected owner prose and controls; no semantic preparation implementation remains in storage, and definition/cursor identity covers new operations |

N0 is independently deliverable. N1 may temporarily coexist with the current path builder;
F02 remains open until N2 migrates every family, including substitutions and continuation DAGs.
N2 consumes the execution companion’s exact-input conventions. A functioning existing loader
can support initial integration; closure-helper availability alone is not native admission.
Coordinate input/epoch helper edits with E1/E2 in that companion.

Root integration owns store/model changes across these slices. Do not parallelize N0/N1/N2
writers on native_service.rs. Pure model fixtures can be prepared in isolation once the contract
is settled. Retirement accompanies each consumer migration, not a final inventory cleanup phase.

## 4. Acceptance and countercases

During implementation use normalized release compile checks and targeted tests only. Relevant
existing controls are lctx-model `native_requests`, lctx `serving_native` and its actual Python
transport fixture; extend these rather than adding a mocked generation provider.

| Control | Independent expectation |
|---|---|
| Required/defaulted twin | Required behavior unchanged; default retains code 32, original proof/path/condition, no assignment and neutral Partial availability |
| Missing/present assessment or slot | Constructor and packet/codec retain code 7 versus admitted twin |
| Crossed/matching variant/context/link | Code 44 without a foreign frame; matching input admits under current rules |
| Same span, different structural path; ambiguous/no read | No fabricated correspondence or stronger conclusion |
| Public formal without admitted context | Valid request domain; explicit uncertainty rather than foreign-formal rejection |
| Duplicate, foreign or nonunique owner inputs | Current invalid-request refusal preserved before assessment |
| Two paths sharing an observation | Distinct original path/restriction identities and correct stored premises |
| Private continuation and shuffled inputs | No invented stored citation; equivalent domains, paths and refusals |
| Construction budget failure | Temporary charges released; no partially prepared owner published |
| Real store tamper/guard cases | Wrong epoch, damaged receipt, absent actual proof row and replacement guard refused; actual work retained through cancellation |

Default-refusal controls currently assert generic code 49; update their source-written
expectations to 32 rather than treating an unchanged test pass as closure. Cover exact reason
through actual transport for legitimate published cases. Some MissingEvidence/IncompatibleContexts
inputs may correctly fail store admission first: prove pure constructor/codec branches and retain
that store refusal, rather than bypassing invariants to manufacture a live native result.

An independent bounded source assessment must show a new native path case can be expressed at
the model operation without interpreting leases or SQL. It must also challenge the defaulted
formal that remains public but inadmissible for scalar assignment. Actual store tests still go
through disposable PostgreSQL 18. Combined gates and final reconstruction follow the coordinator;
no command in this section was run while authoring the plan.

## 5. Alternatives, cost and completion boundary

Moving the whole service would move effects into the semantic owner and is rejected. A generic
preparation framework, mutable store-driven builder or SQL proof interpreter adds no needed
capability. Ordinary functions, typed inputs and opaque checked outputs are sufficient.

The implementation cost includes moving all composition families together, exposing a narrow
borrow of selection data and updating independent refusal expectations. Domain preparation
becomes locally testable and formal resolution reusable; no speed or RSS improvement is claimed.
F01/F02 close only with the relevant independent controls and integrated source assessment,
recorded at the coordinator. They do not close broader parent-plan serving findings or Q0 by
implication. Full FastMCP journeys and activation remain separately stopped.
