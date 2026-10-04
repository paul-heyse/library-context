# Scoped Python incoming-reference enrichment

**Implemented R1/R2, 2026-10-04; assembled acceptance pending.** Supporting design for O07
in the [coordinator](code-facts-analytical-enrichment-plan_2026-10-04.md); §7 owns current
receipts, dispositions and deferrals. `dbabbe6904a3` is the authoring baseline. The scoped
incoming-reference product and joined-process development oracle are integrated. Focused model
controls and the actual differential oracle passed, including alias-policy, typing-only and
unreachable controls. Q1 remains open; production ty project/IDE search remains Deferred under
the resource-contract trigger below. No new production provider is required to finish this scope.

## 1. Selected result and provider boundary

Add a supported answer to “Where is this declaration referenced outside calls in the captured
source?” Use existing final Ruff lexical references and normalized identity first. Current source-use
packets emphasize calls; final binding/reference facts and normalized ReferenceEntityAssessment/
Candidate and ReferenceBindingCharacterization already supply lower-coupling local name evidence.
A new reference packet consumes those identities rather than parsing spelling again.

Pinned ty `find_references` is useful independent evidence, including native Read/Write/Other and
keyword-label/member cases, but is selected as a **development-only differential oracle** here.
It builds a whole result Vec with parallel semantic inference and exposes no charged internal
work/allocation limit. Result caps/post-hoc charging are not the production attempt contract.
Salsa revision cancellation and retrospective memory dumps do not establish safe interruption;
`system_mut` while snapshots run can deadlock. Do not add ty project/IDE to the production query path
on the premise that input file caps solve those semantics.

Exact APIs were checked in independent Ruff/ty fork
`f7bdff69e1fb94ab0ed5b340e977aac0d26e9301` / parent
`3265ed1f944c98bb4c04d632fbefb1257cdb583d`. `find_references` accepts semantic project Db,
ProgramFile, TextSize and include_declaration, returning `Option<Vec<ReferenceTarget>>`.
The public path uses **ResolveAliases**; preserve-alias search belongs to private rename/highlight
paths. This plan does not advertise configurable preserve mode or add a fork just for that option.
**Source-checked correction, 2026-10-04:** the pinned public search returns `None` for an empty
result as well as unavailable resolution; preserve that ambiguous provider result without
converting it into absence. Its ResolveAliases semantic lookup also filters by the requested
spelling, so differently spelled alias uses require their own query. The development oracle
tests that policy explicitly; the production incoming-reference operation retains its own
available-empty distinction. Native kinds are retained as
reported; augmented assignment does not automatically create invented separate read/write rows.

Production ty reference integration is deferred. Reopen only for a concrete supported-reference
gap plus an API/adapter that precharges inference/search allocations and work, safely cancels/joins
all tasks and satisfies original-source attachment/coverage. A bounded external process resource
limit can protect a test harness, but does not make this API a production charged kernel. No new
production provider or third typing authority is needed for the selected product.

## 2. R1 — Derive and serve existing-fact incoming references

### Request and interpretation

The model operation accepts exact target declaration/member or source parameter identity, captured
input/context and explicit finite search universe. For the initial public surface, use GetOperation's
exact member/PublicPath selector with a new opt-in incoming-reference section; exact formal targets
can be a typed subsection selector tied to that member. Do not widen the endpoint into arbitrary SQL
or accept a displayed name as entity authority. If parameter selection needs request vocabulary,
add the generated typed target arm and its parser/native conformance in the same package.

Start with final Ruff **name references** in the captured universe whose native binding support and
normalized candidate/target chain resolve the requested entity. Exact source declaration/formal
correspondence and import aliases are admitted only where the existing normalizer establishes it.
Read/reference, typing-only/runtime/string/TYPE_CHECKING context and original occurrence stay
separate dimensions. Classify non-call use through canonical syntax roles, retaining an explicit
syntax category and evidence. A local decorator/value reference can be useful without being a call
edge; typing-only references cannot become runtime execution counts.

For each reference retain original artifact/span/view, native lexical assertion/support/run,
normalized assessment/candidate/target and entity/formal correspondence, source-context flags and
role/category. Resolve one supported target or retain candidate/unresolved reason. Shadowed names,
same-spelled declarations and aliases to another target never match by text. Do not claim broad
attribute, keyword-parameter or dynamic/cross-world closure from lexical name evidence.

A typed result consists of canonically ordered admitted references, explicit candidate/unresolved
remainder, captured search-domain identity/coverage and bounds/truncation. The scope must state
which artifacts/contexts were examined and which reference categories can be represented. Available
empty means no supported result in that finite represented search domain; it proves neither unused
API nor absent external/dynamic consumers. Deduplication uses nominal source/reference identity,
not display text, and retains independently attributed support.

### Hydration, resources and delivery

Declare exact final lexical reference/context/support, normalized reference assessment/target/
characterization, source occurrence/role, entity/declaration/formal and captured-module/coverage
inputs in serving mappings and preparation. Reuse normalizer identity interpretation; if a missing
correspondence needs extension, implement and replay it at that owner before the reader consumes it.
A source-reference observation alone is not a declaration edge. Do not create a persisted reference
graph for a request-time product.

Charge search/index/read/result work and retained packet bytes with existing resource/lease
lifetimes. Use canonical finite bounds and page cursors bound to target, captured universe, model
mapping and generation. Hydrate/verify support before paging. The optional reference section and
original evidence expansion use typed model results; Python does no matching or syntax inference.
Programmatic text says “captured static reference” with context, never popularity, call frequency,
completeness of external users or execution.

Acceptance includes a value reference and decorator outside call syntax, exact local parameter,
shadowed same-name binding, import alias where normalization supports it, typing-only/string context,
unresolved binding, duplicate/shuffled rows, omitted captured module and non-name/member/keyword
support limit. Exercise wrong target/context/view, bounded continuation, generation change and
missing support. Compare the initial answer to existing call-only/source packets to establish the
selected product distinction. Native and MCP output must preserve all availability/coverage labels.

## 3. R2 — Hermetic ty differential oracle

Build a test-only harness over tiny captured originals; do not feed TYPE_CHECKING-adjusted runtime
bytes to semantic IDE navigation. Use the same explicit version/platform/dependency/typeshed
premise as the selected fixture. `ProjectDatabase::fallible(ProjectMetadata, in-memory System)`
with `ProjectMetadata::new` disables uv discovery; explicit options/program/search paths must own
all analysis context. `Project::set_included_paths` names exact first-party files and the harness
asserts Project.files equals that set. No OSSystem discovery, interpreter query, open editor state,
ambient repository root, uv sync or network acquisition is part of the oracle run.

Fixtures include source dependencies and typeshed only from named captured inputs. Record the
query file/offset, included-file/dependency identity, alias policy ResolveAliases, include-declaration
choice, provider revision and native outcome. Convert native file/range/kind to test report IDs
using exact original source attachment; do not serialize Salsa handles or make a report a canonical
production fact. Dependencies/context files searched outside the nominal included set must be
recorded as a changed universe or the case refused; a project file list alone is not whole search
coverage. Fixed small cases and an external harness timeout bound test execution; kill/join on
failure and report failed/not_run accurately.

Compare overlapping **meanings**, not whole result equality: name references with admitted target
identity can agree with R1; keyword labels, property/member reads and unrepresented categories
may expose a known scope difference. Classify each discrepancy as identity/attachment defect,
provider-policy difference, omitted module/context, unsupported category or unresolved evidence.
Do not replace Pyrefly typing/member/MRO or Ruff lexical resolution simply because ty returns more
rows. An actual interpretation error discovered by the oracle becomes a scoped repair at its
current owner, with a revealing regression case.

Controls: shadowing, resolved alias policy, explicit declaration inclusion, keyword argument label,
decorator/property/member read, augmented assignment, typing-only/unreachable use, omitted module,
empty available result versus None, foreign source view and deterministic reordered files. If a
selected property-read discrepancy needs corroboration, inspect Pyrefly's getter trace as a separate
oracle: descriptor identity, getter origin and returned access type are different observations.
Likewise ty signature-help arity/error-recovered hints cannot become successful applicability proof.
No blanket audit of those competing APIs is required.

Use exact nominal ty_ide/ty_project/semantic family pins as **dev dependencies** of the existing
extractor test harness (or a narrowly test-only workspace package if required by crate layering).
Production dependency closure must remain unchanged. Root integrates workspace pins/lock/policy;
verify the one latest-family/salsa identity and document the oracle-only purpose in pins/owner
guidance. Do not introduce a production cpg-references adapter until the explicit resource trigger
is satisfied under separate authority. Keep test reports with their actual scope; no new register
or mandatory evidence folder is required.

## 4. Completion and remaining limits

R1 and R2 can proceed independently once exact identity/search-domain contracts are understood.
R1 is a required product with scoped lexical coverage; R2 provides independent checks and names
remaining provider differences. Development oracle completion does not qualify production ty
search, global member/reference completeness, retrieval quality or runtime behavior.

Migrate typed sections/targets, serving mapping identity, snapshots and stale adapter rejection
with R1. Delete packet-local spelling matches and speculative duplicate reference stores. Preserve
current call-use packets and category limits because they remain valid consumers. Enduring scope
belongs in acquisition/extraction, API product and serving owners, with accurate evidence labels.
Targeted release compilation/model/extractor/service tests happen while implementing; coordinator
Q1 owns the final disposable PostgreSQL/MCP journey and same-tree `just test-all`/`just hygiene`.
No real-library qualification or operator activation is authorized by this supporting plan.
