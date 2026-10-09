# Integrated persisted execution corrections

**Design / target review · 2026-10-08 · Core/template 3.3, heuristics 1.0, code-intelligence profile 1.5.**

The assembled correction has a coherent architectural route: producer declarations govern captured
admission; model-owned completion distinguishes failed work from uncertain cleanup and committed
effects; native access separates compact candidate ordering from exact membership and rich
hydration; restore transport preserves whole import units in bounded requests. These are
substantive improvements to semantic ownership and execution fit, supported here at
**Implemented / Interface-checked** strength.

The initial source examination found three material integration defects. Cold verification
looked up graph-backed relations in a compiler-only registry (F01); published member selection
retained an uncharged rich list after its bounded sorter (F02); serving companion hydration read
retired graph scalar fields (F03). Corrections for all three have now been independently inspected.
The member collection carries its retained-state reservation through serving consumers, and
classification owns its simultaneous roots and membership set. A focused follow-up exposed a
fourth defect: published members discarded their selected physical pointers and rehydrated nominal
key windows through repeated family-prefix scans (F04). Its minimal physical-pointer correction
has now also been independently inspected. The bounded decision is therefore **Accept the examined
architecture at static Implemented / Interface-checked strength**. The findings preserve their
original diagnoses and meaningful execution obligations.
Native execution and PC6 acceptance remain separate, unfinished evidence obligations.

## 1. Scope, baseline and coverage

The subject is the dirty PC1–PC5 implementation on `main` above documentation decision baseline
`607fef2eb1850eef94e21edcffca43289f15f242`, examined on 2026-10-08. PC0's accepted target is
ADR-0135 (now consolidated by [ADR-0138](../../adr/0138-operation-shaped-native-finalization.md)), described by the
[correction plan](../../plans/persisted-execution-corrections-plan_2026-10-07.md) and the semantic,
extraction and storage owners. The
[correction-causes review](design_review_persisted-execution-correction-causes_2026-10-07.md)
provides original counterexamples and preservation constraints; its F01–F06 are distinct from
this review's new F01–F04.

The functional target is independent typed provider contributions, exact immutable completed
views, necessary semantic admission without producer replay, direct publication, pinned native
serving, and explicit backup/import with independent cold admission. Both catalog and behavioral
profiles are affected. The review covers provider declarations and captured bindings, completion
and workflow siblings, prepared results, table-specific scope realization, compiler and published
selection, DataFusion fetch, blocking sort ownership, and restore request composition. It follows
adjacent serving consumers where the schema cutover changes their behavior.

The workload premise is a personal operator compiling growing pinned libraries, with many
matching members, overlapping scopes and graph neighborhoods, while retaining exact answers
under explicit memory limits. Complete classification may require a complete semantic domain.
Small requested outputs do not imply small examined inputs. Restore may contain many small
statements, large whole units, and a failure partway through a request. Cancellation may interrupt
stream drainage or acknowledged blocking work. These premises make retained state and lifetime
ownership consequential without requiring a speed benchmark.

The method is independent static inspection of source, types, declarations, owners, and resolved
library source. No build, test, probe or operator action was performed by this reviewer. Coordinator
test reports are attributed in §10; they are not fresh reviewer receipts. Review acceptance is not
PC6 qualification or authorization for real-library or operator adoption.

## 2. Responsibilities and semantic authority

| Owner | Responsibility and consumer contract | Reason for change |
|---|---|---|
| `lctx-model::domain::producer_contract` | Supported semantic producer contract, executable binding, captured supplier roles/settings/source association | New supplier role, semantic contract or vocabulary |
| `cpg-extract::contracts` | Compose declarations with executable providers without redefining admission | Provider implementation or supported composition |
| `cpg-core::facts` and `Workspace` | Admit exact completed owners once and expose admitted availability/reporting | Changed premises or supported profile |
| `lctx-model::domain::completion` | Primary and secondary failures, local terminality, remote certainty, resource disposition and committed effects | A genuinely new outcome distinction |
| `lctx-surrealdb::{prepared,schema,reconcile}` | Native result positions/finality and mechanical table-specific lowering from semantic declarations | Physical schema or transport mechanism |
| Native compiler and ordering adapters | Exact contributor membership, nominal/physical identity, bounded ordering and hydration | Access path or bounded transfer implementation |
| Publisher and restore importer | Private ownership, seal/publication, checked sequential requests and abandonment | Publication or import mechanism |
| Serving selection and scope | Complete finite member/classification domain and owned semantic closure | Supported operation or semantic ownership closure |

The producer model distinguishes a supported contract from the current executable build and
from a captured observation. `CapturedProducerBinding` records contract/revision, settings,
supplier roles and source bindings. A supplier carries its tool and revision; a source binding
retains input, context and configuration. This is sufficient to express multiple suppliers and
captured-build admission without making today's provider instance the historical authority.

`ProducerContract::validate_binding` enforces exact declared roles, semantic contract/revision,
fixed settings and supplier identity. `recorded_coverage` selects supported declarations
independently of observed outcomes and validates completed inventory, exact inputs, captured
source/configuration/run/family identity and profile. Extra suppliers reject. NotRequested is a
declarative supported state, rather than a synthetic failed execution. The operation consumes
recorded premises and produces one `AdmittedFacts`; manifest reporting consumes that result.

The nine exact facts-view identities participate in Workspace availability reuse. This preserves
the useful existing admission foundation while removing another reporting classifier. Header
checks establish explicit new state/content/contribution/artifact versions before descriptor
interpretation; no old-format reader is part of this inspected cutover.

Graph entity/assertion identity remains separate from compiler-record identity and from physical
row identity. `ScopeTable` selects the complete registry for each realization. Canonical typed
bodies govern derived scope keys; graph `scope_context` and serving search materialization
columns have named consumers. A global scalar field inventory is not semantic authority.

## 3. Contracts, constraints and failure boundaries

`completion.rs` owns operation completion rather than each workflow independently choosing an
error precedence. It preserves a primary-only error's category and retains typed finalization
failures alongside it. Storage disposition, terminality, remote certainty and committed effect
are distinct. Recursive cleanup eligibility cannot turn a committed seal into permission to
delete its published database. Publication records seal commitment before later invalidation
or readback; backup file durability is separately represented.

The inspected native, bridge, Workspace and publisher siblings use the shared composition.
Blocking work retains an owned join observation across cancellation. Native row processing
keeps the first failure sticky, drains every transport terminal, and stores secondary drain
errors on `NativeRows`. `CompilerRows` records its primary before awaiting drainage; its drop
path reports both retained primary and secondary failures to admission before finishing the
lease. This closes the previously exposed lifetime hole in the inspected source. A dropped
caller does not prove remote rollback, and these contracts do not claim that it does.

`PreparedQuery` describes SQL, variables, selected result positions and every terminal result.
Buffered and streaming paths consume that contract. Streaming suppresses preparation-only NONE
rows while retaining all terminal checks; selected record operations explicitly identify their
multiple result positions. Statement-count or last-result guesses no longer decide whether a
query completed successfully.

Pure producer/admission/completion/parser controls can be isolated from provider execution and
native persistence. Native schema, selected access, publication, and serving cannot be qualified
by those pure controls. The source expresses these testing boundaries coherently. Actual cold
transport and late-error/cancellation cases still require the coordinator's relevant acceptance
receipts, rather than inference from type or code existence.

## 4. Composition and execution fit

The selected compiler route is a composition of narrow operations:

1. Supported single atomic equality branches stream compact nominal keys and real backing
   pointers through a prepared `WITH INDEX by_scope` query. General predicates scan exact
   contribution rows rather than inventing a union-wide ordered membership array.
2. The external sorter orders and deduplicates compact candidates. Bounded runs, charged merge
   heads and descriptors, and carried merge structure replace full-match rich sorting. Equal
   nominal identity with conflicting backing pointers rejects.
3. Bounded key windows establish membership in the exact selected contributors using physical
   row identity. Projected rows are hydrated afterward, residual predicates run before limits,
   and semantic ordering remains distinct from export's physical ordering.
4. Candidate windows and acknowledged blocking requests retain their ownership across awaits.
   The single-slot channel and durable shared join bound outstanding blocking work; temporary
   file lifetime belongs to that worker/result, not to a canceled caller future.

This is a credible qualitative route for overlapping scopes and growing matches. It avoids
substituting a growing native seen set for an array and avoids pulling rich canonical rows
through the sort. It preserves exact contributors, graph aliases, deterministic order and
residual eligibility. The relevant `SelectedRows` window persists across cancellation. Physical
export ordering and one-hop graph alias hydration remain explicit adapter policies rather than
nominal identity assumptions.

The DataFusion provider does not apply its `_limit` to candidate production. It translates
mandatory AND equalities/IN restrictions without treating an OR as mandatory, and retains
residual filtering before projection and fetch. The native execution plan delegates fetch to
the resolved streaming plan after its partition filtering and preserves metrics/statistics
behavior. A late eligible row must not disappear because an earlier candidate was limited.

Schema realization, compiler scope predicates and reconciliation share table-specific nominal
and scope declarations. Adding a compiler-only field need not add it to graph entities. Cold
verification now uses the complete relation registry, as F01 describes. Candidate plans on the
actual pinned native engine remain necessary for closure of the original plan's PC3/PC4
obligations; static plausibility is not an executed-plan receipt.

Restore separates whole parser units from HTTP requests. `Units` recognizes statements and
whole transactions while enforcing checked size limits. `Requests` packs whole units under the
existing byte/count bounds. The importer awaits and checks each request before sending the
next. A failed request can have private effects from later statements inside that request;
the contract permits that uncertainty and abandons/orphans the owned destination. It cannot
continue with another request or misreport rollback. This removes one-request-per-small-unit
amplification without requiring a whole-dump request or rollback implementation.

Published member selection reuses compact candidate sorting and now retains an owning `Members`
reservation for its complete rich domain. Classification owns its simultaneous roots and
requested-member set (F02). Complete domains still consume memory and can explicitly refuse a
budget; the correction does not truncate eligibility. Member hydration now consumes sorted
candidate physical pointers through a bounded direct-node helper, rather than nominal multi-key
family queries whose compound index cannot narrow the selected demand (F04). Bounded nominal
sorting inside that physical window preserves order without a family scan. Serving companion hydration's retired-field defect has been
corrected using canonical input values (F03).

## 5. Extension, substitution and growth scenarios

| Scenario and kind | Expected propagation and inspected behavior | Limit or settling evidence |
|---|---|---|
| Add a supplier role: domain extension | Add the semantic role to the producer contract and its actual provider composition. Captured bindings and exact role validation follow the declared contract; admission has no separate exception list | Independent multi-supplier and extra/missing-role controls |
| Admit a capture under another executable build: binding variation | Historical supplier/settings/source identity is checked against the supported semantic contract, not regenerated by the current provider | New-format cold ordinary/transported admission, both profiles |
| Add a compiler-record scope field: domain/layout extension | Declaration, appropriate `ScopeTable`, mechanical DDL/lowering and reconciliation change together; graph entity scalars need not grow | Actual generated native layout and selected query control |
| Replace request batching or SDK transport: mechanism substitution | Consume whole bounded units; retain sequential checked completion, private destination and abandonment outcomes | Failed multi-request case with no subsequent send |
| Cancel after primary failure or during a late drain acknowledgment: lifetime variation | Sticky primary and retained secondaries outlive awaits; admission owns remaining observation; blocking worker retains join/scratch ownership | Native multi-terminal and cancellation controls |
| Many overlapping scopes, aliases and late residual matches: growth | Compact external ordering, exact membership and bounded hydration preserve answers without native full-match sort/union state | Actual equality/contributor plans and independent selected-result expectations |
| Grow complete published member domain: growth | Complete eligibility remains legitimate; all simultaneously retained rich members, roots and requested membership must have owned budget reservations | F02 correction and small-budget multi-window case |
| Hydrate a selected member subset as the family grows: growth | Already ordered physical candidates now drive bounded direct-record reads; response projections and canonical identities must agree exactly with candidate demand | F04 source correction inspected; actual hydration-plan/result control pending |
| Hydrate corpus or direct-library captures: composition | Canonical capture input drives InputDistribution; corpus membership provides the library; optional CorpusLibrary inclusion cannot remove required distributions | F03's independent actual-schema companion case, pending execution |

These are credible variations already exposed by the target, rather than a hypothetical backend
framework. A new semantic concept may legitimately change a declaration and consumer policy.
It should not require consumers to reconstruct provider or physical-schema meaning. No claim of
inexpensive arbitrary backend replacement is made.

## 6. Findings and current inspected evidence

Current execution disposition belongs solely to
[coordinator §8](../../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition).
The following paragraphs preserve the dated diagnoses and describe inspected correction evidence;
they do not create a second mutable progress ledger or close the original review's F01–F06.

<a id="f01"></a>
### F01 — Cold completed-view lookup excluded graph-backed families

**Owner:** native compiler state verification. **Original consequence:** a valid completed view
for a graph-backed family such as packages/releases could fail with `completed view relation`
because `verify_state` searched `schema::compiler_relations()`, which intentionally excludes
graph-backed declarations. This violated FP-04 / A2 and DP-01/03, G3/G6 at the cold boundary.
The initial observation was in `crates/lctx-surrealdb/src/compiler.rs` during this review.

**Inspected correction, Implemented:** `verify_state` now selects
`ScopeTable::for_relation(&view.relation)` (`compiler.rs:574`) and searches that table's complete `relations()`
inventory before its exact view scan. The correction restores the declaration registry as the
authority and does not introduce a graph-family exception list. Static reinspection finds no
remaining manifestation at this lookup. Closure still needs the coordinator's valid new-format
cold round trip spanning graph and compiler-record families; this is distinct from asserting
all retained backing/admission obligations have passed.

<a id="f02"></a>
### F02 — Published member accumulation escapes retained-state ownership

**Owner:** `crates/lctx-serving/src/selection.rs`, `members` and `classify`. **Original
inspected consequence:** `members` drained bounded compact batches into an uncharged full
`Vec<CatalogMember>`. CatalogMember retains path/name payloads, so a growing matching domain can
retain rich state outside the declared budget after the sorter has released each bounded
transfer. `classify` then builds a full roots vector and requested BTreeSet alongside charged
hydration and finite classification. Neither compact sorting nor a bounded response covers
these simultaneous lifetimes. This violated FP-07 / A4, DP-19/20 and CI-08 / G5.

**Inspected correction, Implemented:** `members` now returns `Members`, which privately owns a
`ChargedVec<CatalogMember>` and `StateCharge` (`selection.rs:13`); slice access cannot detach the rows from their
reservation. Shared `ChargedVec::push` reserves capacity growth and actual dynamic member heaps
before retention. Each bounded hydration window has a transfer charge, and moved member heaps
transfer into the retained owner. Existing serving operations borrow the collection while its
owner remains live, including operation-packet awaits.

`classify` requires that owner under the same request budget. `ClassificationRequest` (`selection.rs:21`) reserves
its fixed generated graph-root strings and vector growth before constructing roots, keeps the
root charge through the hydration await, and owns a `ChargedSet` reservation through candidate
filtering. Failed growth or cancellation drops these owners; the member collection remains
charged for its caller's remaining lifetime. The correction uses the existing charged containers
rather than introducing another resource framework. Complete finite eligibility remains
complete; there is no early response limit or blanket refusal of legitimate global domains.

**Closure evidence:** independent inspection of `selection.rs`, shared `charged.rs` and the
actual operation borrows supports correction of the retained-state diagnosis through success,
failure and cancellation at static strength. Three new pure control sources cover 300 long-path
members, 180 classification roots/requested IDs, retained lifetime/order, typed budget refusal
and foreign-budget rejection. They are **not_run by this reviewer** and do not exercise the
actual native multi-window hydration journey. Execute the revealing multi-window small-budget
case within existing acceptance: complete permitted results or explicit resource refusal,
released reservations, and preserved late eligible members remain the meaningful guarantees.

<a id="f03"></a>
### F03 — Serving capture companions consumed retired graph scalars

**Owner:** `crates/lctx-serving/src/scope.rs`, capture companion hydration. **Original
consequence:** after the entity layout retired `scope_input`, companion queries still selected
that scalar for SourceArtifact, CatalogMember and Unit. The resulting corpus/distribution
keys could omit required evidence even though canonical `body.input` remained present. This
violated FP-04 / A2, DP-01/08, G2/G6 and CI-11 / CI-G2. Search occurrence `scope_input` and
`scope_window` are separately declared materialization fields; their remaining readers are
not instances of this graph scalar defect.

**Inspected correction, Implemented:** capture/source companions now select canonical
`body.input` (`scope.rs:100`); CorpusLibrary keeps typed `body.library`; corpus and distribution keys use shared
`prepared::scope_constants`, including its native string lowering. A corpus capture still
establishes both corpus and library distributions. A direct-library CatalogMember establishes
its own library distribution. Optional CorpusLibrary output does not determine whether the
required distribution is included. The correction reuses semantic values and physical lowering;
it does not restore a compatibility scalar.

`crates/lctx-serving/tests/native_companions.rs` now builds the actual current schema and
independently names expected evidence for corpus SourceArtifact/Unit and direct-library
SourceArtifact/CatalogMember roots. It checks exact distributions, optional corpus inclusion,
foreign exclusion and retired-field absence. **This control is not_run by the reviewer and its
native execution is pending.** Static reinspection supports correction of the source diagnosis;
native execution is the remaining meaningful closure evidence for this query composition.

<a id="f04"></a>
### F04 — Published member hydration discards physical demand and repeats family scans

**Owner:** `crates/lctx-serving/src/selection.rs`, member hydration, and the typed native reader
boundary; disposition PC4/PC6. **Original observed consequence:** `members` retained `Candidate.node`
through compact ordering, then discarded it at the initial `selection.rs:135–136` and called
`NativeReader::records(RecordSelection::Keys)` for every 128-candidate window. That reader emits
`semantic_type=$type AND semantic_key IN $keys` over entity and assertion tables, with nominal
ORDER BY (`reader.rs:102–108`, `reader.rs:142–145`). The schema's `semantic_key` index is compound
`(semantic_type, semantic_key)` (`schema.rs:211`).

Resolved SurrealDB 3.3 source normalizes only singleton IN to equality
(`exec/index/analysis.rs:1438` onward), builds compound prefixes from equality/range predicates
(`analysis.rs:1483` onward), and permits separate multi-value expansion only up to 32 values
whose IN field is the index's first column (`analysis.rs:1085`, `analysis.rs:1114`). These member
multi-key windows cannot narrow that index to the selected keys. A family-prefix index scan can
therefore repeat for every hydration window, even though output rows and application transfer
state remain bounded. This is an examined-work defect, not evidence that every unrelated rich
payload crossed the transport or that results were wrong. It violated FP-07 / A4 and
the relevant DP-10/14 execution-fit guidance.

**Inspected correction, Implemented:** `records_from_candidates` (`reader.rs:159`) consumes the
already carried real candidate nodes for one bounded direct `FROM $nodes` read. It rejects more
than 128 candidates, incorrect family/table, duplicate or unordered nominal keys, a mismatched
response count, and conflicting projected nominal/physical identity. Shared canonical decoding
and validation additionally prove the decoded typed record's key and graph physical identity
match the selected candidate. A checked `PreparedQuery` supplies the single result position and
complete terminal inventory. Missing rows are explicit conflicts, not silently omitted members.

`selection::members` now calls this helper with its ordered candidates and retains the F02
transfer and retained-state owners. The immutable published reader supplies the selected
universe; no compiler-contributor lookup is needed here. Bounded ORDER BY over the explicit
physical source preserves nominal order. Locked `exec/operators/source_expr.rs:103` resolves
bound RecordId arrays through `batch_fetch_in_place`, which point-fetches only those records.
The correction introduces no entity-family scan, early limit, new index, ADR, universal reader
framework or wholesale generic Keys rewrite.

**Closure evidence:** independent inspection of the helper, shared canonical decoders, caller,
control source and locked point-fetch source supports correction of the source diagnosis.
`native_member_hydration_uses_bounded_physical_windows_without_family_scans` (`native_search.rs:60`)
loads 260 selected and 320 unrelated members, checks all three nominally ordered windows and
retained charges, and inspects the actual hydration EXPLAIN for a bounded SourceExpr with no
Table/Index/Union/Dynamic scan. It also checks the 129-row bound, wrong/missing pointer, duplicate
demand and canonical identity mismatch. This independently extends the earlier candidate-only
EXPLAIN control. **The new native control is not_run by this reviewer and its execution remains
pending.** No new native execution was performed for this follow-up. The source diagnosis is concrete;
its elapsed-time contribution and speed benefit are not Measured claims.

F01 and F03 share a cutover-completeness concern but require different corrections: complete
declaration lookup versus semantic capture companion derivation. F02 is a separate retained
state/lifetime issue; F04 independently concerns repeated examined work after compact selection.
None is solved by increasing engine or memory limits. Their corrections
can coexist without adding a competing semantic classifier, old-format reader or early limit.

## 7. Architectural judgments and applicable principles

| Judgment | Verdict at inspected source boundary | Evidence |
|---|---|---|
| A1 — Change locality | satisfied | Producer semantic declarations, executable bindings, schema lowering, admission and completion hide distinct reasons for change; supplier/layout scenarios have named owners |
| A2 — Domain alignment | satisfied after inspected F01/F03 source corrections | Consequential contract/capture/outcome/identity distinctions govern executing admission, table lookup and companion queries; no semantic authority requires a current producer replay |
| A3 — Composability and local reasoning | satisfied | Prepared result positions and terminals, exact-view admission, operation outcomes, candidate ordering/membership/hydration and import-unit/request contracts compose explicitly |
| A4 — Execution fit | satisfied after inspected F02/F04 corrections | Compiler/restore have credible bounded work and lifetime structure; published selection owns retained state and physically hydrates bounded selected-node windows |

FP-01–FP-07 are **satisfied** within the examined correction boundary by the contracts and
scenarios above, after the inspected F01–F04 corrections. DP-01–05/08/09/11/12/18/21/24 and relevant
CI-01–06/10/11/13 are **satisfied at static Implemented strength** in the corrected boundaries:
one semantic authority, typed distinctions, shared enforcement, layered identity, explicit
outcomes, exact reuse premises, deterministic declared universes, hermetic captured inputs and
pinned evidence. F01/F03's original violations are retained in §6, not hidden by this current
source assessment.

DP-06/10/13–17/23 are **satisfied** for the examined compositions: thin pinned-library adapters,
shared lowerings, bounded physical hydration and meaningful independent acceptance cases. The
F04 direct-record library primitive now fits the selected hydration operation; its original
discarded-demand violation remains recorded in §6.
DP-19/20 and CI-08 are **satisfied after F02's source correction**, with retained-member,
classification, completion and blocking lifetimes explicitly owned. CI-07/09 algorithms and CI-12
protected evaluation behavior are not changed or independently audited by this review; they
are **n.a.** to a new algorithm/evaluation judgment. No enclosing algorithm or evaluation
certification is inferred.

## 8. Correctness and fidelity gates

Gate pass here means the inspected mechanism satisfies the stated static scope. It does not
mean execution checks passed or erase the original plan's acceptance conditions.

| Gate | Verdict | Evidence and remaining execution boundary |
|---|---|---|
| G1 Authority | pass | Declarative producer support, captured bindings, canonical bodies and complete table registries own meaning; admitted reporting has one source |
| G2 Semantic fidelity | pass after F03 correction | Captured roles and unavailable/NotRequested remain distinct; canonical corpus/library input meaning is preserved; native companion case pending |
| G3 Validity | pass after F01 correction | Shared admission/reconciliation and header rejection; cold lookup covers complete registry; integrated cold controls pending |
| G4 Hidden behavior | pass | Explicit producer effects/configuration, attempt-owned writes and observational admission; no ambient provider replay introduced |
| G5 Consistency and recovery | pass after F02 correction | Durable drainage, join observation, committed effect, restore abandonment and retained serving member/classification state have explicit owners; execution cases pending |
| G6 Transformation and reuse | pass after F01/F03 corrections | Exact dependency views, nominal/physical ordering, exact membership, residual-before-limit, alias policy and canonical companion lowering; actual plans/round trips pending |
| G7 Truthful capability claims | pass in this review | Implemented/Interface-checked is separated from attributed tests and pending native qualification; no throughput/capacity claim |
| G8 Library leverage | pass after F04 correction | Compact external ordering fits the native state gap and direct-record hydration now consumes its selected physical demand; no extra runtime/framework |
| CI-G1 Fidelity | pass in examined admission/selection source | Supplier/source/model/coverage identity retained; no observed inference relabeling or unknown-as-absence introduced |
| CI-G2 Evidence closure | pass after F03 source correction | Correct capture companions and pinned readers preserve the intended evidence universe; actual serving transport cases pending |
| CI-G3 Evaluation integrity | n.a. | Protected evaluation inputs and evaluator meaning were not changed or audited; PC6 evaluator behavior is not certified |

## 9. Library fit, alternatives and total complexity

The resolved family examined is SurrealDB 3.3.0, DataFusion 55.1.0, Arrow 59.3.0, Tokio 1.53.2
and futures-util 0.3.34. Local locked source and selected offline capability skills were used.
This is a design/code review, not a request for current library documentation.

SurrealDB's atomic indexed branch and record selection are useful execution primitives. Its
resolved union-index operator (`surrealdb-core/src/exec/operators/scan/union_index.rs`) retains
a growing seen HashSet, so a native union is not an equivalent bounded replacement for compact
external ordering. Engine ORDER BY and rich full-match arrays are likewise insufficient when
the requirement is bounded application state and exact output. The chosen composition adds
scratch/merge ownership but constrains it and leaves semantic selection independent.

DataFusion's resolved `StreamingTableExec` applies `LimitStream` after partition filtering and
projection and implements `with_fetch`; `ExecutionPlan`'s default fetch support is absent.
Delegating through that built-in is preferable to a second limiter with different residual
semantics. SurrealDB's resolved remote HTTP importer reads and checks query result errors after
each request; the adapter needs to compose whole units and sequential requests, not reimplement
the SDK's result decoding. Tokio's running blocking task cannot be assumed canceled by dropping
or aborting an awaiting task; retained completion observation is therefore a real lifecycle
requirement, not decorative wrapping.

The simplest viable alternatives coincide with the chosen design in most places: ordinary
model functions for producer binding and completion, table-derived scope lowering, SDK import
with bounded request composition, and DataFusion's native fetch. A universal provider framework,
additional canonical Arrow store, whole-dump request, compatibility scalar or response-limit
shortcut adds burden or violates semantics. Retaining the old global scope layout would make
semantic additions impose unrelated per-kind fields and work. Native-only candidate union/sort
could be revisited if the pinned engine supplies a composed route with the same exact membership,
bounded state, terminality and ordering; availability alone is not that evidence.

No speed or memory-capacity improvement is claimed as Measured. Static reasoning establishes
why compact transfer and request batching remove avoidable amplification, and why carrying the
member/classification reservations corrects F02's complete serving resource contract. F04 shows
why the full candidate-to-payload composition still matters after choosing those primitives.

## 10. Verification evidence and unresolved qualification

**Reviewer execution outcomes:** builds, tests, native plans, probes, real-library compilation,
operator inspection/adoption, and performance measurement are all **not_run**; there is no
reviewer command receipt for them. Read-only source inspection is the evidence for this report.

The coordinator reported on 2026-10-08 a focused model set with 44 passed controls and one stale
fixture failure being corrected/rerun, and isolated PC3 evidence of 11 pure controls plus one
actual native plan/scope control passed. These are attributed bounded reports. This reviewer did
not execute them or inspect a complete integrated acceptance receipt. A fixture correction from
`content` to the actual mutable `view` field with an independent changed-value assertion must
not silently turn the failed initial run into a clean pass.

The remaining execution obligations that matter to the integrated decision are the existing
plan's actual equality/exact-contributor and published plans; exact membership, aliases, order
and late residual/fetch results; both-profile new-format cold admission under captured-build
variation; primary/secondary/late-terminal/cancellation combinations; failed multi-request
restore with no next request; complete backup/restore/direct-seal transport; actual CLI/PyO3/MCP
and evaluator consumers; and the revealing F02/F03 controls and actual F04 hydration-plan/result
control described above. These are scoped
acceptance obligations, not a new test campaign or a demand for probes merely to conduct a
review. Coordinator run handles and summaries own commands and outcomes.

The final static recheck inspected the F02 owner and controls after their author reported source
stability, rechecked F03 against its actual-schema control source with enforced reference targets,
and inspected F04's direct-node helper, actual caller and native hydration control source. The
following SHA-256 values identify those decisive dirty sources and the durable-drain/cold-lookup sources
at that examination; they are source identity, not validation receipts.

| Source | SHA-256 |
|---|---|
| `crates/lctx-serving/src/selection.rs` | `89a87bd910af508873e12b88378389087771d90e73b5c223df347da7ff2d4ec0` |
| `crates/lctx-serving/src/scope.rs` | `53bcf38fe4d595dcb508f7fd9d773d636aaee4fd3a0ae4055518e28509f43907` |
| `crates/lctx-serving/tests/native_companions.rs` | `21258bcb1eaa0dc30457ff9f66559c86bb2eec4c57c2211b560a807cc19c6ade` |
| `crates/lctx-serving/tests/native_search.rs` | `66bed4e4db94f5844e3cff950e9e1c89f59977c5d8d828233283c1ca251b15a0` |
| `crates/lctx-surrealdb/src/compiler.rs` | `03f008979da5d45e9fd65661d7b596f9da5a65d585d4f284ba30af2f4402058a` |
| `crates/lctx-surrealdb/src/reader.rs` | `6fa4396766539a91ef79a1cdf842485f460769edb83314d1e7466ba09c2d6cbc` |

The mixed dirty tree includes ongoing PC6 controls. This report certifies neither an immutable
release revision nor the entirety of PG0–PG9. Later source corrections require targeted
reinspection of the affected finding, and acceptance receipts must correspond to the final
integrated source. No operator database or registration was inspected or changed.

## 11. Rule impacts and disposition route

**Authority changes: none.** The recommendations complete the functional contracts within the
chosen persisted/native architecture; they do not require revising a binding decision, replacing
ADR-0135, relaxing exact dependency pins, or adding an approval gate. The original review's
RC01/RC02 remain its own distinct decision obligations. This review does not apply or reinterpret
them as new authority changes.

Add this review's source links to the sole coordinator disposition owner: F01 belongs to native
state/cold verification, F02 to published serving selection and resource lifetimes, F03 to native
schema/serving companion integration, and F04 to PC4/PC6 published physical-pointer hydration.
Preserve original source-review IDs and existing closure
requirements. A source correction can establish Implemented closure of its diagnosis while
actual native or integrated obligations remain open; accepting ADR-0135 establishes neither.

## 12. Bounded decision

**Accept the examined assembled architecture at static Implemented / Interface-checked
strength.** A1–A4 and scoped G1–G8 are satisfied after independently inspecting F01–F04
corrections, including the complete physical candidate-to-member hydration composition.
Applicable CI-G1/CI-G2 hold in the examined source; CI-G3 is outside this review's
evaluation-integrity scope. All four diagnoses remain recorded, with relevant cold, resource,
native companion and physical hydration execution pending. No remaining material source defect
was found in this bounded review's examined correction routes.

Retain complete semantic eligibility, the independent exact-view/captured-admission contracts,
explicit completion distinctions, table-derived scope meaning, compact ordering and checked
whole-unit import. Execute the existing relevant acceptance boundaries against the final
integrated source before declaring PC6 complete. This is an architectural/source decision
at the stated evidence strength; the enclosing persisted compiler/publication/serving undertaking
and release qualification remain bounded by the coordinator plan and its actual receipts.
