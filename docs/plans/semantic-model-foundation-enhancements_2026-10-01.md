# Semantic-model foundations: focused enhancement plan

**Execution authorized, 2026-10-01; implementation and acceptance in progress.** Companion to the
[Phase 5 plan](semantic-model-phase5-detailed-plan_2026-10-01.md), which owns the combined serving
target, shared contracts and assembled acceptance. The [cutover plan](semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition)
continues to own cross-phase findings. This document owns only F1–F3 enhancement packages and their
local completion. Their identifiers are package names, not competing review finding IDs.

Plan-authoring baseline: main 16ffb9e4 plus preserved dirty changes, inspected 2026-10-01.
Authoring established no runtime qualification. Execution starts from e6c18fe5 plus the preserved
current tree; §5 owns its current package state and receipts. Earlier qualification retains its original scope. Improvements are designed for existing finite semantics; they do not reopen R4 multi-variant
admission, introduce heap identity, change provider ownership or require another store/framework.

## 1. Why these improvements and when to undertake them

| Package | Observed opportunity | Chosen improvement | Relationship to Phase 5 |
|---|---|---|---|
| F1 — reusable selection preparation | Prepared replays broad production inputs and stores only borrowed Data/Output; classification repeatedly scans domains and memberships | Separate classification input contract, charged indexes, strict replay constructor and repository-admitted production owner | Enabling prerequisite for generation-prepared C0; can begin alongside serving mapping/native contract work |
| F2 — expected-domain routing | Canonical adapters independently identify expected-domain relations | One model-owned handled/skipped admission operation, preserving strict required-input checks | Independent canonical improvement; no automatic Phase 5 activation barrier |
| F3 — consumed-row loading | Adapters reconstruct consumed inventories and repeat streaming/routing while vocabulary prefixes differ | Resolve owned input inventory once; a thin typed streaming helper keyed by relation and epoch | Parallel plumbing/local-reasoning improvement; coordinate shared producer edits with F2 |

These are improvements to authority, composition and extension locality (FP-01–FP-06;
DP-01/03/06/08/10/16/18/20/23). They need not demonstrate faster serving to be worthwhile.
Potential scan/allocation savings remain hypotheses. No generic test fixture framework is justified:
existing pure model controls already isolate finite selection, and native/PG controls intentionally
exercise assembled boundaries.

Keep the typed model, completed-source permits, immutable epochs, shared validators, stored graph
hydration, actual PostgreSQL generation tests and finite classifier algebra. Improve the reusable
seams rather than replacing those foundations.

## 2. F1: classification inputs, preparation and evidence admission

### 2.1 Baseline and interpretation

[selection/build.rs](../../crates/lctx-model/src/domain/selection/build.rs) declares broad Data:
C1 EvidenceData, C1 outputs and finite selection facts. The authoritative
catalog_selection_declaration_closure invariant rebuilds and compares all C2 outputs.
[selection/evaluate.rs](../../crates/lctx-model/src/domain/selection/evaluate.rs) Prepared::new invokes
that replay, but its request methods dereference a smaller set of canonical metadata. It does not
hydrate original source bodies. Prepared currently holds Data/Output references and classify scans
domains, domain-context membership, evidence and closure for each candidate/requirement.

A published canonical generation has matching model/content/physical validation receipts and revoked
writer rights. Its lease checks published state, model/lowering and live shape; it does not rehash
all rows or rerun C2. Neither a caller-provided published flag nor an old serving manifest establishes
that narrower inputs are valid. Accidental or validly encoded content corruption must not silently
change served applicability.

The expected change is repeated requests with different finite predicates over one pinned generation.
Generation admission and lookup preparation should be reusable; requirements, observations, witnesses,
conjunction and eligibility remain request-specific.

### 2.2 Chosen target

Define a model-owned ClassificationData container and inventory for the canonical rows actually
dereferenced by evaluation, alongside all six C2 output relations: Context, Witness, SelectionDomain,
DomainContext, DomainClosure and DomainEvidence. This is a semantic read contract, not the full stage
validation/reference closure.

It includes the needed catalog identity/options/defaults; callable/signature/parameter/field/entity
metadata; type/literal and qualification rows; C1 association/scenario/check/deployment/access data;
package/release and referenced binding metadata. Derive its loader dispatch from its explicit typed
inventory. The inventory must be established by inspecting each supported predicate and helper's
actual dereferences, retaining negative lookups and completeness context; do not guess it solely
from currently positive witnesses or export build::Data unchanged under a new name.

Refactor existing evaluation to consume this view and the unchanged selection algebra. Do not add
another evaluator for the new view. Strict preparation retains broad Data/Output replay, then
projects the narrow view and creates indexes. Standalone/external rows use that strict constructor.

The production repository owns a separate encapsulated admitted selection owner. It requires:
- A live leased canonical Catalog generation with the relevant scoped availability.
- The matching C2 declaration-closure validation receipt and source/model/epoch identity.
- Every consumed canonical relation read through its admitted source and hashed against its stored
  content receipt before projection into classification data.
- Local reference-role/domain-uniqueness checks and the shared schema/codec validations.

Do not introduce a public bypass boolean or expose a forgeable trust token. Keep PostgreSQL receipt
and lease details out of model code. A private repository construction path may compose a model
classifier over the verified inputs; arbitrary callers cannot manufacture the serving capability.
Strict replay is the qualification/diagnostic comparison, not a request-time activity.

Store owned canonical data/output plus charged indexes in an ordinary owner; methods create borrowed
views. Avoid self-referential structs, unsafe lifetime extension or separately persisted classification.
Arc sharing retains the original memory reservation once until the last consumer drops.

Index domain by (member, analysis context, domain kind), and domain membership/evidence/closure by
nominal domain/context identities. Index positions are private local handles, not persistent IDs.
Repeated requests execute the same classifier and contextual conjunction; index creation never
precomputes the interpretation of a future requirement.

Start with complete-generation metadata preparation. Predicate-specific slices are an alternative
only if a named workload exceeds the admitted envelope and a complete closure design is settled.
Do not omit difficult members, unobserved contexts or missing lookups to fit memory.

### 2.3 Alternatives and cost

Keeping Prepared::new once per generation is smaller than indexed preparation, but retains broad
producer inputs and couples query admission to reconstruction. Pure indexing without the inventory
split improves scans while leaving this coupling. SQL predicate pushdown as another classifier would
duplicate applicability and incomplete-absence rules. The chosen split changes the read contract and
lookup mechanism while retaining the owned semantic operations and strict replay.

Costs: explicit consumed-inventory maintenance, additional charged index memory, a repository receipt/
content admission path and refactoring evaluation helpers. It creates no new canonical relation and
does not promise better latency without measurement. A missing relevant relation/receipt is refusal,
not a request-specific fallback to incomplete classification.

### 2.4 Implementation and acceptance

One owner changes selection/evaluate, the new classification inventory/container and their tests.
The repository owner integrates the minimal canonical generation/receipt admission consumer in
this foundation scope; Phase 5 C0 will reuse it for its broader catalog routes. Keep build/replay inputs
and publication checks authoritative; remove only the replaced request dependency on broad Data.

Compile lctx-model and run its existing domain_selection_catalog and domain_selection controls
after implementation, retaining their hand-expected witnesses/outcomes and omitted-output rejection.
Add meaningful checks for shuffled inputs, duplicate domains, unchanged conjunctions, changed/missing
membership/coverage, small-budget refusal, repeated requests and release of retained reservations.

Real PG18 C0 controls must compare the production narrow route with strict broad replay on identical
generations, and reject a validly encoded wrong-but-valid consumed row whose receipt no longer matches,
a missing required row, a foreign prefix and a missing C2 validation receipt. A schema-valid round
trip alone cannot close F1.

Completion is the reusable model operation plus its working admitted production consumer, not just
a new container/type. Source/schema metadata alone establishes no semantic or speed acceptance.

## 3. F2: one owner for expected-domain input routing

### 3.1 Evidence and target operation

[structural.rs](../../crates/cpg-core/src/structural.rs) and
[analytic.rs](../../crates/cpg-core/src/analytic.rs) repeat arrays selecting expected-domain relations;
[synthesis.rs](../../crates/cpg-core/src/synthesis.rs) has another array. Other catalog, selection,
retrieval, embedding, Local, execution, models and Summary adapters maintain expected-input lists or
individual type comparisons.

[analysis/expected.rs](../../crates/lctx-model/src/domain/analysis/expected.rs) already owns
method-specific expected inputs and FrontierIndex's supported relation dispatch.
CoverageAdmission::visit checks the captured typed permit/source and rejects unrelated input.
The duplicated adapter decisions mean adding a needed expected relation can require both its model
owner and each core loader to recognize it.

Add an owner operation accepting a typed ReadPermit and batch and returning Handled or Skipped.
It performs captured-source/permit checks for consumed inputs and routes recognized expected rows
through one authoritative model dispatch. A recognized malformed row is an error, never Skipped.
Keep the strict operation for explicitly required inputs; required method inputs must still be
present, declared and completed. Do not infer completeness from which callbacks happened.

One public membership predicate plus a separate visitor would preserve split interpretation; avoid it.
The operation can be ordinary methods over the existing index. No dynamic registry, plugin interface
or universal visitor abstraction is needed.

### 3.2 Migration and independence

Migrate structural/analytic/synthesis first, then all currently identified catalog, C1/C2, retrieval,
embedding, Local, execution, model and Summary expected-domain routing. Audit consumers with scoped
searches for CoverageAdmission::visit and expected-input comparisons; inspect semantic purpose rather
than deleting every similarly named list.

Feed each admitted input batch to its domain consumer and conditional admission where appropriate.
Remove redundant admission-only passes only when their source and vocabulary epoch are identical.
Facts-prefix and Model/Summary-prefix reads of the same relation are not duplicates.

No model schema migration is intended. This package can proceed while Phase 5 mapping/wire/native
work proceeds; its adapter editing surfaces overlap F3, so one owner should integrate both.
Phase 5 does not acquire an invented dependency on completion of every canonical adapter cleanup.

### 3.3 Cost, alternatives and acceptance

Small owner/API change with moderate consumer migration. Keeping the arrays is simpler locally but
retains independently maintained routing; generating a global row framework would impose substantially
more machinery. The chosen operation centralizes a real semantic membership decision without moving
store effects into the model.

Extend existing analysis_expected controls for unrelated input, recognized malformed input, foreign
captured source, missing required input and profile-specific coverage. Run targeted controls on the
migrated structural/analytic/catalog/Summary consumers, including actual PG18 where their behavior
crosses the generation store. Preserve stored outcome/coverage/evidence expectations.

Closure requires the scoped consumer inventory to show no repeated expected-domain classifiers in
the migrated scope and unchanged observed coverage on positive/withholding fixtures. A helper with
unmigrated production callers is not complete.

## 4. F3: declared consumed inputs and epoch-aware loading

### 4.1 Meaningful distinction and chosen mechanism

[local_semantics.rs](../../crates/cpg-core/src/local_semantics.rs) composes separate entry/theory/field
loading loops. [semantic_summaries.rs](../../crates/cpg-core/src/semantic_summaries.rs) reconstructs
SummaryData::inputs repeatedly while traversing type inventories and adding extras.

[summary_production.rs](../../crates/lctx-model/src/domain/execution/summary_production.rs) demonstrates
why reading each relation once is insufficient: Facts-prefix vocabulary feeds entry/binding consumers,
while Model-prefix vocabulary feeds another inventory. The consumed source key is (relation, epoch),
not just table name. A stage's full validation closure is also larger than the rows its producer
needs decoded into domain data.

Resolve the owned consumed-input inventory once per invocation. Add a thin cpg-core streaming helper
accepting the existing typed permit, AttemptSession/StageSession and an explicit batch consumer.
It handles source-bound registration, stream lifetime and error propagation through existing contracts.
One admitted batch may feed multiple explicitly composed domain consumers. Consumption selection and
prefix remain in the domain operation's inventory.

Retain typed macro dispatch initially. Do not create dynamic record decoding or a universal operation/
fixture framework. Cache only resolved declarations and source identity for the invocation; do not add
a persistent result cache. Reference closure remains a separate stage/admission concern.

### 4.2 Work packages and acceptance

Start with Local and Summary, which provide a useful acceptance challenge. Remove their replaced
stream helpers, repeated consumed-inventory construction and handwritten extras only after those
extras have a named owned input contract. Follow necessary adjacent consumers; do not expand into
every producer merely for uniformity.

Compile cpg-core and run its targeted local_semantics, summary_publication and generation_read controls.
Exercise one vocabulary relation at two epochs, duplicate source registration, undeclared input,
profile-specific omission, callback failure, cancellation/drain and resource refusal. Preserve actual
stored/native Summary replay and independent expected answers.

Costs: careful lifetimes and borrowed callback composition, small shared helper and migration effort.
A table-name-only helper is less code but incorrect for the actual epoch distinction; an all-stage
generic framework is excessive. No schema or semantic change is intended, and no measured throughput
claim follows from removing repeated plumbing.

Complete when Local/Summary consume their authoritative inventories through the helper and the two-epoch
controls pass. Further adapter migration needs its own observed opportunity, not a completion quota.

## 5. Coordination, verification and current state

F1 can proceed independently of F2/F3 except shared model exports; F2/F3 share a canonical adapter
owner to avoid competing edits. The root owns integration and shared manifests. Changes that alter
accepted semantics or ownership route through ADR/DESIGN; behavior-preserving API/helper refinements
inside existing boundaries do not need a separate ADR solely for code movement.

During implementation use compile checks and targeted release-profile tests. Integrated test-all and
hygiene run once all functional scope of the actual execution is complete. If F2/F3 integrate into
Phase 5 before Q0, Q0 qualifies that assembled tree. If executed as a separate concurrent scope,
their integrator owns their completion qualification; neither an isolated test nor Phase 5's earlier
receipt qualifies later changes. End-of-turn automation owns formatting/generators.

No additional gates, progress register, fixture framework or evidence folder is introduced. A static
design assessment can establish the change rationale; runtime acceptance remains planned until run.

**Execution boundary, authorized 2026-10-01:** complete F1–F3 as a separately qualified foundation
scope, including F1's minimal canonical PostgreSQL consumer. Broader Phase 5 mappings, process
runtime, catalog routes, vector preparation, native reconstruction and MCP remain outside it.
One production writer migrates F2, then F3, then F1; the root owns integration and independent review.

**Settled F1 prerequisite:** admission binds generation, frontier/profile and applicable availability,
model/physical identity, the C2 declaration-closure receipt, and every consumed relation/source/epoch
content receipt. Use an owner-free serving-role reader factory over the existing canonical lease
protocol; require no runtime migration credentials. Receipt verification and consumed-row scanning
use the original leased connection. The opaque admitted owner retains that lease and charged data/
indexes; arbitrary rows cannot manufacture the store admission capability. Retain strict broad replay
for external rows and diagnosis. Initial production preparation uses the proposed 128 MiB ceiling
through an explicit injected budget. The remaining ADR-0114 serving proposals are not accepted or
implemented by settling this foundation-only contract.

**Current package state, 2026-10-01:** F2 local implementation and focused acceptance passed;
F3 next, F1 not_run. Scope-end functional/hygiene qualification remains not_run.

### 5.1 F2 focused receipt — 2026-10-01

The model-owned `expected_domain_inputs!` inventory drives both typed decoder dispatch and all twelve
assigned canonical adapter families. `visit_if_expected` checks recognized captured sources before
decoding or mutation, and preserves strict required-input checks. Ordinary structural reads remain
strict; unrelated vocabulary epochs are skipped without establishing evidence or completeness.

- **passed:** `python3 scripts/build_environment.py -- cargo check -p lctx-model -p cpg-core`,
  including the final corrected source.
- **passed:** `NEXTEST_TEST_THREADS=8 INSTA_UPDATE=no python3 scripts/build_environment.py -- cargo
  nextest run --release -p lctx-model -p cpg-core --test analysis_expected --test structural --test
  analytic --test catalog_core --test catalog_evidence --test catalog_selection --test
  analytic_embedding --test local_semantics --test base_execution --test model_publication --test
  summary_publication --test synthesis_documentary --test synthesis_refutation --test
  retrieval_preparation`, 43 tests across 15 binaries at the earlier implementation tree.
- **failed, corrected:** the subsequent final-cleanup rerun of `analysis_expected`, `catalog_core`,
  `local_semantics`, `structural` and `analytic` passed eight controls and failed nine adapter controls.
  A lower callable-aspects macro retained its query guard into the next read; each typed stream is
  now scoped and released before the next source. The mandatory structural loader's strict read
  was also restored, with filtering confined to admission-only inputs.
- **passed:** `NEXTEST_TEST_THREADS=8 INSTA_UPDATE=no python3 scripts/build_environment.py -- cargo
  nextest run --release -p cpg-core --test catalog_core --test local_semantics --test structural
  --test analytic`, all nine corrected controls (run `b48fa2c5-0140-45a6-9dc3-295773961c87`).

Independent implementation review found no material F2 findings at `b3a511c2` plus the preserved
baseline and F2 delta (assigned-source SHA256
`dc83c33c55489ece948a33849fb114171da1b087e2c571f805a3cc1eb1af630f`). This is bounded source approval
and composite focused acceptance; it does not qualify F1/F3, assembled foundations or Phase 5.
