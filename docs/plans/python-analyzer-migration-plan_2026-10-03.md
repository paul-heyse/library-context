# Python analyzer migration and populated fact providers

**Proposed, 2026-10-03.** Companion to the [code-facts coordinator](code-facts-expansion-plan_2026-10-03.md),
which owns execution status, findings and acceptance. C1/C7/C8/C9/C12 and the producer portions of
C2–C6 depend on this work. Source contracts were Interface-checked at the review's exact revisions;
no new seam has compiled. Use `python-analyzers`' migration and source index, then
[supply contracts](../design_review/evidence/2026-10-03_code-facts-expanded-target/supply-contracts.md).

## 1. Baseline and target supply

Current Pyrefly 1.3.1 is a minimally patched fork with Ruff 0.0.11. `cpg-flow` links ty/Ruff
0.0.14 and salsa 0.28.2. Canonical syntax comes from Pyrefly's borrowed AST. Dependency policy
assumes one extraction Ruff family and a separate flow-only family. The target changes those
responsibilities deliberately; updating dependency versions alone would neither populate Ruff's
semantic model nor make its AST compatible with Pyrefly's AST.

| Role | Exact target | Source and integration boundary |
|---|---|---|
| Type/resolution | Pyrefly 1.4.0-dev.3 | Upstream `80cec3f57364bc11d4a39a419f6894a8eabcaa00` plus reviewed minimal fork patch; embedded registry Ruff 0.0.14 |
| Canonical syntax/lexical semantics | Ruff 0.16.10 | Upstream tag commit `3265ed1f944c98bb4c04d632fbefb1257cdb583d` plus narrow extraction patch; most Rust crates 0.0.16, **ruff_linter 0.16.10** |
| Flow/index | ty Rust libraries 0.0.16 | Same latest-Ruff fork revision and source identity; salsa family exactly 0.28.5 |
| Development/parity CLI | Ruff 0.16.10; Pyrefly Python distribution 1.4.0.dev3 | Development lint/format and parity only, not production subprocess providers |

ty's unpublished CLI/project/IDE/server at this tag are not a PyPI ty version. Production needs
the Rust core/index line, not installation of a matching hypothetical CLI. Keep current toolchain,
allocative backport, Cargo profiles and environment normalization. No floating latest resolution.

## 2. Source identity and fork ownership

Choose one operator-owned Ruff monorepo fork for latest Ruff **and** ty. Explicit crates.io patches
point every reachable published latest-family crate at the same immutable fork revision, including
ruff_linter's different version. Direct requirements stay exact. The incompatible embedded 0.0.14
registry family is unaffected. Resolve the actual reachable set from Cargo metadata; do not mix
a git `ruff_linter` with registry AST/db crates of the same version, because their types differ.

Family validation must inspect package name, version **and source ID**, plus reachable provider
roles. It allows the two explicitly named families and rejects a third source/version or latest
family leaking into Pyrefly's embedded API. `scripts/check_family.py` currently loses nominal
source identity: repair it, including ambiguous dependency-name lookup. Preserve one Pyrefly
revision and one exact salsa/salsa-macros/salsa-macro-rules trio. Hakari feature unification must
not introduce salsa features into Pyrefly's older Ruff family through a shared bare-name rule.

A published immutable fork revision and checked patch are M1 prerequisites. M0 creates/reviews
those patches; the future commit hash is determined by that work, not invented in this plan.
Record upstream tag, fork commit, patch digest, environment-read classification and patch tests
in pins/check tooling. Retain old upstream/fork refs for reproducible historical checkouts, but
retain no obsolete runtime adapter or fallback authority in the current implementation.

The cost is maintaining two small algorithm-adjacent observational patches. A public upstream
seam can replace either once it offers the same contextual payload and controls. Copying a
Checker/solver or accepting empty public constructors is rejected: both would increase drift
and weaken the first-operation input. M0 supplies compile evidence before dependent packages
rely on these seams; a failed prototype blocks its dependent payload, not an excuse to drop C1/C9.

M0 prototypes live in an isolated task scratch harness against the exact prospective forks;
they do not require an already migrated main-workspace lockfile. Use the repository's pinned
toolchain and keep any experimental artifact directory separate from ordinary cached builds.
This supplies working interfaces for M1 without circular dependence on the production migration.

Decisive pinned contracts: [Ruff traversal](https://github.com/astral-sh/ruff/blob/3265ed1f944c98bb4c04d632fbefb1257cdb583d/crates/ruff_linter/src/checkers/ast/mod.rs),
[Pyrefly overload traces](https://github.com/facebook/pyrefly/blob/80cec3f57364bc11d4a39a419f6894a8eabcaa00/pyrefly/lib/alt/answers.rs),
[Pyrefly exit/divergence solver](https://github.com/facebook/pyrefly/blob/80cec3f57364bc11d4a39a419f6894a8eabcaa00/pyrefly/lib/alt/solve.rs),
and [ty narrowing limits](https://github.com/astral-sh/ruff/blob/3265ed1f944c98bb4c04d632fbefb1257cdb583d/crates/ty_python_core/src/narrowing_constraints.rs).

## 3. Canonical syntax and attachment

Latest Ruff owns canonical source occurrences/Syntax. `cpg-extract/src/syntax_records.rs` and its
typed syntax consumers move to the latest family's AST; Pyrefly extraction modules retain
explicit embedded-family aliases. Never use a borrowed embedded AST as if it were the latest AST.
Existing model node kinds stay append-only and independent of provider enums.

The attachment operation in `lctx-model` receives source snapshot/view identity, exact byte
range, model kind/role and provider origin. It returns unique source attachment, a synthetic
identity, or a located unresolved/ambiguous result. Same-range nested nodes require kind/role
disambiguation. File paths and provider-local integer IDs cannot become persistent identity.
Ranges are bytes; include UTF-8, zero-width/synthetic and latest-syntax controls.

ty's current same-length TYPE_CHECKING projection remains an explicit distinct view with a
verified range map; it is not mislabeled original bytes. Record parse failure or unsupported
provider syntax independently: valid latest Ruff syntax can have unavailable Pyrefly typing.
Provider coverage does not become overall source absence. Sharing Ruff/ty parses is optional
only after lifetime/API evidence; separate parses are a valid target.

## 4. Populated Ruff observation seam

At the pinned tag, `ruff_linter/src/checkers/ast/mod.rs` holds private Checker/check_ast; the
SemanticModel constructor contains no visited facts. The driver traverses body/deferred work,
exports, lambdas, loops, comprehensions, definitions, bindings, unresolved references and scopes.
A final global context flag cannot recover the context at each node.

Add a narrow public `semantic_facts` entrypoint backed by the **existing driver**, leaving Checker
private. Refactor the common runner to accept an optional fallible observation sink; ordinary
lint calls use no sink and retain behavior. Emit node-context observations when references and
context are available, and final owned scope/binding/reference/export projections after deferred
passes. Do not duplicate the traversal or export borrowed SemanticModel/AST into analytics.

Payload meaning includes source/range/kind/role, provider-local scope links, lexical definition
and reference resolution, qualified import name, shadow chain, execution/annotation context,
static branch characterization and unresolved coverage. Later-finalized references join against
the final table; a callback cannot invent resolution before the driver establishes it. Ruff
qualified names characterize lexical imports, not filesystem resolution or instance fields.

Use explicit path/source-kind/Python/platform/typing-modules/custom-builtins settings; record all
fact-affecting inputs in provider context. Disable fixes and ambient config discovery. A minimal
rule configuration must still execute required semantic passes; verify this in M0. Diagnostic
rules are separately selected in P3, not an all-rules lint product. Explicit cancellation/row
budgets propagate through the sink and fail/mark Partial before publishing truncated output as
complete. If a pass must drain after cancellation, charge its work and publish no complete claim.

Replace equivalent spelling recognizers only after alias, rebound-name, deferred-annotation and
static-branch fixtures establish coverage. Distinct source spelling is still source evidence.
If upstream skips a region, characterize that coverage honestly rather than treating no callback
as absence. One syntax authority can retain disagreeing provider observations.

## 5. Native Pyrefly and ty additions

Pyrefly native types must be encoded while their transaction/answer context is live, preserving
structured Overloaded and generic terms. `get_all_overload_trace` returns candidate alternatives
and a closest index even when that candidate was **not chosen**. Only
`get_chosen_overload_trace` supplies chosen evidence. No index-to-applicability shortcut.

Pyrefly `alt/solve.rs::context_value_exit` computes two results: exception arguments and normal
None arguments. The suppression helpers are not complements and are not a sound model policy
by themselves. Choose a narrow explicit native query via the live Answers/solver context that
delegates to this computation with a **located context binding/type**, kind and range, returning
owned normal/error result types, call applicability/diagnostic status and receiver/member basis.
Expose a library observation record, not public solver internals. Preserve gradual/error cases;
do not export a helper bool as exact runtime suppression. Default-range reachability helper calls
are not located evidence. A dedicated trace store may be used only if the native answer lifetime
requires it; keep it query-scoped and bounded, not a global cache.

For terminal statements expose the existing qualified divergence decision with callee type,
declared/inferred return and method/body-kind exclusions, or a narrow observation of those
existing checks. `CallResult=Never` alone is rejected. These queries must not repurpose hover
traces or change inference rules; the solver deliberately uses without_tracing for divergence.
M0 compiles this interface; M3 produces selected located payloads, B3 owns interpretation.

ty reachability and narrowing are different algebras. Preserve typed predicate/alias/place
identities, binding-versus-declaration roles and eager/lazy snapshot timing. Narrowing's graph
formula is `uncertain OR (p AND true) OR (!p AND false)`. Its builder falls back to ALWAYS_TRUE
at resource limits without exposing precision. In the same Ruff fork add a conservative
`precision_lost` flag set only at existing limit fallbacks, carried into NarrowingConstraints,
with a public accessor. Do not change formula, threshold or inference. Mark uses from an affected
scope Partial/over-approximate; terminal true cannot certify exact tautology after loss.

Lazy closure `AssumeBound` is a candidate assumption, not runtime boundness. Persist capture
origin/mutability and selected definition candidates, not a concrete value from names. Current
Flow remains NotRequested in Catalog and requested by the behavioral profile; do not enable
ty inference as a parallel production type authority.

Pyrefly captures/protocol/signature observations needed by Catalog do not depend on requesting
ty Flow. Persist their available characterization in both profiles; ty timing/use-def refinement
is behavioral-profile work. Use Ruff builtin/stdlib tables and contextual typing classifiers
for identity/configuration checks. Its side-effect/truthiness helpers are labelled screening
inputs, not runtime pruning permission. These selected helpers need no extra persistent table
unless their actual consumer requires durable support.

## 6. Work packages and revealing controls

| Package | Implementation slice and retirement | Required local evidence |
|---|---|---|
| M0 | ADR superseding one-parse/embedded ceiling and owner §B1/§B8/§B9 changes; prototype seams and family/source checks | Compile minimal populated Ruff and Pyrefly exit/terminal queries; no-sink lint parity; incompatible source/family rejection; accepted patch scope before M1 |
| M1 | Rebase Pyrefly patch, move exact manifests/lock/uv/policy; update native APIs and tool pins | Touched-crate check/release tests, fork/env/family controls, CLI parity fixtures and native adapter freshness |
| M2 | Canonical latest syntax, populated Ruff rows, source attachment, native signatures/metadata/import/member/diagnostic producer fields | Aliased/local decorators, TYPE_CHECKING views, deferred annotations, unsupported parser, UTF-8 and ambiguous/synthetic locations; normalization/product first consumers in linked packages |
| M3 | Additional ty predicates/narrowing/capture timing and selected Pyrefly exit/terminal/protocol payloads | Known graph formula, precision fallback, lazy unbound capture, normal/error exit distinction, Never receiver versus declared divergent callee; B1/B2/B3 first-use tests |

M1 includes known API migrations: Transaction bindings now come through `get_answers(...).bindings()`;
ModuleAnswersContext uses a bindings method; upstream annotation access replaces the obsolete fork
patch. Encode Overloaded alternatives and qualified opaque NamedInts; remove CallableResidual
matching. Embedded compare/dict-comprehension/argument container changes and latest Identifier
changes stay in their respective adapters. Adopt the required thiserror 2.0.21, remove the obsolete
LSP git exception only after its last consumer is gone, and make directed lock updates. Do not
use a broad floating Cargo update or remove allocative/toolchain/profile policy.

The known rebased Pyrefly patch is smaller than the old patch because annotation access is public;
new observation seams must be explicitly reviewed in addition to that historical patch receipt.
Classify new build/test environment reads, including THIRD_PARTY_STUBS and POLARS_TEST_PATH,
with runtime config/PYREFLY/PYSA reads. Hermetic provider configuration remains required.

M1 semantic controls include Unknown/None simplification, enum literal values, Any reassignment,
overloaded target uncertainty, iterator origins and parenthesized ranges. ty import bindings and
dict-unpack definitions change candidate shapes; test ImportFromSubmodule fallback attachment.
These are expected semantic deltas to inspect, not reasons to force old oracle output.

After migration remove obsolete adapter API branches, old current-tree dependency exceptions and
equivalent syntax/recognition paths. Document current role coverage in extraction/pins/AGENTS
and fork checks; retain no runtime old-pin fallback. Library sources remain in their pinned
acquired environments. Borrowed provider values end with transaction/salsa/traversal lifetime;
owned bounded rows cross into the model/store.

## 7. Integration and acceptance limits

Model declarations, dependency context, coverage, native preparation, stage inputs and PG
publication move together. Provider diagnostics have original source/context, not unconditional
behavioral authority. M2/M3 are not complete until their first normalized/behavior/product
consumers in the coordinator run with actual payloads; mock constructors do not qualify a seam.

Run targeted release-profile checks/tests during packages. Full `just test-all`/`just hygiene`
wait for Q0 across the series; no real-library run here. Source-inspected migration probes from
the skill are historical library evidence, not a passing current-tree migration. All local
acceptance above is **not_run** at authoring. Independent extraction performance and memory
benefits remain unmeasured; measure only with a separately authorized representative library.
