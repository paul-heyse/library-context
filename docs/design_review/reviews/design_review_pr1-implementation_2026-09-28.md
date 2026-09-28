# PR1 catalog implementation review

## 1. Scope, outcome and coverage

**Decision: Revise. Interface-checked, 2026-09-28.** This is a bounded
change/conformance review of PR1 against ADR-0072 and
[product §14.4 and §14.13](../../design/sections/api-and-evidence-product.md).
It applies core 3.0, code-intelligence profile 1.1 and the repository binding.
The baseline is `a6c9fcfe53d28b8a41a7d85188bffff9ea5a5188` plus the concurrent,
uncommitted PR0–PR1 work. Source findings below describe the inspected defects;
the implementation agent began repairs during this review. This document is
evidence, not the current disposition owner.

Reviewed: catalog compilation and source contracts, public candidates and
preferred paths, constructor associations, canonical validation, bundle capability
selection, PG projection/import contracts, operation hydration, native loading and
MCP refusal. Excluded: the PR0 runner and all answer banks; PR2 normalization;
PR3 scenario/deployment completeness; PR4–PR6 retrieval and comparison; broader
behavioral soundness. No production file was changed by this reviewer. No build,
test, live database operation or integrated gate was run.

Expected changes examined were a new `.pyi` or shadowed class alternative, an
inherited constructor, a catalog compilation with a source corpus, and retaining
the existing singleton query when optional behavior is selected.

## 2. Owners, contracts and composition

**Implemented, inspected 2026-09-28:** the principal responsibility split is
appropriate for this slice.

| Owner | Contract and consumers | Expected change |
|---|---|---|
| [cpg-schema catalog](../../../crates/cpg-schema/src/catalog.rs) | Exposure, binding, signature, ordered formal, constructor, type and evidence relations; profile/capability vocabulary | New catalog semantic category changes an explicit relation, then serving schemas derive mechanically |
| [cpg-core catalog](../../../crates/cpg-core/src/catalog.rs) | Construct catalog contracts from canonical facts; expose retained operation facets and document units | New declaration/provider shapes change the domain transform, not PG decoding |
| [attempt](../../../crates/cpg-core/src/attempt.rs) | Mandatory catalog plus selected enrichment, one canonical publication boundary | Catalog or behavioral profile composes the same catalog owner |
| [serving projection](../../../crates/cpg-schema/src/serving_projection.rs) and [PG projection](../../../crates/lctx-postgres/src/projection.rs) | Versioned schema, finite vocabulary, references, receipts and generated additive DDL | Physical projection changes stay rebuildable from canonical rows |
| [PG hydration](../../../crates/lctx-postgres/src/hydration.rs) | Bounded record assembly for one pinned generation, without facet-string signature inference | Another packet consumer consumes catalog records and constructor links |
| [generation](../../../python/lctx_mcp/src/lctx_mcp/generation.py) and [server](../../../python/lctx_mcp/src/lctx_mcp/server.py) | Native lifetime only when selected; explicit unavailable response and packet refusal | Optional capabilities change startup/tool admission without changing exposure IDs |

| Fact family | Fidelity and identity | Coverage and consumers |
|---|---|---|
| Public member and binding | Derived exposure identity; declarations and provider observations retain separate binding roles | Preferred operation IDs remain declaration IDs. Candidate enumeration remains the F02 concern |
| Signatures and parameters | Source and provider fields retained separately; source fact IDs and signature ordinals survive PG/MCP | Missing provider evidence remains a reason; symbolic forms are not treated as complete parameter lists |
| Constructor association | Class/signature relation, including source-less provider observations | Own generated constructors have an implementation route; inherited and alternate-class closure needs F02 evidence |
| Types and declaration evidence | Structural terms and reachable arguments; pinned source digest, span and bytes | Hydration follows typed subject/term references within the selected generation |
| Optional behavior and briefs | Capability selection is explicit, independently of row count | Selected native artifact inventory is indivisible; absent selection becomes `not_requested`, not an empty success |

The catalog transform is testable without PostgreSQL or a native semantic
executor. PG hydration legitimately needs its database boundary. The source
reconstruction validator is a useful corruption check, but its use of the same
`contracts()` function cannot independently establish candidate or constructor
completeness. Hand-authored expected shapes remain necessary for F02.

## 6. Correctness and fidelity gates

These are source-inspection judgments, not test outcomes.

| Gate | Verdict | Evidence or remaining boundary |
|---|---|---|
| G1 Authority | pass in the inspected schema/projection boundary | Serving catalog schemas derive from the canonical declaration; DDL vocabulary and references use that authority |
| G2 Semantic fidelity | fail at reviewed checkpoint | F02 loses attributed class member/constructor alternatives; F03 changes an existing supported query |
| G3 Validity | unresolved for assembled PR1 | Canonical source equality, schema/FK/vocabulary validation and manifest receipts exist. Injected failures and the corrected shapes were not run by this reviewer |
| G4 Hidden behavior | fail at initial checkpoint; correction inspected | F01 made a catalog corpus use behavioral producer/model context. The source now preserves `input.profile` |
| G5 Consistency and recovery | unresolved | Capability-conditioned artifact inventory, generation pinning, bounded hydration and explicit packet refusal are implemented. Both-profile import/restore/startup checks remain outside this review's execution |
| G6 Transformation and reuse | fail at reviewed checkpoint | F02 and F03 are observable projection/composition changes, independent of canonical row validity |
| G7 Truthful capability claims | unresolved | Explicit unavailable responses and optional native construction exist. No end-to-end qualification or complete-signature claim is established here |
| G8 Library leverage | pass for the inspected slice | Existing Arrow/DataFusion, SQLx and serialization mechanisms remain in use; no replacement store, provider framework or query interpreter was introduced |
| CI-G1 Fidelity | fail at reviewed checkpoint | F02 candidate loss and F03 unresolved/empty replacement of a retained supported answer |
| CI-G2 Evidence closure | unresolved for assembled PR1 | Source facts, spans and type references have implementation and validation routes; generated/inherited candidate closure still needs the F02 controls |
| CI-G3 Evaluation integrity | n.a. to this bounded review | No PR0 runner, development answers, sealed confirmation material, gold or heldout content was read |

## 7. Findings

Current execution disposition belongs in
[forward-plan §6.2](../../plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition),
under the existing PR1/AP rows. The observations below retain their original IDs;
repairs and verification should be linked there rather than maintaining another
status register.

<a id="F01"></a>

### F01 — The corpus run changed a catalog request into a behavioral producer policy

**Interface-checked, 2026-09-28.** The initial
[extraction `run()`](../../../crates/cpg-extract/src/lib.rs) constructed
`corpus_input` with `CompileProfile::Behavioral`, regardless of the requested
profile. Producer identity and `context_facts()` consume that field, so the
catalog corpus used behavioral model-context inputs and recorded a behavioral
producer policy. The catalog-only explicit not-requested coverage branch in
`run_release()` was also skipped. The corpus's normal family inventory does not
run Flow; the defect is inconsistent policy/provenance and skipped catalog
coverage, not an assertion that corpus Flow normally executes.

Owner: `cpg-extract` profile propagation. Principles: FP-03/05, DP-05/08/18,
CI-10; G4/G6, A2/A3. Correction: preserve the explicit profile through corpus
construction, with no hard-coded optional model selection. **Implemented repair
inspected during this review:** `profile: input.profile`. Closure still needs a
catalog-with-corpus fixture checking both producer policy and coverage, plus the
retained behavioral corpus case. No fresh functional result is asserted here.

<a id="F02"></a>

### F02 — Selecting a preferred class before member construction loses source alternatives

**Interface-checked, 2026-09-28.** The initial
[`contracts()`](../../../crates/cpg-core/src/catalog.rs) enumerated member paths
from winner-filtered [`public_paths`](../../../crates/cpg-schema/src/public.rs),
then expanded only declarations with the chosen declaration's module and qualified
name. `export_candidates` retained alternative top-level `.py`/`.pyi` class nodes,
but did not enumerate their members. A method present only on the nonpreferred
stub class therefore had no member/binding/signature packet. Same-name shadowed
class definitions have the analogous failure. The initial constructor association
also limited own links to class nodes appearing in preferred public paths, so an
alternate class's collected constructor signature could not be hydrated.

Owner: `cpg-core` catalog candidate/constructor construction, with schema-owned
public selection semantics. Principles: FP-02/04/05, DP-02/07/08, CI-02/04;
G2/G6, A2. Correction: enumerate attributed class/member observations before
preferred selection, preserve their roles, and link constructor signatures to all
relevant class candidates. Keep effective interpretation unresolved where the
facts do not establish it; source alternatives must not become fabricated runtime
possibilities. The legacy preferred-path view can remain for behavioral consumers.

During review, the implementation added own-member expansion across raw class
candidates and removed the preferred-class restriction on own constructor links.
Those changes address part of the source trace; they are not functional closure.
The inherited alternative route was still under inspection. Generated
constructors need an explicit control such as `@dataclass Base` with `Child(Base)`:
inheritance cannot depend only on public `Child.__init__` source paths when the
constructor has no source declaration. A Child-only root must retain supporting
owners outside the enumerated public surface.

Closure: hand-authored `.py`/`.pyi` and shadowed-class fixtures covering own and
inherited methods, source and generated constructors, and a narrowed dotted root;
verify exact retained roles and signatures through canonical rows and real PG/MCP.
Do not use source-reconstruction equality alone as the expected result.

<a id="F03"></a>

### F03 — Catalog unresolved-member handling bypasses retained singleton hydration

**Interface-checked, 2026-09-28.** In
[`get_operation()`](../../../crates/lctx-postgres/src/hydration.rs), the new
`member.operation_node_id IS NULL` branch returned an unresolved packet with empty
constructor/fields/fates before calling `resolve_on()`. Public variable
observations now create such members. A module-global singleton spelling that
previously resolved through `singletons.global` to its class is intercepted by
that branch. This contradicts the retained
[`get_operation` contract](../../design/sections/synthesis-and-serving.md) and the
explicit PR1 preservation requirement. The pre-existing singleton route remains
in `resolve_on()`, but is unreachable for this request shape. For a spelling with
no catalog member, the later constructor/parameter assembly also needs a catalog
for its resolved class rather than treating `catalog=None` as no contracts.

Owner: `lctx-postgres` operation resolution/hydration. Principles: FP-02/03,
DP-02/08, CI-04; G2/G6, A3. Correction: compose selected singleton resolution with
the exposure packet before choosing unresolved fallback, preserving public member
identity and source provenance while retaining class/constructor/field behavior.
An unselected behavioral capability should continue to report unavailable or
unresolved information truthfully. The implementation agent confirmed the defect
and began this repair during review; no completed functional result was inspected.

Closure: compare a known supported public singleton name with its class path in
the behavioral profile, including constructor parameters, fields and fates; also
exercise a noncatalog singleton spelling and the catalog-only counterpart. Carry
the result through real PG hydration and MCP decoding.

## 8. Library fit and verification boundary

**Interface-checked, 2026-09-28:** domain attribution and source/provider
reconciliation appropriately remain custom code. The existing analyzers already
provide declaration, parameter and ancestry facts; the corrections should compose
those facts rather than add another Python semantic analyzer. Relational
candidate selection can reuse the shared DataFusion declaration ranking, and PG
hydration already fetches relation batches. Neither fix needs a new framework.

**not_run:** `cargo check`, focused Rust/Python tests, `just fmt`, `just test-all`,
`just pilot`, database migration/import/restore, and product evaluation. The review
used `git diff`, `rg`, `sed` and line-numbered source inspection only. Test files
were read to understand their claimed shapes; their existence is not a pass.

## 12. Architectural judgment and decision

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | satisfied for this bounded slice | The observed repairs have coherent catalog/extraction/hydration owners; schema and serving adapters derive the new records without a new subsystem |
| A2 Encode meaning structurally | violated at reviewed checkpoint | Explicit exposure/binding/signature roles are a sound structure, but F02 omitted relevant candidates before that structure was populated; F01 also broke explicit profile propagation |
| A3 Extend through composition | violated at reviewed checkpoint | F03 made the new catalog path replace a retained optional query instead of composing its enrichment; F01 had the analogous profile propagation failure |

**Bounded decision: Revise.** Complete F02/F03 repairs and their independent
controls, and obtain the F01 corpus receipt. The schema-driven catalog,
capability selection and projection boundaries are useful implemented foundations,
but they do not offset the fidelity regressions.

**Enclosing architecture: not assessed for acceptance.** This review does not
certify PR0–PR1 integrated qualification, the full API/evidence product, comparative
value, recovery of both profiles, or general behavioral soundness. A subsequent
check should link the closure evidence from the forward plan and preserve these
dated source findings.
