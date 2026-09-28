# PR1 catalog implementation review

## 1. Scope, outcome and coverage

**Decision: Accept the bounded PR1 implementation and local qualification,
2026-09-28.** Source inspection and focused receipts support closure
of these three findings, including the two hydration gaps identified during
follow-up. Final reinspection found no additional blocking architectural defect
in the mixed-generation transport/current-admission and retained-runtime recovery
paths. Both profiles have real-library compile/import/MCP-smoke and restore
receipts, at the build provenance and sampling scope stated in §4. The complete
code gate, final-runtime import/reconciliation, explicit operator cutover and
populated operator restore passed. PR0 fair-comparison admission, PR2–PR6 and
general semantic completion remain outside this acceptance. This is a bounded
change/conformance review of PR1 against ADR-0072 and
[product §14.4 and §14.13](../../design/sections/api-and-evidence-product.md).
It applies core 3.0, code-intelligence profile 1.1 and the repository binding.
The initial baseline was `a6c9fcfe53d28b8a41a7d85188bffff9ea5a5188` plus concurrent
PR0–PR1 work; final recovery reinspection used
`d9eb2eefc8f32fcfa960f0bb9bde4eb3a86b4dd0` plus its concurrent dirty tree. Source
findings below preserve the original defects and dated follow-up evidence from
the concurrent repairs. This document is
evidence, not the current disposition owner.

Reviewed: catalog compilation and source contracts, public candidates and
preferred paths, constructor associations, canonical validation, bundle capability
selection, PG projection/import contracts, operation hydration, native loading,
MCP refusal, mixed-generation backup/restore, retained-runtime separation,
profile contract parity and the supplied real-library pilot/cutover/restore receipts.
Excluded: the PR0 runner and all answer banks; PR2 normalization;
PR3 scenario/deployment completeness; PR4–PR6 retrieval and comparison; broader
behavioral soundness. No production file was changed by this reviewer. No build,
test, live database operation or integrated gate was run by this reviewer;
the named implementation-agent receipts in §8 were read together with their test
code and the corrected production paths.

Expected changes examined were a new `.pyi` or shadowed class alternative, an
inherited constructor, a catalog compilation with a source corpus, and retaining
the existing singleton query when optional behavior is selected. The final
follow-up also examined restoring a schema009 database containing current
catalog/behavioral generations and retained bundle12 generations, plus rollback
through the separate schema008 baseline.

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
| [import/recovery envelope](../../../crates/lctx-postgres/src/import.rs), [recovery orchestration](../../../scripts/postgres_recovery.py) and [serving probe](../../../scripts/postgres_recovery_probe.py) | Preserve exact retained content; admit only compatible generations to current serving | A retained old-format generation changes transport classification, without introducing a legacy query decoder |

| Fact family | Fidelity and identity | Coverage and consumers |
|---|---|---|
| Public member and binding | Derived exposure identity; declarations and provider observations retain separate binding roles | Preferred operation IDs remain declaration IDs. F02's named own/inherited candidate shapes now have canonical receipts |
| Signatures and parameters | Source and provider fields retained separately; source fact IDs and signature ordinals survive PG/MCP | Missing provider evidence remains a reason; symbolic forms are not treated as complete parameter lists |
| Constructor association | Class/signature relation, including source-less provider observations and attributed ancestry | Private/generated ancestor support survives narrowed roots canonically; the real PG/FastMCP receipt checks constructor-owner evidence in the child packet |
| Types and declaration evidence | Structural terms and reachable arguments; pinned source digest, span and bytes | Hydration follows typed subject/term references within the selected generation |
| Optional behavior and briefs | Capability selection is explicit, independently of row count | Selected native artifact inventory is indivisible; absent selection becomes `not_requested`, not an empty success |

The catalog transform is testable without PostgreSQL or a native semantic
executor. PG hydration legitimately needs its database boundary. The source
reconstruction validator is a useful corruption check, but its use of the same
`contracts()` function cannot independently establish candidate or constructor
completeness. Hand-authored expected shapes remain necessary for F02.

## 3. Recovery composition and final reinspection

**Interface-checked, 2026-09-28:** `recovery_envelope()` checks the SHA-256 of
the exact canonical manifest bytes against generation identity. Current
projection3/bundle13 manifests also pass the strict typed validator and canonical
generation check. Known projection2/bundle12 manifests are retained as transport
content, with a bounded artifact-receipt inventory; unknown format pairs refuse.
This envelope is used by inventory and artifact relocation, not by the current
relational or native reader. `Repository::pin()` retains typed manifest validation,
and the database check still requires the exact embedded migration count,
versions, success flags and checksums. No old runtime is admitted to schema009.

Receipt3/inventory2 restoration verifies dump identity, all restored logical-table
fingerprints, the complete ready-generation/selection inventory, raw manifest
identity and exact artifact closure before changing locators. It then relocates
artifacts for every retained generation, reconciles and serves current generations,
and explicitly reports legacy serving as `not_run / legacy_runtime_required`.
A selected legacy generation refuses usable current recovery; it is neither
silently dropped nor substituted. Restored current selections explicitly use
exact retrieval because physical ANN admission cannot survive restoration.

**Tested, 2026-09-28, implementation-agent receipts inspected:** the disposable
schema008→009 rehearsal preserves two legacy generations and serves one current
catalog and one current behavioral generation. Its actual FastMCP lifecycle/read
probe observes native loading absent/present respectively. The rehearsal separately
asserts selected-legacy refusal and corrupted-legacy-artifact refusal. The probe's
test-only removal of the legacy selection occurs only after observing the refusal;
production recovery does not perform that removal. The separate protected
schema008 backup is restored and both generations served using the retained old
CLI and Python/native packages. These results support the bounded recovery
contract in [§14.13](../../design/sections/api-and-evidence-product.md#section-14-13)
and the [operator runbook](../../postgresql.md#complete-recovery-and-diagnostics).
They do not establish a post-cutover rollback of newer history, host recovery or
operator cutover. The matching commands and receipts are in §8.

The simpler alternative of decoding all retained manifests as current would
reject preserved ready generations during backup. Relaxing runtime schema checks
would instead mix incompatible readers and rows. The chosen bounded transport
envelope and separate retained-runtime restore avoid both without a compatibility
framework or another storage authority (FP-01/03/04, DP-02/05/08; A1–A3, G1/G5/G6).

**Interface-checked final code-gate repairs, 2026-09-28:** the legacy raw-only
compile adapter can supply an empty public scope; `collect_attributes()` now
returns the corresponding empty result before planning callable facet SQL.
Conflicting raw type-term identities still refuse publication with typed
`Invalid / unique:type_terms`, rather than silently choosing a payload. These
inspections reveal no new architectural blocker. The earlier targeted repair
receipt contained three failing schema/identity snapshot migrations; later
qualification evidence is recorded separately in §4. No complete code-gate pass
is inferred from a successful individual repair.

**Interface-checked condition-reference and bundle-verifier follow-up,
2026-09-28:** `summary_flow_steps.condition_id` now admits the declared union of
provider `conditions` and composed `analysis_conditions`. This matches
[`summaries::load_conditions()`](../../../crates/cpg-core/src/summaries.rs) and the
[native IPC reader](../../../python/lctx_semantics/src/ipc_input.rs), which already
hydrate both catalogs; the latter refuses conflicting payloads for one identity.
The shared `summary-flow-step-source-equality` reconstruction remains in force,
and positive native proof steps still require finite diagrams. This correction
does not change `summary_flows.condition_id`'s analysis-only reference or the
published-analysis premise required by `completion_outcomes()`. It repairs proof
citation closure rather than widening positive-summary admission. The physical PG
summary-step table has no condition foreign key, so no DDL alteration follows.
Under the [ADR skill](../../../.claude/skills/adr/SKILL.md), this is a bug fix inside
the existing condition contract, not a new architectural choice requiring an ADR.

`bundle::verify()` also correctly delegates kernel admission to the shared typed
projection manifest. `validate_envelope()` binds outer format, kernel and
capabilities; `generation()` invokes `validate()`, which still requires the
current formats and complete advertised native closure. Removing the earlier
unconditional kernel check allows a catalog generation's intentional null kernel
without weakening selected-native refusal. The catalog regression now calls
`verify()` itself. These two follow-ups were source-inspected only; no passing
test or pilot result is inferred from their implementation.

## 4. Real-library pilot, code gate and operator qualification

**Tested, 2026-09-28, implementation-agent receipts inspected:** the
[pilot probe](../evidence/2026-09-28_catalog/pilot_probe.py) compiled the pinned
FastMCP release/corpus in catalog and behavioral profiles, once with fake
embeddings and once with live vLLM embeddings. Each generation was imported and
explicitly selected in disposable PostgreSQL, then served through a subprocess
stdio FastMCP `Client`. Each smoke checked discovery and three catalog packets;
catalog profiles returned `not_requested` for brief search, while behavioral
profiles also searched and hydrated all 20 produced briefs. Fake passes are
functional controls; only the vLLM legs supply live-embedding evidence. These are
sampled packet/retrieval checks, not every public API's invocation qualification.

The subsequent backup/restore receipt preserves all four current generations
across 78 logical tables, serves each with the correct absent/present native
capability, and restores the selected behavioral-vLLM generation using exact
retrieval. Each restored generation exposes 1,534 eligible operations. The
24.69-second restore includes serving verification and meets the declared
900-second local objective for this run; it is not a general performance claim.
The earlier mixed-format and retained-runtime receipts remain distinct controls.

**Provenance boundary:** these pilots used implementation build7. The later raw
condition/test-link validation repair makes raw validity checks unconditional
across profiles; the proof-condition union reference repair is described above.
Both are admission changes rather than changed fact construction. The
implementation agent supplied a successful build8 CLI/native compilation receipt
and reports atomic replacement of editable native modules, without wheels.
The final runtime subsequently imported and reconciled both live artifacts and
served them during the operator cutover below. The build7 compiles are not
relabelled as build8 compiles.

**Tested profile parity, 2026-09-28:** the retained
[comparison](../evidence/2026-09-28_catalog/compare_profiles.py) compares multisets
for all nine catalog relations across the live profiles. All agree, including
5,097 members, 1,360 signatures, 5,139 parameters and 333 constructor associations.
Member/declaration/signature/type identities, ordinals, kinds, defaults and source
bytes/ranges/digests remain in the comparison. It excludes run-qualified fact
references, binding/evidence IDs derived from those references, and optional brief
status/reason. `FactSink::fact()` includes `run_id`, so the separately retained raw
row comparison's differences are expected; the passing comparison establishes
contract equality, not identical extraction provenance or row order.

**Latest full gate: passed, 2026-09-28.** `just test-all` run6 completed with
exit 0; its retained log is `build/pr1-operator-cutover/gates/test-all.log`.
It passed 446 Rust tests, 184 Python tests, 18 real-PG Rust tests and two PG Python
tests, plus fixture controls and the format/lint/type/rules/ADR/dependency/pin/SQLx
checks. Declared skips remain skips (19 in the primary Rust selection, two in
Python and two in the real-PG Rust selection). Earlier snapshot, receiver/error
assertion and projected-relation-count failures were repaired as contract/test
migrations. The receiver test now explicitly preserves `self` with no fates and
the never-infer-unused note while retaining all four original explicit-input
`abstract_body` assertions. No production behavior was changed to fabricate a
receiver fate or satisfy the old error-string assertion.

**Tested operator cutover and recovery, 2026-09-28:** the final runtime applied
migration009 and passed the strict database check, imported and reconciled both
live-profile artifacts, then explicitly selected behavioral generation
`2081c10e6ef2e803c832b19e04f595a2ff6cf0f26822461ac89a1122b9823ab4`.
Catalog generation `115529089522683876f095aa4b21819938aab138260ecb43fdbcc5f522e5193a`
remains independently ready. Both passed live stdio MCP smoke and artifact
diagnostics; the selected retrieval profile is exact. The receipt identifies
the final CLI by SHA-256, and status reports schema009 current with no artifact
failure. The catalog diagnostic has three required artifacts and the behavioral
diagnostic has 32, consistent with their selected native capabilities.

The post-cutover backup records 78 tables and four ready generations, completing
in 18.14 seconds. Its disposable restore passed in 12.49 seconds including usable
serving: both current profiles served with native absent/present respectively,
the two legacy generations were preserved with `legacy_runtime_required`, and
the exact behavioral selection was restored. The separate schema008/old-runtime
restore remains the rollback receipt; current preservation does not claim to
serve legacy content or reverse newer operator history. These measured local
runs satisfy the bounded PR1 cutover/recovery controls, not a full-product or
high-availability claim.

## 6. Correctness and fidelity gates

These are review judgments informed by source inspection and the separately
attributed focused receipts in §8; they are not integrated test outcomes.

| Gate | Verdict | Evidence or remaining boundary |
|---|---|---|
| G1 Authority | pass in the inspected schema/projection boundary | Serving catalog schemas derive from the canonical declaration; DDL vocabulary and references use that authority |
| G2 Semantic fidelity | pass for the reviewed corrections | Public and member-ID singleton requests preserve exposure identity; a retained private spelling obtains its resolved class's catalog/constructor contracts |
| G3 Validity | pass for bounded PR1 qualification | Canonical source-equality/corruption controls, complete code gate, both-profile final-runtime import/reconciliation and operator restore passed |
| G4 Hidden behavior | pass for F01 | The source preserves `input.profile`; the corpus receipt checks one producer, two runs and explicit not-requested Flow coverage across all source files |
| G5 Consistency and recovery | pass for bounded local qualification | Mixed-format recovery and the populated operator restore preserve legacy generations and serve current profiles; the captured exact selection survives. Selected legacy and corrupted legacy artifacts refuse; separate schema008 restoration exercises its retained runtime |
| G6 Transformation and reuse | pass for the reviewed corrections | F02 constructor-owner evidence and F03 no-member singleton contracts now cross their named real PG serving boundaries |
| G7 Truthful capability claims | pass for bounded PR1 claims; broader product qualification excluded | Real-library profiles expose selected native/brief states through final-runtime MCP and operator restoration. Sampled packets and produced briefs do not establish every API's invocation correctness, fair comparative superiority or PR2–PR6 acceptance |
| G8 Library leverage | pass for the inspected slice | Existing Arrow/DataFusion, SQLx and serialization mechanisms remain in use; no replacement store, provider framework or query interpreter was introduced |
| CI-G1 Fidelity | pass for the reviewed corrections | Canonical F02 alternatives retain roles; F03 composes selected singleton enrichment with public-member or resolved-class catalog context |
| CI-G2 Evidence closure | pass for the reviewed constructor-owner route | Hydration includes signature constructor owners in evidence/type subjects; the child packet's private generated base evidence is asserted through FastMCP |
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
construction, with no hard-coded optional model selection. **Tested repair,
2026-09-28, implementation-agent receipt inspected:** `profile: input.profile`.
The `catalog_profile_applies_to_corpus_and_library` test passed and checks one
producer, two runs, zero flow values and a not-requested Flow coverage row for
every source file. The behavioral input still propagates Behavioral by source
inspection. This is sufficient closure evidence for the reported propagation
defect; it does not require an unrelated behavioral corpus rerun. The producing
command is recorded in §8.

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

**Tested canonical repair, 2026-09-28, implementation-agent receipt inspected:**
the catalog test now checks source/stub own members, a stub-only inherited member,
a shadowed class member, dotted roots and generated constructors inherited from
both public and private bases. The constructor cases use narrowed child roots,
assert an inherited association with `ancestry_fact_id`, and ensure supporting
owners do not leak into the enumerated public surface. Source inspection confirms
candidate expansion traverses reported MROs, source alternatives retain their
roles, and constructor selection retains required ancestor definitions.

**Tested hydration repair, 2026-09-28, implementation-agent receipt inspected:**
the follow-up found that `catalog_record()` omitted signature
`constructor_class_id` from evidence/type subjects. A source-less generated
constructor inherited from a private base could consequently lose that base's
retained evidence in the child packet. The corrected subject closure includes
constructor owners. The catalog test imports the published bundle into real
PostgreSQL and uses a FastMCP `Client` to assert both the inherited association
with `ancestry_fact_id` and source evidence containing `class _PrivateConfig` in
the child packet. Together with the canonical cases above, this supports closure
of F02 at the reviewed candidate/constructor shapes. Source-reconstruction
equality alone is not the independent expected result.

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
unresolved information truthfully.

**Tested public-singleton repair, 2026-09-28, implementation-agent receipt
inspected:** the `resolved_attribute_access` test compiles/publishes its behavioral
fixture, imports it into disposable PostgreSQL, and compares `probe.settings`
with `probe.Settings`. It verifies shared operation identity, the singleton
spelling and `singleton_class` resolution, matching fields and constructor
associations. The helper crosses real SQLx/PyO3 and validates the MCP response
model; it does not call a FastMCP `Client` tool. Source inspection confirms
capability-aware singleton resolution now precedes unresolved fallback and adds
the class to catalog hydration.

**Tested remaining-route repair, 2026-09-28, implementation-agent receipt
inspected:** the follow-up found that a singleton spelling without a catalog
member still obtained no catalog context for its resolved class. The corrected
route resolves once, fetches the class catalog member by its preferred path and
operation ID, and reuses that resolution during assembly. The updated real
PG/PyO3 test checks a public singleton queried by member ID, plus private
`probe._dynamic_settings`, which has no catalog member: its packet retains a
constructor and the `probe.DynamicSettings` class catalog. This preserves the
legacy route without inventing a public exposure for the private spelling.
These receipts support closure of F03 at the reviewed routes. Separately, the
catalog-only FastMCP receipt keeps `catalogpkg.alias` unresolved when the provider
does not establish its binding; assignment syntax alone is not promoted into a
resolved target.

## 8. Library fit and verification boundary

**Interface-checked, 2026-09-28:** domain attribution and source/provider
reconciliation appropriately remain custom code. The existing analyzers already
provide declaration, parameter and ancestry facts; the corrections should compose
those facts rather than add another Python semantic analyzer. Relational
candidate selection can reuse the shared DataFusion declaration ranking, and PG
hydration already fetches relation batches. Neither fix needs a new framework.

**Implementation-agent receipts inspected, 2026-09-28:**

| Command | Outcome and inspected scope | Session receipt |
|---|---|---|
| `LCTX_CATALOG_FIXTURE="$PWD/build/pr1-catalog-fixture" cargo test --release -p cpg-core --test catalog -- --nocapture` | **passed:** one canonical catalog/publication test, named candidate/constructor/root controls and forged-default rejection | `/tmp/lctx-pr1-catalog-test12.log` |
| `cargo test --release -p cpg-core --test syntax catalog_profile_applies_to_corpus_and_library -- --nocapture` | **passed:** one catalog profile propagation test | `/tmp/lctx-pr1-corpus-test.log` |
| `cargo test --release -p cpg-core --test resolved_attribute_access -- --nocapture` | **passed:** one behavioral publication plus real PG/PyO3 hydration regression, including public singleton/model decoding | `/tmp/lctx-pr1-singleton.log` |
| `uv run pytest python/lctx_mcp/tests/test_catalog.py -q` | **passed:** one actual PG import/FastMCP Client test; ordered parameters, raw defaults, explicit unselected capabilities, duplicate-exposure ID ambiguity, unresolved assignment alias, generated/inherited constructors and private constructor-owner evidence | `/tmp/lctx-pr1-pg-catalog5.log` |
| `cargo test --release -p cpg-core --test resolved_attribute_access -- --nocapture` | **passed:** updated behavioral publication and real PG/PyO3 regression, adding singleton member-ID and no-member private-name routes | `/tmp/lctx-pr1-singleton2.log` |
| `uv run python docs/design_review/evidence/2026-09-28_catalog/recovery_probe.py build/pr1-mixed-recovery-v2` | **passed:** disposable schema008→009 migration, four preserved generations, current catalog/behavioral serving, selected-legacy refusal and legacy-artifact corruption refusal | `build/pr1-mixed-recovery-v2/receipt.json` |
| Retained-runtime `restore-drill` command below | **passed:** original schema008 backup, both retained generations served with native analysis | `/tmp/lctx-pr1-legacy-restore.log` |
| `uv run python docs/design_review/evidence/2026-09-28_catalog/pilot_probe.py build/pr1-pilots-v3` | **passed at build7:** catalog/behavioral × fake/live-vLLM compile, import, selection and stdio MCP smoke; subsequent four-generation backup/restore | `build/pr1-pilots-v3/receipt.json`, profile smoke logs and `restore.json` |
| `uv run python docs/design_review/evidence/2026-09-28_catalog/compare_profiles.py build/pr1-pilots-v3` | **passed:** all nine live-profile catalog relation multisets agree under the explicit run-provenance/brief-state exclusions | `build/pr1-pilots-v3/catalog-contract-parity-final.json` |
| `just test-all` | **passed:** complete run6, including 446 Rust, 184 Python, 18 real-PG Rust and two PG Python tests, fixture and named gate checks; declared skips retained | `build/pr1-operator-cutover/gates/test-all.log` |
| Final-runtime operator commands and post-cutover `postgres_backup.py backup` / `restore-drill` | **passed:** schema009 migration/check; both live imports/reconciliation/smokes/diagnostics; explicit behavioral selection; four-generation populated backup/restore | `build/pr1-operator-cutover/receipt.json`, `steps.json`, per-step logs and `restore.json` |

The retained-runtime command supplied by the implementation agent was:

```sh
PYTHONPATH="$PWD/build/postgresql-pr1-baseline-a6c9fcf/python/lctx_mcp/src:$PWD/build/postgresql-pr1-baseline-a6c9fcf/python/lctx_storage/python:$PWD/build/postgresql-pr1-baseline-a6c9fcf/python/lctx_semantics/python" \
  .venv/bin/python build/postgresql-pr1-baseline-a6c9fcf/scripts/postgres_backup.py \
  restore-drill build/postgresql-pr1-baseline-a6c9fcf/pre-cutover.dump
```

The final operator commands used the default protected configuration. The
implementation agent ran inline orchestration rather than a retained script;
these are its standard CLI invocations, with the receipt's concrete artifacts:

```sh
catalog_bundle="$PWD/build/pr1-pilots-v3/catalog-vllm/generations/e31d495216be2091"
behavioral_bundle="$PWD/build/pr1-pilots-v3/behavioral-vllm/generations/a81b1c02655a67b7"
catalog_generation=115529089522683876f095aa4b21819938aab138260ecb43fdbcc5f522e5193a
behavioral_generation=2081c10e6ef2e803c832b19e04f595a2ff6cf0f26822461ac89a1122b9823ab4
target/release/lctx db migrate
target/release/lctx db check
target/release/lctx serving import-bundle --bundle "$catalog_bundle"
target/release/lctx serving reconcile --generation "$catalog_generation"
target/release/lctx serving import-bundle --bundle "$behavioral_bundle"
target/release/lctx serving reconcile --generation "$behavioral_generation"
target/release/lctx serving select --library fastmcp --generation "$behavioral_generation"
uv run python -m lctx_mcp.smoke "$catalog_bundle" --embedder vllm
target/release/lctx serving status --generation "$catalog_generation" --verify-artifacts
uv run python -m lctx_mcp.smoke "$behavioral_bundle" --embedder vllm
target/release/lctx serving status --generation "$behavioral_generation" --verify-artifacts
target/release/lctx db status
uv run python scripts/postgres_backup.py backup build/pr1-operator-cutover/current.dump
uv run python scripts/postgres_backup.py restore-drill build/pr1-operator-cutover/current.dump
```

These logs were supplied by the implementation agent and read by the reviewer;
they were not reviewer executions. Commands and cases are retained here because
the `/tmp` files are session receipts, not durable repository evidence. The
[catalog evidence page and recovery probe](../evidence/2026-09-28_catalog/README.md)
retain the recovery procedure and outcome while protected dumps/runtime assets
remain in ignored operator storage.

**not_run by this reviewer:** builds/tests, `just fmt`, `just test-all`,
`just pilot`, database migration/import/restore, and product evaluation. The actual
catalog PG import receipt exercises the corrected capability-conditioned
projection's successful import path. Corruption qualification is bounded by the
named negative tests and gate, not inferred from that successful import alone.
Earlier failed targeted/full-gate attempts are superseded for current disposition
by run6, while their repair rationale remains above. This review ran no commands
to reproduce or resolve failures and does not turn the inspected pilot scopes
into full product qualification.

## 12. Architectural judgment and decision

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | satisfied for this bounded slice | The repairs have coherent catalog/extraction/hydration owners; recovery extends existing inventory/relocation without a second query or storage subsystem |
| A2 Encode meaning structurally | satisfied for the reviewed corrections and recovery | F01 propagates policy; F02 retains candidate/ancestry/evidence structure; recovery explicitly distinguishes preserved transport from current-runtime admission |
| A3 Extend through composition | satisfied for the reviewed corrections and recovery | Singleton routes compose catalog context with existing resolution; recovery composes immutable content verification, strict current readers and the separately retained old runtime |

**Bounded decision: Accept the PR1 implementation and local qualification.**
The inspected source and named receipts provide adequate
closure evidence for the reported profile propagation, candidate/constructor
retention and singleton hydration defects. No additional blocking architectural
finding emerged from final recovery reinspection. Both-profile real-library
compile/import/MCP-smoke/restore receipts support the assembled paths at the
declared build7 scope; the final runtime's import/reconciliation, live smokes,
operator cutover and populated restore close the remaining admission/deployment
checks. The complete code gate passed. Acceptance remains limited to these
reviewed scenarios. Revisit for an exposed contract-preservation/admission defect,
another retained recovery format, or a newly required API shape beyond this slice.

**Enclosing product: not accepted by this review.** PR0 fair-comparison admission
remains blocked outside this review; no comparative result is inferred. PR2–PR6,
full API/evidence product acceptance and general semantic completion remain open
under their existing owners. The forward plan should link this bounded closure
evidence while preserving these dated source findings.
