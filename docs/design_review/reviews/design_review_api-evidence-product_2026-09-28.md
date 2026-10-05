# API and evidence product target review

Retired source citations below are historical paths within this review's recorded baseline
and dated inspection scope, including any working-tree limitations. They do not point to
replacement owners. Recover committed source through [Git history](../../README.md#historical-recovery);
the findings and their original evidence strength remain unchanged.

## 1. Scope, outcome and coverage

**Date:** 2026-09-28. **Tier / purpose:** design / target. Core standard 3.0,
code-intelligence profile 1.1 and the library-context binding apply. Subject: the supplied
[pathway](../../external-review-pathway-working-product.md),
[R01–R08 recommendations](../../library_context_product_recommendations_739f9df.md), source
baseline `739f9df` and the replacement [target §14](../../design/sections/api-and-evidence-product.md).
The implementation owner and two independent read-only reviewers inspected surface/evidence and
retrieval/product boundaries. No product code changed and no new functional test or comparative
measurement was performed. Source observations below are **Interface-checked**; target benefits
and acceptance protocols remain **Proposed**. Existing implementation receipts retain their dates.

**Conclusion:** adopt the API/evidence product direction with bounded new extraction. Most immediate
value comes from preserving, connecting and exposing existing facts, not completing general Python
semantics. The current implementation needs revision for this product; the replacement design is
accepted at Proposed strength for the named scenarios. An accepted design is neither an implemented
product nor demonstrated superiority over Context7.

Coverage includes acquisition, canonical facts, public identity, signatures/options, decorators,
scenarios, deployment material, evidence closure, optional analyses, projection publication,
PostgreSQL/native/MCP consumers, retrieval and evaluation. General semantic completeness, remote
deployment of library-context and new ranking-engine selection are excluded with task-driven triggers.
The current execution/status owner is [forward-plan §6.2](../../plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition).

## 2. Responsibilities, dependencies and semantic ownership

| Concept / owner | Existing boundary and observed problem | Target responsibility |
|---|---|---|
| Acquisition / `cpg-extract` | Pinned source, environment and Markdown/MDX/example inputs; package metadata is incomplete for deployment answers | Attributed source and declared package requirements; no request-time imports |
| Contracts / `cpg-schema` | Authoritative Arrow facts, types, graphs and validators; several consumer representations lose detail | Add member/variant/option/scenario/unit contracts and one finite view catalog |
| Catalog / `cpg-core` | Public roots and operations depend on optional analysis; usage is seed-oriented | Mandatory catalog over captured facts, with selected analysis enrichment |
| Semantics / analytics and pure native kernel | Bounded claims with five verdicts and existing refusal rules | Preserve admission; one surface-normalization input, no convenience weakening |
| Publication / Delta, bundle and `lctx-postgres` | Canonical receipts and immutable PG generations already exist; all native artifacts are required | One capability-aware manifest and evidence closure through the same transitions |
| Selection / Rust PG repository and native kernel | Facet filters choose established/conditional records, without a joint-support contract | Typed requirements with witnesses, coverage and explicit context quantification |
| Transport / `lctx_storage`, `lctx_mcp` | Explicit async lifetime, lexical fusion and hydration | Coarse requests, bounded packets and optional-capability loading; no new interpreter |
| Evaluation / evaluation tooling | Semantic truth suite and PG qualification do not establish agent task advantage | Independent matched-condition task execution with a separate confirmation set |

The target's [representation flow and owner table](../../design/sections/api-and-evidence-product.md#section-14-3)
preserve dependency direction: evidence → canonical facts → compiled catalog/units → immutable
serving generation → shared Rust selection → MCP rendering. A catalog module is sufficient; a
new framework or editable knowledge store would introduce unjustified authority and lifecycle.

## 3. Contracts, constraints and testing boundaries

Public exposure, declaration and effective signature are distinct identities. Variants preserve
ordered parameters, binding and evidence; a generated constructor or property accessor is not
reconstructed from whichever declaration is easiest to find. Defaults remain original expressions
unless their literal value is established. Configuration domains distinguish declarations, prose
choices and branch-tested literals.

Requirement outcomes preserve per-term basis, variant/exposure/configuration witnesses and
completeness scope. Structural type filters do not imply runtime assignability. Existential
matches across separate overloads cannot establish one joint invocation; absence requires closure
over the requested domain. Any contradicted requirement excludes ordinary discovery, then conflict
and unresolved groups remain distinct. These are selection outcomes, not replacement behavioral
verdicts. The full contract is [§14.7](../../design/sections/api-and-evidence-product.md#section-14-7).

Evidence identity includes original artifact/revision/span. Association method and version alignment
are explicit. Parse/bind/execute checks, example intent and context completeness are independent;
passing an expected-failure test is not a successful-use recipe. Pure contract tests can exercise
these distinctions without PostgreSQL; publication, role, cancellation and recovery controls then
challenge the actual consumer boundary. Those are required future checks, not results of this review.

## 4. Composition and execution

The target composes four paths: a mandatory declaration/catalog path; optional bounded analysis;
original docs/scenario/deployment evidence; and retrieval units compiled from those records.
The first usable vertical slice must reach production PG hydration and MCP, not stop at Arrow tables.

Catalog readiness is independent of brief completeness and selected semantic capabilities.
`not_requested` does not mean empty/false; corrupt advertised artifacts still prevent publication or
startup. Evidence closure admits API/options/scenario roots without fabricating behavioral assertions.
Publication and selection remain separate existing transitions, and old readers retain their pins.

Ranking uses the classified eligible set, preserves the winning unit and contributes once per
family/API before fusion. The ranking witness is not automatically proof of a requirement.
Bounded packets expose original examples, configuration and deployment evidence with expansion links.
Coarse rebuild identities reuse exact captured facts without a new incremental-computation system.

## 5. Change and failure scenarios

| Trigger | Observed propagation or failure risk | Target owner / settling evidence |
|---|---|---|
| Undocumented API with unmodeled wrapper | Analysis-gated roots and brief/native assumptions can prevent independent catalog use | Catalog/manifest: useful source signature with local effective-surface uncertainty; no-brief and corrupt-advertised controls |
| Add a property, synthesized constructor or overload | Facet-name hydration cannot recover the full contract; textual decorator logic competes with strict admission | Schema/surface owner: binding/variant/accessor identities survive Delta → PG → MCP |
| Two config fields initialized from different arguments | Broad attribution can contaminate unrelated option readers | Bounded field links: independently authored two-field/no-leakage fixture, mutation/alias refusal |
| Example requires previous registration and `async with` | Current short pattern extraction rejects enclosing constructs and cannot prove contextual completeness | Scenario owner: original context and intent preserved, no fabricated setup |
| Add docs-only deployment page | Forcing it into a guessed operation or assertion loses provenance and independent retrieval | Acquisition/evidence roots: page remains discoverable without an exact API association |
| Combine options from incompatible overloads/configurations | Independent facet membership may appear to satisfy a single implementation request | Rust classifier: variant/domain witnesses and explicit joint applicability |
| Add a retrieval family or duplicate examples | Repeated view lists and entity-only ranks hide which evidence matched; duplicate voting can bias fusion | Finite view contract plus unit IPC and family normalization; independent duplicate/winner controls |
| Catalog-only process loads, then database is restored | Existing unconditional native loading conflicts with optional semantic capability | Shared capability manifest through validation, startup, diagnostics and restored serving |
| Context7 lacks matching release/source | A source-availability win can be mislabeled structural intelligence | Frozen comparable population; unmatched results are diagnostic, comparative acceptance blocked if unavailable |

## 6. Correctness and fidelity gates

Verdicts concern the **specified target**, not implemented behavior. Operational qualification is
unresolved until PR1–PR6. The baseline gaps are findings below, not waived failures.

| Gate | Target verdict and own basis | Remaining evidence |
|---|---|---|
| G1 Authority | **pass, Proposed:** canonical facts, one catalog projection and shared classification | Executable schema/consumer migration |
| G2 Semantic fidelity | **pass, Proposed:** variants, relationships, evidence intent and local unknowns preserve meaning | Independent boundary/negative fixtures |
| G3 Validity | **pass, Proposed:** mandatory vs optional capability, reference closure, versioned compatibility and explicit refusal | Validator and production round trips |
| G4 Hidden behavior | **pass, Proposed:** no serving import/generation; any runtime observation is opt-in and attributed | Isolated observation controls if activated |
| G5 Consistency and recovery | **pass, Proposed:** reuse ready/selection/pinning and extend one manifest | Both capability profiles through actual restore |
| G6 Transformation and reuse | **pass, Proposed:** source identity survives projections; duplicate classifier/view decisions are replaced | Witness/default/signature round trips and deletion inspection |
| G7 Truthful capability claims | **pass for design labeling; product effectiveness unresolved:** usable and differentiated milestones differ | Frozen independent comparison; parity cannot close objective |
| G8 Library leverage | **pass, Proposed:** existing parsers, engines and DB capabilities reused; no speculative new subsystem | Focused pinned API check for any newly selected format/parser |
| CI-G1–CI-G3 | **pass, Proposed:** attributed observations, typed relationships and qualified coverage/negative claims | Claim/evidence controls through real product tools |

## 7. Findings and applicability

These findings are dated baseline observations. [Forward-plan §6.2](../../plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition)
alone owns current status. `AP/Fxx` identifies this review; R01–R08 remain external recommendation IDs.

| ID | Finding, evidence and consequence | Principles / correction / closure |
|---|---|---|
| <a id="F01"></a> F01 | **Catalog depends on optional semantic machinery.** `attempt.rs:645,1182` emits no public paths/operations without analysis; `generation.py:86` constructs the semantic executor unconditionally; `serving_projection.rs:12–51` requires the native closure. An unresolved/no-brief API is not an independent product path | FP-01/03, A1/A3, G3/G5. PR1 separates mandatory catalog and explicit capabilities; close with no-analysis/no-brief serving plus corrupt-advertised refusal and restore |
| <a id="F02"></a> F02 | **Rich source contracts are flattened or classified twice.** `hydration.rs:239–251` builds parameter names from facets and drops starred names; class handling at `:85` seeks a public `.__init__`. `synth.rs:648–660` uses textual decorator names while `flow_model.rs:309` requires resolved restricted descriptors. `types.rs:1058–1103` records field flags but not default/factory expressions | FP-02/04/05, A1/A2, G2/G6. PR1/PR2 introduce member/variant/options and one surface owner, add bounded missing extraction, replace lossy hydration/classifier. Close with overload, shadowing, property, constructor and field-specific controls |
| <a id="F03"></a> F03 | **Useful evidence is restricted by brief/assertion eligibility.** `usage.rs:1–17,58–86` is seed-oriented and refuses enclosing control constructs; `bundle.rs:99` roots evidence in assertions. `library.rs:33` distribution data lacks full package requirements/extras/entry points. Contextual implementation/deployment evidence cannot be a first-class product independently | FP-01/03/05, A1/A3, G2/G6. PR3 broadens original scenario associations and evidence roots, preserves negative intent and captures bounded metadata. Close through real PG/MCP/recovery, including a non-seed and docs-only result |
| <a id="F04"></a> F04 | **Facet conjunction does not express customized applicability.** `repository.rs:266–408` accepts established/conditional facet records and returns only matched scope; it has no per-term open/conflict/variant/joint contract. A record match cannot promise one usable configuration | FP-02/04/05, A2, G2/G7. PR4 adds the shared Rust classifier, quantified domains and joint context; close with incompatible-overload/condition controls and scoped-absence tests |
| <a id="F05"></a> F05 | **Retrieval loses evidence identity and repeats view policy.** `behavior.rs:1703` excludes class documents; `retrieval.rs:335–381` aggregates to entity scores and encodes no winning unit. View lists span materialization/import/ranking/qualification and fusion uses a fixed-width channel mask | FP-02/04, A1/A2, G6. PR4 finite view catalog and winning-unit IPC; family normalization, class/evidence units and query-spec migration. Close with independent witness, duplicate and existing exact-symbol controls |
| <a id="F06"></a> F06 | **Existing exits do not establish the requested product outcome.** Current tools/semantic suite do not jointly qualify API invocation, setup and customized discovery against Context7. Richer internal facts alone cannot establish a benefit | FP-03/06, A3, G7. PR0 freezes independent baselines; PR5 bounded tools/packets; PR6 usability then matched comparative gain. Close only with task execution and negative/cost evidence, not a design approval |

Source owners:
attempt (`f6562d3^:crates/cpg-core/src/attempt.rs`, recover through Git),
[generation](https://github.com/paul-heyse/library-context/blob/739f9dfbfa2fb3acb52574e8bf894599b5d34af7/python/lctx_mcp/src/lctx_mcp/generation.py),
[projection](https://github.com/paul-heyse/library-context/blob/0bc8ea11171d6fd96827a8e250964d3de5b3b4ce/crates/cpg-schema/src/serving_projection.rs),
[hydration](https://github.com/paul-heyse/library-context/blob/0bc8ea11171d6fd96827a8e250964d3de5b3b4ce/crates/lctx-postgres/src/hydration.rs),
synthesis (`crates/cpg-core/src/synth.rs`),
flow admission (`crates/cpg-core/src/flow_model.rs`),
types (`9efce30:crates/cpg-extract/src/types.rs`, recover through Git),
usage (`crates/cpg-core/src/usage.rs`),
[evidence closure](https://github.com/paul-heyse/library-context/blob/0bc8ea11171d6fd96827a8e250964d3de5b3b4ce/crates/cpg-schema/src/bundle.rs),
[acquisition](../../../crates/cpg-extract/src/library.rs),
[repository](https://github.com/paul-heyse/library-context/blob/0bc8ea11171d6fd96827a8e250964d3de5b3b4ce/crates/lctx-postgres/src/repository.rs),
behavior (`crates/cpg-core/src/behavior.rs`),
[ranking](https://github.com/paul-heyse/library-context/blob/0bc8ea11171d6fd96827a8e250964d3de5b3b4ce/crates/lctx-postgres/src/retrieval.rs).

**Applicability:** FP-01–FP-06 are satisfied by the proposed ownership/contract/composition routes;
the baseline violations above prevent claiming that property of the present product. Applicable
supporting concerns—single authority, data fidelity, uncertainty, pure transforms, bounded effects
and independent verification—are addressed in §§14.3–14.12. No configurable provider framework,
generic query language or proof of arbitrary Python behavior is necessary for these scenarios.

## 8. Library fit and total complexity

Reuse existing Ruff/Pyrefly/ty facts, Arrow/DataFusion/Delta publication, SQLx/pgpq/pgvector serving,
BM25, the embedding cache, the native condition kernel and FastMCP. New catalog rules are domain
logic in focused Rust modules, not justification for a new analyzer or store. No dependency pin
changes are selected by this review.

Context7 MCP documentation research and current primary documentation establish a stronger baseline
than “documentation-only search”: version/language hints, source links and dynamic source snippets
already exist. [Official search contract](https://context7.com/docs/api-reference/search/search-context7-documentation),
[source indexing](https://context7.com/docs/adding-libraries). Thus the proposed differentiator is
structured requirement/evidence composition and task benefit, not source snippets alone.

Conditional library choices and primary references live in [§14.11](../../design/sections/api-and-evidence-product.md#section-14-11).
PostgreSQL trigrams are plausible for measured name-discovery gaps; FTS does not preserve BM25 ranking
by substitution. Optional Python `inspect` observations have import/annotation side effects and do
not prove effective behavior. Neither route is mandatory for the selected first-product scope.

## 9. Alternatives and tradeoffs

| Alternative | Assessment and revisit condition |
|---|---|
| Finish general Stage 3–5 semantics first | More proof coverage, but useful contracts/examples remain gated on unrelated hard problems. Retain as research, activate only a task-relevant relation or exposed-claim repair |
| Unstructured source/docs retrieval only | Simplest useful product and mandatory internal baseline; may match Context7 without differentiation. Prefer it if structure supplies no benefit, while reporting the original objective unmet |
| Catalog plus bounded analysis, chosen | Reuses the strongest existing work; adds real contracts and migration complexity with named consumers. Local uncertainty keeps source facts useful without asserting effective behavior |
| New framework, graph store, solver or generative recipe service | Adds lifecycle/authority before a demonstrated gap. No present consumer warrants adoption |
| Aggressive runtime introspection | Can observe otherwise inaccessible APIs, but dependency execution and environment effects complicate reproducibility. Trigger only for a central API with insufficient static/original evidence |

The chosen design is a substantial product refocus, not a cosmetic packet renderer. Its cost is
explicit: public-member identity, capability-aware publication, richer evidence closure, metadata,
typed classification and retrieval migration. PR1 exposes a useful first slice; general research
and conditional features do not block it.

## 10. Verification and uncertainty

**Interface-checked, 2026-09-28:** inspected source boundaries and current official Context7,
Python 3.14 and PostgreSQL 18 documentation. Independent reviewers challenged the target's
overload quantifiers, structural type semantics, aggregate contradiction precedence and comparative
population. Those refinements are incorporated in §§14.7/14.12.

**not_run:** functional tests, live compilation, PostgreSQL probes, agent-task comparison and
retrieval benchmarks; this task changes design authorities only. Prior PG qualification is
attributed to [its existing evidence](../evidence/2026-09-28_postgresql-operations/README.md), not
rerun or broadened here. Documentation/ADR and unchanged-freeze checks are recorded in STATUS.
Response limits and task thresholds are Proposed engineering choices to freeze before results.

The largest remaining uncertainty is empirical: whether agents use the structured evidence to
complete more tasks than the strongest comparable Context7 route and the same unstructured local
corpus. No source walkthrough resolves that question. Broader library claims also require another
library and repeat confirmation; useful FastMCP pilot deployment does not.

## 11. Authority changes and disposition

[ADR-0071](../../adr/0071-api-evidence-product.md) supersedes ADR-0021, transfers its preserved
scope/truth/freeze clauses and replaces first-product priorities and evaluation restrictions.
DESIGN §§1/13 and affected analytics/synthesis/serving/evaluation owners now point to the replacement
target. Prior frozen semantic exits stay unmet and separate; no score/threshold/digest changes.
The current forward plan owns PR0–PR6 and AP/F01–F06, while retaining explicitly deferred semantic
work and the completed PostgreSQL relationship. PG0–PG17 is reused, not reopened as a parallel plan.

The supplied external inputs remain unchanged. Their dated observations are assessed in
[§14.2](../../design/sections/api-and-evidence-product.md#section-14-2), including superseded
vector/driver recommendations and the already-fixed shared token admission. Review artifacts
remain evidence; §14 and the current plan own the target and scheduling respectively.

## 12. Architectural judgment and decision

| Judgment | Verdict at Proposed target strength | Basis |
|---|---|---|
| A1 Localize change | **satisfied** | New surface/document/requirement concepts have named owners; duplicate view and invocation decisions have deletion obligations |
| A2 Encode meaning structurally | **satisfied** | Member/variant/evidence/unit identity, local coverage, requirement outcomes and joint contexts survive representation boundaries |
| A3 Extend through composition | **satisfied** | Catalog, selected analysis, evidence, classification and packets compose existing publication/serving mechanisms without an additional framework |

**Target decision: Accept scoped, Proposed.** The design covers the named first-product journeys
and includes credible failure/change routes. **Current product: Revise.** AP/F01–F06 remain open;
neither operational qualification nor differentiated value follows from this acceptance. The
enclosing general behavioral architecture remains incomplete within its retained research scope.

Next: evaluation and schema owners execute PR0's task/contract inventory and baseline feasibility,
then the catalog owner delivers PR1 through real PG/MCP. Formatting and integrated product gates
follow completion of the subsequently authorized functional scope, as required by repository policy.
