# Assembled P0–P2 semantic-model exit review

## 1. Scope, outcome and coverage

**Design · target; independent assembled review; 2026-09-30.** Core/template **3.2**, CI
profile **1.3**, and the repository binding selected through
[`standard.toml`](../design_principles/standard.toml). Subject: shared-main dirty tree based on
`9efce30`, after the functional P0–P2 packages. This review uses the `design-review` and
`design-review-code-intelligence` skills. The foundation/lifecycle reviewer inspected the
model, store, driver and removal boundary; an independent producer reviewer inspected native
correspondences and raw-field/answer obligations. Neither reviewer edits production or runs Q.

**Final decision: Accept scoped for the assembled P0–P2 facts exit.** F01 is closed within its
scope after independent source reinspection and the author's focused receipt. The current complete
gate, full catalog/behavioral/repeated behavioral pilots, measurements and refusal controls
**passed**; their raw evidence was independently read on 2026-09-30. Retained resource allowances
and P3 triggers remain explicit; this decision does not broadly close those findings.

Authorities read: architecture map, DESIGN §15 and affected facts/extraction owners,
ADR-0085–0088/0092/0094–0100, cutover plan packages, §4.1.2 raw inventory, §5 retirement inventory,
§6 retained obligations and §8 dispositions. The current standard governs this fresh assessment;
older reviews keep their original standards and finding IDs.

Supported scope: pure typed contracts, generation store, native facts providers, admission,
both facts profiles, explicit CLI publication/inspection, deterministic fresh recomputation and
bounded resource refusal. P3 normalization, P4 analysis/catalog and P5 serving remain dormant;
their mere compilation is not a capability receipt. No served-product quality, complete runtime
semantics, total-RSS limit or failure-injected network recovery is claimed.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Meaning and owned operations | Contract/dependencies | Expected change |
|---|---|---|---|
| `lctx-model::domain` | Nominal identities, observations/support, conditions, named policies, frontier and admission | Pure declarations and invariants; no store/frontend dependency | Domain concept or fidelity policy |
| Provider records/stages | Translate native syntax, symbols, calls, types, flow, documents and deployment observations | Qualified provider/context/run; typed emit/contribute/handoff | Analyzer upgrade or new correspondence |
| Assembly | Sole writer of shared vocabulary and profile-correct unrequested coverage | Contributions merge through declared writers | New vocabulary contributor |
| `lctx-postgres::generations` | Lower model, COPY, leases, state/receipts, validation, publication and recovery | Consumes model metadata and execution proof | Store mechanism or lifecycle contract |
| `cpg-core::facts` | Compose selected providers and permanent generation lifecycle | Exact schedule/frontier before effects; generic stage sink | Compile workflow |
| `lctx` | Acquire pinned input, select supported profile/frontier, open verified roles, report result | Thin invocation of owners; explicit generation selection | Operator interface |
| Historical/dormant modules | Retained service readers and future-layer answer obligations | Named consumers in plan §5–§6; no facts authority | Their P3–P5 migration |

`lctx → cpg-core/cpg-extract → lctx-model`; PostgreSQL effects depend on the model. Extraction
and flow no longer depend on `cpg-schema`. The remaining `decl/id/legacy` machinery in
`lctx-model` is explicitly for dormant schema consumers; no adapter connects it to `domain`.

Consequential distinctions are explicit: source artifact versus use/ownership; declaration
versus provider symbol; module/class/decorator caller versus nominal callable symbol;
alternative/modality versus invocation; attributed observation versus derived conclusion;
partial/unavailable/not-requested/no-scope versus complete; current generation versus historical
operation. Source and support checks authorize transitive native/type references. These
definitions govern construction, admission and stored validation, rather than only naming outputs.

The producer half specifically inspected `call_records` phase/channel/modality translation and
override downgrading; `type_records`' exhaustive native match, opaque residuals and closure
fidelity; ty's subject-specific attachment/condition refusal, nested reaching markers and
structured call paths; parsed/raw/materialized document distinctions; and deployment
environment/interpreter/report attribution. Root-referenced supporting definitions are supplied;
this does not claim transitive complete dependency analysis. Independent native controls retain
async-return, record-field flag and inherited declaration-attachment answers.

The pilot-discovered native signature corrections also preserve evidence. ADR-0095 gives both
positional kinds one validation group while retaining per-slot kind and ordinal; keyword binding
still excludes positional-only slots. ADR-0096 appends `NativeUnavailable` for individually valid
ordered native slots that cannot form a bindable List. Invalid List interpretation alone takes
this fallback; resource/limit/infrastructure errors remain fail-stop. It retains members and
annotations, omits declaration-parameter links, reports root and supporting-input signature
boundaries/Partial coverage, and binding refuses OutsideProviderModel without assignments.
These paths were independently reinspected. Native signature, dictionary-key and Unicode-value
controls are now explicitly registered, bringing the fixture inventory to 50 families.

ADR-0097 owns arbitrary mapping fields separately from callable parameters through nominal
TypedDictFieldList/Field membership. Shared charged validation checks duplicate keys, order,
gaps/missing members and changed payloads; TypeIndex follows every field term. ADR-0098 keeps
unavailable native slot text distinct from bindable callable interpretation; unavailable type
callable closure requires DisplayOnly transitively. The native dictionary fixture actually
exercises arbitrary keys and empty-slot Signature NativeUnavailable; it does not emit the
TypeTerm Callable NativeUnavailable form. That type form has source/model controls, not a native
fixture receipt.

The full pilot then exposed PostgreSQL's inability to store an embedded NUL in a text literal.
ADR-0099's `Utf8Text` privately owns valid Unicode, serializes UTF-8 bytes and rejects invalid
UTF-8 during deserialization. Its semantic Key delegates the original String framing and its
HeapSize charges String capacity. Declared Binary lowering mechanically uses Arrow Binary and
PostgreSQL bytea for literal strings, TypedDict keys, record-field names and optional native
callable/slot/parameter names. Existing shared decoding/content validation applies the UTF-8
check before publication. None/empty/NUL remain distinct and Literal String remains distinct
from Literal Bytes. No store-specific escaping, old reader or semantic ID bridge is introduced.
These scalar, identity, invariant and lowering paths were independently source-reinspected.

ADR-0100 addresses the next pilot-exposed envelope mismatch. Bound-method function, overload
signature and generic parameter/body ports admit exactly Other/Truncated residuals alongside
their existing named forms. Overload/TypeParameter roles, ordering, nonempty sequences and
ordinary wrong-arm checks remain. Native BoundMethod/Overload/Forall translation already
propagates child opacity; shared TypeIndex closure visits every residual and requires matching
provider/context plus DisplayOnly. Its memo includes fidelity, so a prior DisplayOnly check
cannot authorize NativeStructural support. This retains independently known envelope structure
without establishing the unknown child's callable/type-variable meaning. These model and
correspondence paths were independently source-reinspected.

The flow lookup repair partitions imports, identifiers, references and scopes by the exact source
key previously tested in each predicate, bindings by scope and declarations by owner. It preserves
original map/vector iteration and first-match/uniqueness behavior; retained index keys/slots charge
before insertion. Runtime import classification is computed once over all BindingEvents and read
through resolved events, preserving cross-source resolution. This is a provider-owned lookup
refinement, without a new domain definition. The final full-library pilots below provide the
observed capacity envelope for this implementation and corpus.

The subsequent walrus correction distinguishes native evaluation scope from lexical binding owner.
Exact attached target occurrence plus root name selects BindingObservation.scope only when its
candidates agree; conflicting target owners refuse. Read alternatives authorize an owner only if
every candidate is a Binding target with a retained observation and equal scope. Missing,
unresolved/builtin or disagreeing candidates cannot be discarded to manufacture agreement.
Absent evidence uses the existing explicit native-scope fallback. Formal roots still identify the
parameter variable; raw native use/definition evaluation scopes and the shared Bound same-Place
invariant remain unchanged. These routes and charged occurrence partition were independently
source-reinspected. Producer-review [F04](design_review_a13-a16-b2-producers_2026-09-30.md#F04)
owns the original scope correspondence defect and its scoped closure.

## 3. Contracts, constraints and testing boundaries

[`domain/model.rs`](../../../crates/lctx-model/src/domain/model.rs) validates declared names,
keys, nominal references/subtypes, sums, invariants and derivation inputs before producing a
ValidatedModel. Declared record/codebook metadata derives Arrow/DDL representation. Generated
support invariants share the assertion contract and refuse missing support, cross-provider/context
ownership and unauthorized evidence. Append-only codes and model/producer source digests make
contract changes discoverable.

[`domain/admission.rs`](../../../crates/lctx-model/src/domain/admission.rs) owns requested
families/grains, shared analysis roots, exact scope/family/provider coverage and required-family
availability. Preflight rejects above-frontier dependencies, missing requested coverers and
unscheduled assertions. Admission reconciles actual stage outcomes with coverage; its private proof
binds model, profile, schedule, coverage and stored content. Memory and PostgreSQL use this owner.

[`domain/stages.rs`](../../../crates/lctx-model/src/domain/stages.rs) derives order from declared
readers/contributors, enforces one writer, binds one attempt/sink identity and poisons failed or
cancelled writes. Explicit empty outputs participate in completion. Contributions deduplicate equal
identities and refuse conflicting payloads. Handoffs retain the batch's reservation until readers
finish; an unconsumed contribution prevents stage completion.

The permanent store requires typed attempt → seal → validate → publish transitions, matching
planned/written outputs and receipts. Its scoped lowering and validators refuse above-frontier
data. Publication grants and state change atomically; it never selects. A generation lease checks
state/digests/live columns on its original connection and cannot reconnect after transport loss.
Disposable PG controls exercise this boundary; memory fixture equality alone cannot establish it.

The integrated-rule repair was separately reinspected. Model runtime queries use the existing
read-only SQL helper. `InspectionSession::query` retains its plan verification and typed ReadOnly
failure on the same leased session; renaming the method does not weaken admission. The panic
rule permits exactly `std::panic::catch_unwind(AssertUnwindSafe(|| provider.run(&mut context)))`,
the sole production occurrence at the whole-provider boundary. Per-module catches and scoped
thread catches remain forbidden. A caught panic returns Err through `context.close`, drops
provider state before notifying the pump, skips `output.finish` and poisons the attempt through
StageAccess drop. The existing control checks failed restart/finish and reservations zero. The
rule is a syntactic guard, not a semantic proof for arbitrary copies of that expression.

## 4. Composition and execution

| Operation | Inputs/output and owner | Policy versus mechanism | Effects, limits and fidelity |
|---|---|---|---|
| Capture/acquisition | Pinned project/environment → captured source/input records | Acquisition/input contracts | Explicit effects; frozen bytes checked again before generation creation |
| Native production | Captured artifacts → qualified observations/support/vocabulary | Native correspondence in provider; domain validation shared | Bounded typed batches/channel; explicit local boundaries |
| Assembly | Contributions → shared conditions/types/occurrences and unrequested coverage | One writer; no native claim invention | Deterministic merge; catalog Flow has no provider/run |
| Facts admission | Stored facts plus execution receipt → FactsAdmission | One model policy | Exact coverage matrix; partial remains partial |
| Publication | Sealed/validated attempt → PublishedFacts | Generation lifecycle | No selection or historical operations write; uncertainty explicit |
| Inspection | Published generation → bounded typed/DataFusion reads | Same leased generation and declared schema | Read-only; model/physical/frontier checked before scans |

Content hashes exclude telemetry/timestamps/generation IDs. The CLI reports stage elapsed time,
sampled peak RSS and reservation high-water separately from semantic content. `ResourceBudget`
is allocation admission, not an allocator or process-RSS limit. SQLx completed-operation buffer
cleanup is now implemented; within-operation allowances and conservative shared-Arrow charges
remain explicitly bounded limitations described below.

## 5. Change and failure scenarios

| Scenario / change kind | Owned route and propagation | Evidence or limit |
|---|---|---|
| Add a fact family / domain concept | Domain relation/family/grain + native producer mapping; derived lowering and generic runner | Preflight refuses incomplete schedules; no CLI-local completeness classifier |
| Upgrade native analyzer / binding | Native correspondence and provider fingerprint; independent known answers challenge mapping | Fingerprint includes embedded SQL/migrations/models/scripts/specs, excludes pycache |
| Catalog → behavioral / policy | Model profile and declared ty stage; same driver/store | Catalog Flow NotRequested is provider-absent; behavioral coverage remains qualified |
| Memory → PG / mechanism substitution | StageSink boundary unchanged; permanent store adds lifecycle/proofs | Real producer/fixture digest comparisons attributed to author |
| Required provider refuses / execution failure | Fail-stop attempt, acknowledged abort, registry/schema cleanup | Focused real-PG failure control attributed below |
| Publish acknowledgment/abort lost / execution failure | Actual registry remains authority; primary plus secondary diagnostics retain generation/phase | B1/Dc F01 corrected; actual lost-commit injection not established |
| Deployment target grows / resource change | Receipt validation admits its selected source before retaining bytes | F01 correction reinspected; focused denial control returns typed Resource without relabelling receipt |
| Next layer migrates / domain composition | Named dormant consumers/answers in plan §5–§6 before deletion | No compatibility bridge or downstream acceptance inferred |

## 6. Correctness and fidelity gates

| Gate | Final scoped verdict | Evidence and boundary |
|---|---|---|
| G1 Authority | pass | One typed facts owner, one vocabulary writer, one outcome registry; independently inspected |
| G2 Semantic fidelity | pass | Qualified caller/phase/channel/modality and type/flow/document boundaries; independent controls and full pilots do not establish complete native semantics |
| G3 Validity | pass | Shared support/invariant/admission and permanent receipt validation; current complete gate passed |
| G4 Hidden behavior | pass | Explicit acquisition effects, facts-only refusal, no hidden selection/gold input; three published facts generations remain unselected |
| G5 Consistency/recovery | pass within stated limits | F01 closed scoped; completed SQLx cleanup inspected; capture/provider resource refusals leave byte-identical registry and clean store; named allocation/RSS allowances retained |
| G6 Transformation/reuse | pass | No legacy IDs/adapters; canonical content, declared stages and identical full repeated behavioral content |
| G7 Truthful claims | pass | Actual Partial/NotRequested profiles, attributed receipts, sampled RSS versus reservation accounting and downstream exclusions remain explicit |
| G8 Library leverage | pass at inspection strength | Existing Arrow/serde_arrow, pgpq/SQLx/PG, DataFusion pools and BDD kernel reused |
| CI-G1 | pass | Unknown/unrequested/partial native assertions remain attributed facts; independent correspondence controls inspected |
| CI-G2 | n.a. | P3–P5 serving is excluded; raw assertion support remains G3/CI-G1 |
| CI-G3 | pass | Native/compiler inputs exclude gold/held-out/reference skills; independent controls and qualification inputs retain their separate roles |

## 7. Findings and applicability

<a id="F01"></a>
### F01 — Deployment receipt validation retains an uncharged selected target

**Medium; FP-05/06, DP-20 · A2, G5, CI-08.** In
[`deployment.rs`](../../../crates/cpg-extract/src/deployment.rs), `check_receipt` reads the
entire selected source target before checking the fixed policy path. Its caller charges only
receipt bytes × 32; a receipt is at most 64 KiB while the selected artifact can be much larger.
Capture itself streams source bytes, so that receipt charge cannot cover this later allocation.
Even an invalid arbitrary selected target allocates before refusal. The resource budget therefore
cannot refuse the supported operation before the target is retained.

**Correction:** stream the required source hash or reserve the target's allocation before reading.
The fixed policy-target check must remain explicit, and allocation admission must cover invalid
selections too. Keep this at the deployment receipt boundary; a new generic resource framework
is unnecessary.

**Closure:** independent source reinspection plus normal/tiny-headroom target validation control;
failed validation must release reservations and publish nothing. Current disposition belongs in
cutover plan §8. The producer reviewer reported this source defect independently on 2026-09-30.

**Correction reinspected (Implemented/Interface-checked, 2026-09-30):** target bytes × 2 + 4096
are reserved before the read; its allocation remains charged through both hashes. The stage
propagates `ModelError::Resource`/`Limit`/`Infrastructure` as a fail-stop attempt refusal rather than converting it into
a malformed receipt or OutsideProviderModel coverage. Both independent reviewers inspected this
correction. `typed_deployment`'s independent target-reservation denial control runs the real
producer path with other producers funded: it requires Resource with the target owner, no
TaskReport, no malformed-receipt reinterpretation and reservations zero. The normal receipt twin
retains its ordered duplicate tools. The author reports the three-test suite passed on 2026-09-30.
**F01: closed within this scoped defect.**

**Existing findings retain their original IDs.** Foundation-refinements F01–F03 are reinspected
in the [current foundation review](design_review_p2-foundation-reinspection_2026-09-30.md).
[B1/Dc F01](design_review_b1-dc-lifecycle_2026-09-30.md#F01) is corrected: driver cleanup preserves
both causes, safe generation ID, phase and Unconfirmed classification. This does not establish
actual network failure injection.

[Resource F06](design_review_resource-slices_2026-09-29.md#F06) is narrowed after source
correction: `visit_physical` awaits its inner scan so the stream disappears before
`shrink_buffers()` on completed success/error; `transaction_on` retains the commit/rollback result,
then shrinks and returns that result without erasing Commit/Rollback causes. Pinned SQLx 0.9's
shrink reduces excess capacity without discarding unread data. This reduces completed-operation
retention; it does not admit SQLx's within-operation row before allocation, establish empty buffers
after cancellation, or prove total-RSS coordination. These remain declared allowances.

[Resource F07](design_review_resource-slices_2026-09-29.md#F07) remains conservative accounting:
shared Arrow buffers can be charged by multiple holders. It can refuse useful work earlier, but
does not silently release a live charge or establish a one-charge-per-buffer measurement.
`GenerationLease::read`'s bounded cloned-row accumulator was corrected:
it now reserves row slots/heap before cloning and holds that charge through final Batch encoding.
`Natives` now admits persistent roots/artifact paths/modules/resolutions/symbols through its
StateCharge before insertion; this source correction was reinspected. Remaining native adapter
collections, including `SymbolRecords`, remain outside a complete allocation-admission proof,
distinct from native parse/solver heaps. The final measured envelope identifies these allowances
explicitly. No full-memory claim is accepted from a zero-reservation counter or sampled RSS alone.
The plan's input-validation F02 remains narrowed, not broadly closed; resource F08 retains
B-tree inline undercount, post-mutation nested growth and transient adapter calibration. Its
P3 growth beyond the measured envelope triggers sizing correction. F07's P3 trigger is a measured
reader materially constrained by duplicate Arrow charges. These remain owned in plan §8.

FP-01–06 and applicable DP-01–24 are supported within this review scope by the ownership,
contracts and scenarios above. FP-05/DP-20/CI-08 additionally have Tested admission/refusal
controls and the Measured Q envelope below; the external allowances and P3 triggers qualify
their resource claims rather than establishing complete allocation accounting. Graph analytics/heuristic claims and serving projection rules are
n.a. because P3–P5 are excluded, not because those dormant capabilities are accepted.

## 8. Library fit and total complexity

The current machinery has named consumers: generated typed codecs/DDL reduce duplicated
declarations; SQLx/pgpq own transactional I/O; native frontends own semantic observations;
DataFusion supplies compute and memory pooling where invoked; BDD operations retain bounded
condition semantics. The model's neutral ResourceBudget also serves memory production, so it is
not a forwarding wrapper without a consumer. No new pin/library API is certified by this review.
F01 is corrected using the existing local reservation boundary; streaming hashing remains a
simple alternative if future target sizes justify avoiding that buffer.

## 9. Alternatives and tradeoffs

| Alternative | Assessment and revisit |
|---|---|
| Current typed model + single PG store | Coherent current P0–P2 architecture; rebuild from pinned inputs, explicit frontier |
| Old-ID adapters/parallel Delta authority | Rejected accepted-cutover alternative; duplicates identity/publication meaning |
| Separate new operational outcome events | Rejected by ADR-0094 without a retention consumer; creates reconciliation work |
| Generation-only outcomes | Preferred; abort/pre-generation refusal deliberately leaves no durable history |
| Materialize deployment target without charge | F01; receipt size does not bound target size |
| Streaming target hash | Simple local correction; reuses existing captured source, avoids allocation retention |

## 10. Verification and uncertainty

Production inspection is **Implemented/Interface-checked, 2026-09-30**. Reviewer test/format/lint/
integrated gate/pilot commands are **not_run**. Earlier focused receipts remain attributed to the
implementer, as recorded in the source reviews and plan; they are not fresh reviewer executions.

Author-reported commands on 2026-09-30 (Cargo prefix:
`python3 scripts/build_environment.py --`): model `domain_admission/domain_calls/domain_types`
and extraction `producer_fingerprint` **passed** (9/13/12/1); real PG `domain_calls/domain_types`
**passed** (2/1); `cpg-core --test facts_generation` **passed** (2); CLI
`--test compile_facts --test store_cli` **passed** (1/2). The CLI fixture uses explicit no-op
acquisition, so it does not qualify locked-uv acquisition or whole FastMCP compilation. These
receipts followed corrections; initial failures are retained in their source reviews/plan.

The author additionally reports `cargo test --release -p cpg-extract --test typed_deployment
--test typed_symbols -p lctx-postgres --test generation_stages` **passed** (3/4/1) after F01,
Natives and small-read corrections. The source/control reinspection above is independent; its
execution receipt is attributed. SQLx cleanup itself is source-reinspected; the current complete
gate and final store/Q controls below passed after that correction.

ADR-0095/0096 focused receipts are attributed to the author on 2026-09-30: `cargo test --release
-p lctx-model --test domain_calls -p cpg-extract --test typed_symbols` **passed** first 14/5,
then 15/5 after the unavailable native form. The first native fixture placement caused two
failures before relocation/correction. The native positional control checks ordered `self`
PositionalOrKeyword then `__context` PositionalOnly; model controls check positional binding,
backward-group refusal and unavailable duplicate-slot roundtrip/binding refusal. A dedicated
duplicate-native dependency-producer coverage control was not executed; that branch was
source-reinspected; the final pilots below qualify the observed full-library coverage without
turning that unexecuted targeted branch into an independent negative control.

The Unicode correction's author-reported composite receipt on 2026-09-30 is
`python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test unicode_values
-p lctx-postgres --features testing --test unicode_values -p cpg-extract --test typed_types`:
**passed** (1/1/5) after initial compile and ordering-expectation repairs. Independent source
inspection confirms empty/ordinary/NUL/non-ASCII model/Arrow roundtrip, invalid UTF-8 refusal,
unchanged semantic String key and distinction from Bytes; real PG COPY/validate/publish/read/
retire; and actual native NUL literal and record-field key controls. Producer-review
[F02](design_review_a13-a16-b2-producers_2026-09-30.md#F02) owns the original source finding and
its scoped closure. This changes the model schema: all pre-correction full-gate/pilot receipts
are historical progress. Complete qualification below uses the current post-ADR-0100 model
digest rather than those earlier receipts.

The next full catalog attempt **failed** after 515.155 seconds on bound-method callable-shape
validation; the author reports abort cleanup left the registry empty. Producer-review
[F03](design_review_a13-a16-b2-producers_2026-09-30.md#F03) owns that source finding and ADR-0100
correction. The author reports `python3 scripts/build_environment.py -- cargo test --release
-p lctx-model --test domain_types -p lctx-postgres --features testing --test domain_types`
**passed** (15 model tests and one real-PG test) on 2026-09-30. Source inspection confirms the
model's 24 port/residual/fidelity/foreign-context cases and PG's 16 residual envelope cases.
The preceding complete-gate receipt (672 Rust, 219 Python with 216 repeated Python, plus
doctests) predates ADR-0100; it does not qualify the changed model/digest. The final current
gate and pilots below supersede it for this exit assessment.

The flow cost correction followed an interrupted behavioral run (exit 130 after 802.703 seconds),
not a completed throughput result. A later behavioral run **failed** same-Place validation after
439.920 seconds; the archive retains the exact native/read/definition diagnostic and a minimal
comprehension-walrus regression that failed before repair. The author reports
`python3 scripts/build_environment.py -- cargo test --release -p cpg-extract --test typed_flow
--test determinism` **passed** after repair (5 flow/2 determinism, nine scope/member cases;
runtime resolution spans 17 modules). Its [receipt and raw controls](../evidence/2026-09-30_facts-qualification/flow-binding-owner-receipt.json)
were read. The control tests positive owner correspondence; unknown/missing/disagreeing branches
remain source-inspected rather than inferred native negative tests.

**Current complete gate: passed**, attributed to the author on 2026-09-30, with the
[gate receipt](../evidence/2026-09-30_facts-qualification/gate-receipt.json) and
[raw log](../evidence/2026-09-30_facts-qualification/raw/qualification-gate.log) independently read.
`just test-all` reports 674 Rust passed/11 skipped, 219 Python passed/56 skipped, the real-PG
repeat suite 217 tests passed/one test and one binary skipped, plus doctests, formatting, Clippy, Python
checks, rules, agents/ADR checks, dependencies and gold checks. Model digest:
`dad6dc7983136391c358394dfd3befb7a23aaf30e42719b8e0b57f513a6ed0ef`; this gate follows the
source/scope partitions and walrus-owner repair. The PostgreSQL repeat suite includes native
and memory companion tests; 217 is not a count of individual real-PG executions. This is a
composite success after earlier repairs, not an initially clean run. Suspended downstream checks remain outside the gate.

**Final Q: passed**, attributed to the author on 2026-09-30. The [pilot receipt](../evidence/2026-09-30_facts-qualification/pilot-receipt.json),
[measurement receipt](../evidence/2026-09-30_facts-qualification/measurement-receipt.json),
[refusal receipt](../evidence/2026-09-30_facts-qualification/refusal-receipt.json), raw CLI outputs,
generation details, leased artifact queries, refusal diagnostics and before/after inventories were
independently read. These are full FastMCP 4.0.5 locked-closure and declared pinned-corpus facts
runs, including actual acquisition and native providers, not the earlier no-op acquisition fixture.

The commands are `target/release/lctx compile fastmcp --through facts --profile catalog`, then
`--profile behavioral` twice. Store reset/check, generation list and final store check also
**passed**. Each generation is Published at the facts frontier, unselected, with writer ended and
no active readers. All three have the current model digest above and physical digest
`abad55f8cd01ab5a2b534ae7998fa9aab689914d5e5162d022c88a9b31014c82`.
Behavioral producer/schedule digests match across fresh repeated publication. Catalog content is
`3fea0558aa3ccc3895f86d8ea7b86b18a38201c4cfce668138f7f1701a08cb73`;
both behavioral content digests are
`13bb1140f0b3dd35391002b3c2b7e338144932713f5a1c5a3771034ca0fa6a5b`.
Telemetry, generation IDs and elapsed-time differences therefore do not change this observed
recomputed semantic content.

| Full publication | Elapsed seconds | Peak reserved bytes | `/usr/bin/time -v` MaxRSS, converted to bytes | Flow |
|---|---:|---:|---:|---|
| Catalog | 474.3168 | 2,350,303,801 | 4,158,078,976 | NotRequested |
| Behavioral | 628.9559 | 2,693,597,645 | 4,234,682,368 | Partial |
| Behavioral repeat | 669.4755 | 2,693,597,645 | 4,227,559,424 | Partial |

Per-stage elapsed time and `/proc` VmRSS samples at 20 ms are retained separately in the
measurement receipt and raw logs. They are a different observation from time-v MaxRSS, not one
exact unified peak counter. Shared-host timing is capacity evidence for this corpus; it is not a
performance comparison. Leased queries in all three publications report 10,149 captured artifacts,
69,507,733 captured bytes and largest Python source `google/genai/types.py` at 883,926 bytes.
Exports/Syntax/Lexical/Artifacts/Deployment are Complete; Signatures/Calls/Types/Docs remain
Partial. Catalog has no requested Flow provider/run; behavioral Flow remains Partial, rather than
being relabelled Complete after successful publication.

The published leased diagnostic confirms 59 Other (`unnamed_native_overload`) BoundMethod
children and no Truncated children; its
[raw query](../evidence/2026-09-30_facts-qualification/raw/bound-residual-diagnostic.txt) was read.
That observed correspondence does not establish stronger callable meaning.

Both catalog resource controls use the same production CLI with `--memory-bytes` and exit 1
with the expected typed Resource refusal, so their control outcome is **passed**:

- 65,536 bytes refuses captured-input metadata before beginning a generation or provider stage:
  requested 8,332,436 bytes with 0/65,536 reserved.
- 268,435,456 bytes runs acquisition/deployment, then Pyrefly refuses `lexical_scope_supports`
  after 24.748 seconds in that stage: requested 143,360 bytes with 268,367,657/268,435,456
  reserved. Its time-v MaxRSS is 1,360,457,728 bytes; this directly demonstrates that a reservation
  ceiling does not cap process RSS.

The three-published-generation inventory is byte-identical before and after both refusals; store
check reports three generations and zero findings. These full CLI controls do not expose a final
zero-reservation counter. Zero release counters are established only by the focused controls
that actually assert them, not inferred from a clean registry.

**Resource disposition:** completed SQLx buffer retention is reduced per operation and retained
small reads/native maps are admitted. Native parse/solver heaps, allocator retention, transient
adapter collections, within-operation SQLx buffers and the one bounded row materialized before
admission remain external allowances. Shared Arrow aliases retain conservative multiple-holder
charges; B-tree/nested-growth sizing remains calibrated rather than exact. The recorded envelope
supports the scoped exit and specified refusals, not an absolute process-memory guarantee.
Resource F06 is narrowed; F07/F08 and input-validation F02 retain the dispositions/triggers in §7
and plan §8. No broad closure follows from these successful pilots.

Root reports `just docs-check` **passed**. An earlier integrated run reported Rust **passed**
(666 tests) and Python **passed** (219 tests, 56 skipped), then rules **failed** (four failures).
Read-only helper/method and the exact whole-provider panic-rule corrections were independently
source-reinspected; the current complete gate passes those rules. Earlier formatting/Clippy
and test failures are retained; no initially clean full-gate claim is made. Runtime retirement is an attributed inspected
inventory in the [qualification evidence](../evidence/2026-09-30_facts-qualification/README.md):
exact nine obsolete paths, no active semantic/serving readers, 13,093 protected files unchanged in
size/mtime. A read-only reviewer filesystem inspection additionally confirmed all nine inventory
paths absent and all seven protected roots present on 2026-09-30. The size/mtime comparison
remains the author's attributed receipt. Acquisition caches/benchmarks/backups and historical
receipts are retained.

Suspended semantic-soundness, MCP smoke, structured evaluation, SQLx check and dormant
analysis/catalog/serving suites remain **not_run** with their missing P3–P5 prerequisites. The
runtime flow oracle challenges conservative admission/reaching membership, not every condition's
exact semantic truth. Fixture registration/roundtrip does not prove every raw native known answer.

## 11. Authority changes and dispositions

ADR-0092/0094–0100 and §15 carry the changed semantic/lifecycle/native-signature/value owners. The active plan §8 owns
assigned findings; this review is evidence. The binding B2/B3/B7/B12 and vocabulary route
were corrected and independently reinspected to distinguish cut-over P0–P2 from dormant
downstream contracts.
The plan's raw-field and retirement inventories preserve future answers before removing obsolete
extraction/runtime state; retained `cpg-schema` consumers do not remain facts authority. Final
execution metadata in the plan/STATUS, documentation publication and catalog regeneration remain
the implementer's handoff work; they do not change the qualified production digest assessed here.

## 12. Architectural judgment and decision

| Judgment | Final scoped verdict | Reason |
|---|---|---|
| A1 Localize change | satisfied | Domain/provider/store/CLI changes have distinct owners; native upgrades stay at correspondence boundary |
| A2 Encode adequate domain meaning and govern behavior | satisfied within stated limits | Model definitions govern construction, validation, admission and publication; F01 and pilot-exposed correspondence corrections are closed scoped; measured allocation/RSS allowances remain explicit |
| A3 Extend through composition | satisfied | Generic stages/sinks/leases compose profiles and stores without parallel semantic authority; repeated full behavioral content agrees |

**Final: Accept scoped, 2026-09-30.** The initial F01 revision requirement was corrected and
independently reinspected, with attributed focused controls. Current complete-gate and assembled
Q receipts support the P0–P2 facts exit at the Implemented/Interface-checked, Tested and Measured
strengths stated above. No further material blocker was identified within this bounded review.
Resource residuals keep their named owners and P3 triggers; network-loss recovery, complete native
semantics, P3–P5 capability and served-product quality remain outside this acceptance.
