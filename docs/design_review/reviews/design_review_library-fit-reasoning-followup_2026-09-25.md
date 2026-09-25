# Design review: reasoning-library conclusions and the omitted architecture

**2026-09-25 · Design / target · Decision: Revise.**

The [source review](design_review_library-fit-reasoning_2026-09-25.md) correctly favors expanding
biodivine, retaining fcars as an oracle, and withholding a production z3/OxiDD migration. Its
most consequential confirmed defects are incomplete cyclic value propagation and BDD admission
that depends on representation history. Several proposed corrections need qualification: Ascent
has a timeout API, datafrog has relevant uses beyond runtime rules, Arrow IPC does not itself
unify semantic decoders, and literal recognition is not the whole dynamic-access problem.

The wider investigation finds additional defects in the **meaning passed between owners**:
field-access recognition, operation verdicts, claim evidence, FCA attributes, embedding cache
admission/commit results, corpus dependency identity, and model-service configuration. These
matter before adding more engines. Correct Boolean algebra or concept enumeration does not repair
information discarded before analysis or after publication.

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Source review F01–F16 and library decisions; current Rust workspace and Python serving paths at `92eca2e8244a82f574be48e3124855d14f6b8241` |
| Tree | Initial tracked tree unchanged; source review and its BDD evidence were already untracked. They are preserved. This follow-up adds review/probe evidence and updates STATUS only |
| Standard | Repository core 3.0, code-intelligence profile 1.1, and library-context binding from `docs/design_review/design_principles/standard.toml`; design-review and companion profile skills |
| Reviewer | Codex, with three independent read-only investigations of flow/provider semantics, synthesis/serving, and acquisition/cache workflows; accountable reviewer re-read the source paths used below |
| Tier / purpose | Design / target, assembled-code follow-up. This is not Stage 3 exit or release qualification |
| Functional scope | Boolean/primitive reasoning; recursive flow and summaries; FCA/RCA; source acquisition and context identity; embedding preparation/cache; brief synthesis, operation lookup, and immutable serving generations |
| Expected changes | More shared condition roots; recursive transfer families; equivalent Python import/callee spellings; a new facet or renderer; a new RCA attribute; another cache-fill workflow; a source/model release upgrade |
| Method | Pinned offline skill contracts and source, current Context7 discovery, independent source traces, small probes at material uncertainties. No production fixes, pin changes, ADR decisions, or plan scheduling |
| Coverage limits | Not an exhaustive audit of every extractor rule or SQL validator. No live Qwen run, fresh pilot, clean wheel, whole-workspace gate, large-catalog memory measurement, or recursive-engine bakeoff. Original F07/F08/F15 measurements and broad claims are not recertified |

**Evidence convention:** source-path existence and traced branches below are **Implemented /
Interface-checked, 2026-09-25**; proposed corrections and expected benefits are **Proposed**.
Only explicitly named executions are **Tested**. Diagnostic probes that reproduce defects have
execution outcome `passed`; the observed contract remains violated. Historical receipts remain
historical. This distinction applies to every table below.

## 2. Responsibilities, dependencies and semantic ownership

| Component | Coherent responsibility / hidden decisions | Consumer contract and dependency direction | Scenario consequence |
|---|---|---|---|
| `cpg-extract::{library,config}` | Acquire code; define all analyzer-readable inputs, context and run identity | Explicit release/context → provider configuration and facts | Selected corpus files do not cover the full import search root (F07) |
| `cpg-flow` / `cpg-core::flow_model` | Provider flow extraction; resolve/domain-normalize value and access relations | Attributed flow facts → contributions, access facts and negative premises | Two different recognizers of builtin attribute access disagree on equivalent spellings (F05) |
| `cpg-schema::{condition_kernel,primitive_theory}` | Canonical Boolean structure; exact-input assignments over checked links | Repository IDs and typed outcomes; biodivine handles remain private | Good Boolean/theory split; total hydrated generation cost is not represented (F01) |
| `lctx-analytics::summaries` / `cpg-core::summaries` | Pure finite composition and SCC policy versus acquisition/publication | Explicit inputs/outcomes should flow inward; sessions remain outside | ARC-02/03 already own isolation/refusal work; an engine choice must follow this boundary |
| `lctx-analytics::concepts` / schema / Stage F | Enumerate concepts over domain attributes; render results | Typed attributes should map to bit positions, then back to evidence-bearing meaning | English labels currently carry semantic identity and call modality disappears (F04) |
| `cpg-core::embed` | Admit requests, populate cache, return canonical committed vectors | Shared spec + requests → committed values/keys/version | Two entry points split admission policy and one returns locally computed candidates without reconciling the committed winner (F06) |
| Embedding spec / launch / clients | Bind one model deployment to a declared vector space | Owned deployment specification → launcher and client configuration | Revision/dtype repeated in launcher; endpoint model name does not verify deployment revision (F08) |
| Stage F / bundle / Python renderers | Synthesize assertions; publish their evidence closure; render without reinterpretation | Canonical claims/verdicts/supports → structured and Markdown views | Facet verdict and support closure are lost in separate projections (F02/F03) |

| Concept | Semantic authority / identity | Derived forms |
|---|---|---|
| Boolean function | Sorted repository atom encodings and content-addressed BDD root | In-memory diagrams, display DNF, native decisions; support history must not affect substitutability |
| Runtime claim | Stated model, canonical relation/verdict and cited evidence | Facets, assertions, resources. A string label is not a substitute for modality or evidence |
| Embedding value | The insert-only cache row at its committed version | Analytic input vectors and generation vectors; newly calculated vectors are proposals until commit |
| Analysis context | Complete pinned content/membership and configuration of actual provider inputs | Context/run/content identities; a path or Git HEAD is not the content it exposes |

**CI fact and fidelity table**

| Fact / relation | Provider and fidelity | Coverage / unknowns | Identity and consumer |
|---|---|---|---|
| Boolean decision | biodivine 0.6.3; exact over independent Boolean atoms | Node/work/support refusal is unknown; no concrete Python execution follows from satisfiability | Canonical atom/root IDs; predecessor screening and native path queries |
| Exact-input assessment | `primitive_theory`; derived under checked entry links and builtin assumptions | Unsupported comparisons and missing links remain unknown | Link IDs + effect-model digest; path-local compatibility/refutation |
| Reaching contribution | ty/Pyrefly/Ruff inputs plus custom `Model::reach`; derived may-analysis | Cycle-head memo can omit transfer variants; source-review F05 confirmed | `(Origin, Transfer)` and raw fact provenance; summary seeds/boundaries |
| Attribute read / no-read premise | ty flow traversal plus core builtin resolution; derived | Qualified/aliased literal access is omitted in tested cases (F05) | Field/global place; served `never_read` claim |
| RCA attribute | Declared call projection plus relational scaling; derived | Candidate/definite distinction erased before FCA and rendering (F04) | Currently display strings; concept members and implication claims |
| Operation facet | Canonical materialized verdict; derived | Per-value verdict and facet completeness are distinct; lookup drops the first (F02) | `(node, facet, value)`; lookup and exhaustive filter |
| Brief assertion | Programmatic synthesis over findings/evidence | Canonical support exists; served finding closure and resource citations are incomplete (F03) | Assertion/support IDs within one snapshot |

## 3. Contracts, constraints and testing boundaries

| Contract | Enforcement / lifecycle | What is sound or missing | Isolated verification |
|---|---|---|---|
| Diagram identity/admission | Validated node order, reduction, closure and Merkle IDs | Structural validation is real; admission is history-sensitive and hydration allocates per root | Actual-source kernel harness, no store |
| Primitive refutation | Checked value links, exact builtin values, bounded Boolean composition | Keep assignments/proof separate from displayed residual; never promote Boolean SAT to execution | Truth tables plus independent Python literal controls; existing tests are examples |
| Recursive transfer | Source reach memo and summary refusal paths | A cycle needs an explicit join/fixpoint, not a DFS cutoff; ARC-03 must retain refusal causes | Actual `Model::reach` harness; full extraction/publication impact separately scoped |
| Access/no-read | Shared publication checks reconstruct core access/negative-premise rows | Reconstruction agrees with the same missed semantic input; bare/qualified/aliased controls challenge it independently | Real extraction + DataFusion derivation + `flow_model::run` probe |
| Serving | One loaded generation; digest/schema/native checks | Integrity and version pinning do not prevent lossy result projection | Existing fixture generation with current Python lookup/rendering |
| Cache fill | Token validation in one entry; insert-only Delta merge and retry | Admission differs by caller; returned vectors can differ from committed values | Coordinated race and over-cap fake-embedder controls proposed; no live-provider pass claimed |
| Corpus context | Explicit config, selected-release hash and site-packages hash | Whole readable corpus import root lacks content/membership closure | Unselected-helper edit/addition versus clean recomputation proposed |

The successful isolated flow probes also show that resolving these questions need not bootstrap
embedding, Delta publication, or the server. Preserve this boundary while implementing ARC-02.

## 4. Composition and execution

| Question / stage | Projection and settings | Accuracy / limits | Output and evidence linkage |
|---|---|---|---|
| Compile one release/corpus | Selected source facts; explicit root-first corpus imports | Pinned dependency intent; F07 leaves actual root contents outside context identity | Runs feed snapshot comparison identity |
| Propagate value sources | Function-local use/definition graph with transfer variants | May-analysis; current cycle cutoff is not its fixed point | Contributions → summary seeds/refusals; source-review F05 |
| Decide / hydrate conditions | Union atom vocabulary; per-diagram 128 atoms / 50k nodes / 1M product preflight | Exact Boolean answers or typed refusal; catalog row caps do not budget expanded retained diagrams | Root IDs/structural nodes → native diagrams; F01 |
| Evaluate exact input | One operation/formal; validated link subset; 32 assignments / 1M assignment work | Path-local may-model, not operation-wide behavior | Cited links and typed `TheoryBoundary` |
| FCA/RCA | Declared function universe; optional `+fca,+rca`; one existential relation-scaling step | FCA can be exact over a semantically lossy attribute context | Current string members → Stage F text; F04 |
| Prepare vectors | Briefs, operation views and optional E0; one hashed spec/cache | Byte windows are preparation; tokenizer should own admission; merge chooses stored value | Keys/version + vectors to analytics/generation; F06/F08 |
| Serve claims | One canonical generation, deterministic ID hydration; no second search for details | Facet/claim precision must survive each renderer | F02/F03 lose interpretation/citations after canonical construction |

## 5. Change and failure scenarios

| Trigger | Owner / expected contract change | Observed propagation or hidden knowledge | Settling evidence |
|---|---|---|---|
| Many conditions share a large suffix | Kernel generation preparation; explicit aggregate budget, same persistent IDs | Every root gathers/clones/validates/owns its closure; storage sharing hides retained expansion | 256 roots / 320 stored nonterminals → 17,152 owned BDD nodes (F01) |
| Add effect/exception/role recursive families | Summary transform, joins, convergence and refusal contract | An engine macro cannot choose semantic keys, witnesses or cutoff behavior | Source F05 reproduction; ARC-02/03; engine comparison criteria in §8 |
| Replace `getattr` with its qualified or imported alias | Access normalization; unchanged access meaning | Provider spelling check and core resolution take different paths | Real-provider qualified/alias failures and bare/direct controls (F05) |
| Display a candidate facet or add another rendering | Consumer returns canonical value/verdict and support closure | Lookup erases verdict; Markdown renders claim text without its finding citation | Existing-generation probes (F02/F03) |
| Add/relabel an RCA attribute | One typed attribute declaration plus domain logic | Producer builds English; renderer parses prefixes and can silently ignore a new form | Source trace through `relational` → `attributes_text` (F04) |
| Add another embedding workflow or race a cache fill | One admission/commit/read operation | Callers share cache keys while enforcing different admission; a loser vector can escape | Source trace; focused race/cap controls proposed (F06) |
| Edit/add an unselected helper imported by a selected example | Context identity or immutable acquisition should change/reject | Selected hash and HEAD stay fixed; provider can read changed helper | Actual search path/hash/HEAD trace; reproduction proposed (F07) |
| Upgrade model/tokenizer revision | One spec drives launch and vector-space declaration | Launcher literals and client spec must change together; same model-name response is accepted | Source trace; deployment/launch conformance proposed (F08) |

## 6. Correctness and fidelity gates

These verdicts cover the paths examined here; they are not a certification of unexamined code.

| Gate | Verdict | Independent evidence / required action |
|---|---|---|
| G1 Authority | fail | F06 competing local/committed vector authority; F08 duplicated deployment configuration; carried ARC-01 |
| G2 Semantic fidelity | fail | F02 verdict removal, F04 modality-to-label loss, F05 equivalent spellings interpreted differently |
| G3 Validity | fail | F06 shared cache admission assumption is not enforced on every entry path; kernel node integrity itself is checked |
| G4 Hidden behavior | unresolved for full scope | Explicit provider config and ordinary pinned reads are sound inspected paths; full actual-input closure remains unresolved. No new claim of an inspection mutating the store. The constant ty-settings mismatch belongs to configuration fidelity/CI-10, not hidden effects |
| G5 Consistency / resources | fail | F01 budgets do not account for retained hydration expansion; source F05 has no fixed-point/exhaustion contract; F06 can cite committed values it did not consume |
| G6 Transformation / reuse | fail | Same Boolean ID has different admission (source F02); cycle query order changes contributions (source F05); F02/F04/F07 lose meaning/dependencies |
| G7 Truthful capability claims | fail | F05 no-read claim exceeds extracted coverage; F08 deployed revision not established by spec hash equality; source F16 remains a documentation/implementation mismatch |
| G8 Library fit | fail, bounded | Actual-source F01 confirms needless full-result construction for decisions. Do not count Ascent deferral, datafrog rejection, or lack of a global solver as independently proved violations |
| CI-G1 Fidelity | fail | Tested no-read premise and erased facet verdict; source-traced RCA relabeling. A generic graph/BDD/FCA oracle does not repair these |
| CI-G2 Evidence closure | fail for served resource; unresolved resolution in tool | F03 removes finding citations from Markdown and provides no served finding→source/model closure. This does **not** allege those IDs are missing from canonical Delta |
| CI-G3 Evaluation integrity | pass within inspected boundary | Gold/oracles remain separate from compiler inputs in the inspected routes. These probes challenge behavior; they do not tune parameters against gold. Full evaluation not run |

## 7. Findings and assessment of the source review

### 7.1 Source-review conclusions: confirm, qualify, or withhold endorsement

The IDs in this table belong to the **source** review. Its §11 remains their disposition owner
until scheduling; this assessment does not transfer, close, or renumber them.

| Source finding | Follow-up assessment |
|---|---|
| **F01 decisions** | **Confirmed and strengthened (Tested).** Actual kernel admits two 512-node operands whose 130,562-node conjunction yields `NodeLimit`; `check_binary_op` decides nonempty in 130,560 nonterminal tasks. An admitted 8,192-node diagram and its complement are refused by product preflight but decided in 8,190 tasks. The original 131,072-node operand exceeded the kernel's 50k admission; this probe removes that evidence gap. Keep transfer/support admission: replacing final apply alone does not bypass `in_union` limits. `None` from **check_binary_op** is unknown; only the separately justified **limit-1 result-size** idiom interprets `None` as nonempty. Upstream `cmp_implies` uses limit **2** with implication, not literally the limit-1 conjunction code cited. Its analogous bounded-size technique supports the reasoning. Count visited nonterminal tasks honestly; it is not all work or wall-clock time |
| **F02 support** | **Confirmed (Tested).** Live false diagram retains 128 redundant atoms; same-ID hydrated false diagram has zero. Adding `z` is refused only in the live instance. Normalize effective support after construction/operations and preserve ordering/identity. This fixes a substitutability defect; it need not make every intermediate budget decision algebraically associative |
| **F03 restriction / minimal links** | **Partly confirmed; separate requirements.** `given` still nominates from legacy DNF (`condition_kernel.rs:769–790`); cube restriction is a good bounded candidate generator, but retain `factor ∧ quotient == original`. Cofactoring alone is not factor division. Restriction preserves satisfiability under assignments, not the original function; persist original roots and assignment evidence, label any residual. Current prefix links are valid, merely non-minimal. Minimal explanation is an optional product improvement, not a demonstrated correctness breach. Deletion minimization costs up to one initial restriction plus 32 deletion trials, each scanning a diagram; give it a total work budget. If changing work counters, version/rename rather than reinterpret `bdd_preflight_pairs` |
| **F04 membership / integer equality** | **Confirmed as a local gap.** `primitive_theory.rs:281–317` lacks these branches; exact strings/integers need no solver. Preserve Python bool/int and operator restrictions. This is not sufficient to close Q09: operation-wide aggregation, links, effects, and remaining summaries are still open. The current code and native endpoint promise path-local outcomes |
| **F05 cyclic reach** | **Confirmed more precisely (Tested).** Actual `Model::reach` body over `A = p ∪ Call(B), B = A` gives A only Identity when queried A-first, but Identity+Call when queried B-first; acyclic control preserves both. Repair fixed-point semantics before engine selection. Demonstrated impact is missing transfer contributions and associated boundary/seed coverage. A surviving finite identity witness is not automatically false, and an end-to-end wrong summary was not reproduced. Parameter unread premises do not simply count contribution rows; the original negative-premise consequence was too broad |
| **F06 recursion engine** | **Reasonable spike, overstrong rationale.** Compare at the first genuinely shared recursive family, after ARC-02/03. Counting existing loops does not establish identical lattice/transfer semantics. Ascent has `generate_run_timeout`; this is not a deterministic inner-round work cap. Datafrog's explicit joins, small dependency surface and iteration control are relevant even for fixed rules. Neither engine automatically bounds witness growth or preserves provenance |
| **F07 recursive SQL** | **Source shape confirmed, cost/closure not rerun.** Both CTEs use `UNION ALL` over node/pair state (`cpg-schema::{concepts.rs:36–40,communities.rs:45–53}`). Set-valued recursion is a sensible candidate; a `UNION` does not itself impose a resource budget. Revalidate pinned execution/termination and key identity before adopting; original quantitative results remain attributed to that review |
| **F08 SCC / keyed order** | **Implementation and documentary mismatch confirmed; no new measurement.** `summaries.rs:15,56,98–119` uses Tarjan and reorders components. Reassess stack-safe SCC choice against current named policy; small keyed scheduling remains a defensible bespoke operation. Do not turn it into an automatic rustworkx-core dependency. New decisions must follow current accepted-ADR immutability rules, not edit an accepted amendment log as the original suggests |
| **F09 literal dynamic access** | **Confirmed but incomplete correction.** Quote-prefix tests are unsound, yet literal syntax alone misses qualified/aliased builtin access. F05 below is a tested cross-owner defect with a shared-resolution correction. Bare literal `getattr` is already handled and is a valuable positive control |
| **F10 abstract flags** | **Direction supported, with provider semantics retained.** Use attributed resolved abstract/body facts for the negative-premise exclusion. Pinned Pyrefly `pyrefly/lib/alt/function.rs:447–454` marks trivial/ellipsis Protocol methods abstract; Protocol membership alone does not establish abstractness |
| **F11 ty settings** | **Configuration mismatch, insufficient claimed effect.** Empty settings exist (`cpg-flow/src/lib.rs:327–329`), but `predicate.rs:334–405` already uses `RuntimeContext` for the repository's own comparisons. The pinned semantic index does not itself fold `sys.version_info` in the way the proposed regression assumes (ty-flow `reachability.md`, TY04). A meaningful test must hit a provider operation actually affected by settings (e.g. resolution), not claim a generic version-guard fix or a three-line integration estimate |
| **F12 independent tests** | **Agree, with semantic layering.** Small independent Boolean truth tables test the kernel; actual provider/control-flow cases test Python-to-fact lowering; served projection round trips test claim fidelity. fcars agreement cannot detect erased RCA modality. Absence of a property-test framework alone is not a gate failure |
| **F13 Arrow IPC** | **Candidate, not closure by construction.** Bundle schemas are projections with textual kinds and omitted canonical columns (`cpg-schema/bundle.rs:301–367`); canonical row decoders expect their own full schema (`table.rs:194–203`). IPC bytes do not automatically create a typed projection decoder or derive proof-kind support. Centralize that decoder/vocabulary first; decide transport by total conversion cost. `arrow-ipc` is pinned elsewhere, but the native crate currently declares only cpg-schema/PyO3. Adding an existing workspace dependency differs from needing no dependency work. F02/F03 survive a transport-only change |
| **F14 B/C order** | **Narrow to the concrete Pass B state defect.** Its suppression-sensitive path property is absent from the visited key (`pass_b.rs:341–406`), while suppression inspects the saved path (`:445–450`). Merely rejecting unsorted input does not resolve multiple semantic routes. Pass C sorts each output group before truncating (`pass_c.rs:148–166`); no Pass C output-order defect was established by its builder preserving input order |
| **F15 SQL round trips** | **Plausible cleanup, not a freshly qualified 21-table replacement.** Distinguish semantic rederivation from write-fidelity checks before deleting any validator. ARC-02 already owns the meaningful acquisition/policy separation; line reduction is not the architectural goal |
| **F16 schema migration prose** | **Mismatch confirmed; proposed correction needs a decision.** `delta::verify` rejects drift (`delta.rs:108–115`) while DESIGN §6.3 claims merge migrations. Decide supported migration/rebuild behavior, including access to older snapshots. "Fresh store" is a proposed lifecycle contract, not something this review can make authoritative by editing prose alone |

The kernel probe is [recorded here](../evidence/2026-09-25_library-fit-followup-kernel/README.md).
Flow/provider probes are [recorded here](../evidence/2026-09-25_library-fit-followup-flow/README.md).

### 7.2 Additional findings

Each ID below is local to **this** review; its current disposition is in the [forward plan §6](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition).

<a id="F01"></a>**F01 — Catalog budgets count shared storage, while hydration retains expanded per-root diagrams.**

- **Owner / principles:** `cpg-schema::condition_kernel`, native generation preparation;
  FP-05/06, DP-10/20, CI-08; A2, G5.
- **Evidence:** `hydrate_catalog` checks condition/node row counts at
  `condition_kernel.rs:80–85`, then calls `Diagram::from_catalog` and retains one Diagram per
  root at `101–113`. `from_catalog:279–300` gathers/clones each root closure;
  `from_root_and_nodes:311–374` reconstructs owned library storage. Native `ConditionGraph`
  retains these diagrams but reports stored node count (`python/lctx_semantics/src/lib.rs:139–163`).
- **Tested:** 256 distinct roots sharing a 64-node tail require only 320 stored nonterminal rows,
  yet hydrate into 17,152 owned BDD nodes. The [actual-source probe](../evidence/2026-09-25_library-fit-followup-kernel/README.md)
  measures node counts, not RSS/time/OOM. The current row caps admit much greater aggregate
  expansion; cost follows the sum of root closures rather than the number of unique stored rows.
- **Consequence:** adding many ordinary summary conditions can greatly increase validation/startup
  work and retained memory without approaching the advertised stored-node cap. The original
  OxiDD rationale confuses persisted deduplication with manager-style in-memory sharing.
- **Proposed correction:** keep biodivine; add an explicit aggregate preparation/retention budget
  with deterministic refusal, and avoid repeated structural validation where one validated
  catalog can serve it. Consider a bounded lazy diagram cache only if query locality justifies it;
  do not add a global manager merely for this finding. Keep semantic IDs independent of handles.
- **Closure:** shared-tail and disjoint-root controls exercise aggregate admission before large
  allocations; native diagnostics distinguish stored from retained counts. Measure a representative
  generation before selecting a different engine. Delete redundant eager copies if that route wins.

<a id="F02"></a>**F02 — Operation lookup removes each facet value's verdict.**

- **Owner / principles:** Python operation response contract; FP-02/05, DP-02/08, CI-02/04;
  A2, G2/G6, CI-G1.
- **Evidence:** schema defines value verdict separately from facet completeness
  (`cpg-schema/src/behavior.rs:922–945`). Load retains it; `_record` discards it with
  `for facet, value, _` (`operations.py:282–289`), and `Operation.facets` is only
  `dict[str,list[str]]` (`:155`). Filtering correctly distinguishes match/no/open (`:454–462`).
- **Tested:** current Python code over pre-existing FORMAT 7 fixture generation `3fc3f0a672790ffe`
  returns `pkg.controls.Registry.add` in `pkg.configure`'s plain delegation facet despite a
  stored `unknown` verdict. Filtering that facet returns zero matches and 17 unknown operations.
  [Probe and input provenance](../evidence/2026-09-25_library-fit-followup-serving/README.md).
- **Consequence:** the facet entries do not distinguish unknown from established/conditional
  values. Separate `Fate` arrays retain verdicts (`operations.py:231–249,343–345`), but recovering
  a facet value's status requires interpreting and matching those arrays where available.
  A facet-wide incompleteness flag describes missing rows, not the status of the rows displayed.
- **Proposed correction:** return typed `{value, verdict}` entries, retaining separate completeness.
  Derive this projection from the existing data; no solver, new database, or transport change.
- **Closure:** a single facet with established and unknown values preserves both distinctions in
  structured lookup and agrees with filtering; inspect the generated response schema as well.

<a id="F03"></a>**F03 — Finding-backed brief claims lose their traversable evidence/model closure; Markdown drops the citation itself.**

- **Owner / principles:** bundle support projection and Python rendering; FP-02/03/04,
  DP-08/21, CI-01/06/11; A2/A3, CI-G2.
- **Evidence:** Stage F `Draft::citing` keeps finding/status/kind (`synth.rs:283–290`);
  public-access/coordinates assertions use it without direct evidence (`:1085,:1195`). Canonical
  findings point to invocation/model and witnesses (`cpg-schema/src/findings.rs:22–66,82–102,125–148`).
  Bundle retains finding id/kind but no finding/witness/invocation closure (`bundle.rs:117–135`).
  Python hydrates only direct evidence IDs (`server.py:234–269`); Markdown prints text/status and
  an aggregate evidence list, omitting per-assertion support edges (`:301–314`).
- **Tested:** an existing fixture's coordinates claim cites only finding
  `c60a2729bc4b5c4b839676c6c97a7d03`; the generation cannot resolve that finding, and its Markdown
  representation omits the ID. Other evidence in the brief does not restore that claim's edge.
  [Probe](../evidence/2026-09-25_library-fit-followup-serving/README.md). No claim is made that the
  finding is missing from canonical Delta or that the behavioral sentence is false.
- **Consequence:** the advertised serving projection cannot take that claim to its model/source;
  the resource loses even the retained reference. Integrity checks and ARC-01 do not repair omitted
  evidence or a lossy renderer.
- **Proposed correction:** export a bounded canonical support closure for supported assertions,
  including invocation/model and witness/fact source references; hydrate deterministically and
  render the same support edges at each assertion. At minimum preserve current support IDs in
  Markdown and explicitly report unavailable resolution. No separate evidence framework is needed.
- **Closure:** follow a finding-only coordinates claim to its model and actual source span through
  the served generation; structured/resource forms preserve identical support relationships;
  missing support is rejected or the claim becomes unknown.

<a id="F04"></a>**F04 — FCA/RCA uses presentation labels as semantic attributes and loses candidate-call modality.**

- **Owner / principles:** schema attribute contract, `lctx-analytics::concepts`, Stage F;
  FP-04/05/06, DP-02/07/08, CI-02/06; A1/A2, G2/CI-G1.
- **Evidence:** `analyze.rs:1420–1429` maps call arcs to `(caller,callee)` without modality.
  `crates/lctx-analytics/src/concepts.rs:244–275` explicitly admits definite **or candidate** arcs and labels both `calls X`.
  FCA members retain strings (`:280–307,394–415`). Stage F parses English prefixes, silently
  ignores unknown forms, and renders unqualified `calls` (`synth.rs:349–407`); shared signatures
  and implications consume those labels (`:1637–1698`) under structurally-observed finding policy
  (`cpg-schema/src/findings.rs:627–635`). **Interface-checked**, optional `+fca,+rca`; no fresh RCA run.
- **Consequence:** exact concept enumeration over candidate relations can become a claim that an
  API calls a target. An ordinary label change or attribute addition also requires synchronized
  semantic edits in analysis and prose parsing. A concept-set oracle given the same flattened
  context cannot detect the discarded modality.
- **Proposed correction:** give attributes typed semantic keys, endpoint/value identity, modality
  and evidence; map keys to bit positions, render after analysis. Keep labels out of membership
  identity. A smaller interim correction preserves the declared may-call semantics in one owned
  rendering contract and rejects unsupported attribute forms; it does not fully solve typed identity.
- **Closure:** candidate-only shared-target and definite controls retain their different meaning
  in shared-signature and implication outputs; relabeling does not change concept membership.
  Replace prefix parsing when the typed path lands; retain independent concept-set controls.

<a id="F05"></a>**F05 — Attribute-access meaning is split between spelling-based extraction and resolved core interpretation.**

- **Owner / principles:** provider/core normalized access boundary; FP-01/02/04,
  DP-02/08/23, CI-02/04/06; A1/A2/A3, G2/G7, CI-G1.
- **Evidence:** `cpg-flow` emits a literal attribute load for syntactically bare `getattr`/`hasattr`
  (`lib.rs:1220–1230`). Core dynamic-access handling uses its own builtin/name resolution and
  quote-prefix rule (`flow_model.rs:1963–2001`). Negative premises combine collected reads and
  dynamic boundaries (`:2140–2288`). Bundle exports them (`bundle.rs:274–281`), and
  `operations.py:366–379` turns a true singleton field premise into "no read ... anywhere".
- **Tested:** real extraction, DataFusion derivation and `flow_model::run` over qualified
  `builtins.getattr(self, "qualified_only")` and an imported `getattr as read_attribute` produce
  no read/dynamic boundary and true field/global no-read premises. Bare `getattr` and direct
  attribute controls correctly make the premise false. [Evidence and exact scope](../evidence/2026-09-25_library-fit-followup-flow/README.md).
  This probe does not publish a new Delta snapshot or execute a fresh served query.
- **Consequence:** equivalent resolved builtin calls have different coverage; the current serving
  branch can render an unsupported negative. Source-review F09's literal-node correction alone
  leaves this seam open. A validator reconstructing the same producer output will agree with it.
- **Proposed correction:** normalize resolved builtin access with explicit literal/dynamic name
  status, receiver/value provenance and unsupported outcome at one boundary. Downstream read and
  no-read logic consumes that contract. Keep provider observations attributed; do not move all
  Python policy into an adapter or add a new resolver. Bare, qualified and aliased spellings must
  take the same declared semantic path.
- **Closure:** real-provider controls for all three spellings, computed names and shadowed builtins;
  qualified/aliased literal reads withhold no-read claims, and a genuinely unread field remains
  distinguishable. Then trace one corrected premise through publication and serving.

<a id="F06"></a>**F06 — Cache-fill workflows duplicate admission policy and return values before reconciling the committed winner.**

- **Owner / principles:** `cpg-core::embed`; FP-01/03/04/05/06, DP-03/09/11/19;
  A2/A3, G1/G3/G5/G6.
- **Evidence, admission:** `embed_documents` token-checks missing requests (`embed.rs:215–230`)
  and relies on cached keys having passed that check. `embed_texts` directly calls embedding
  (`:284–297`) into the same cache. Default operation views call it (`behavior.rs:1564–1587`),
  as does optional E0 (`analyze.rs:636–654`). The 4096-byte window is only a proxy
  (`neighbours.rs:35–46`); DESIGN §11.1 currently calls it a guarantee under 2048 tokens.
- **Evidence, commit:** `embed_texts:292–306` puts freshly calculated vectors in its local map,
  merges, then returns that map without rereading. `delta::merge_global:192–246` preserves existing
  keys and retries against a concurrent winner. The returned version names committed rows, not
  necessarily the returned vector. E0/kNN consumes the latter (`analyze.rs:636–654,690–701`),
  while the snapshot records the cache version (`attempt.rs:1064–1085`) and the generation joins
  the committed cache (`bundle.rs:283–293`). DESIGN §11.1 explicitly allows slightly different
  live vectors for identical requests. **Interface-checked**; no race or live tokenizer executed.
- **Consequence:** one entry point invalidates another's cache-admission assumption. With two
  misses for the same key, attempt A can consume vector A while recording winner B's cache version;
  a canonical replay can change near-threshold analytic results. The race consequence concerns
  enabled E0/kNN techniques; the missing token check also affects ordinary embedded operation views.
- **Proposed correction:** one cache-fill operation owns missing-key token admission, batching,
  merge and canonical versioned readback. Callers choose texts/views and consume committed results.
  Keep Delta's existing insert-only merge; delete duplicate policy and local-vector authority.
- **Closure:** a two-attempt test with deliberately distinct valid vectors proves returned values
  equal the read at the returned version. An over-cap embedder control rejects through both
  entry points before embedding/insertion. Cached admitted requests still need no live service.
  Such doubles validate cache semantics only, never live Qwen qualification.

<a id="F07"></a>**F07 — Corpus import resolution observes content outside the context/run identity.**

- **Owner / principles:** extraction acquisition/context; FP-02/04/05/06, DP-04/09/21,
  CI-10; A2, G6.
- **Evidence:** corpus release hashes selected documents and usage (`config.rs:125–148`),
  but Pyrefly searches the whole corpus root first (`:240–249`). Context hashes site-packages,
  not that root's complete membership/content (`:368–395`). Corpus input retains library
  environment paths (`cpg-extract/src/lib.rs:274–289`); run ID is derived before source reads
  (`:341–380`). Existing fetched source is checked only by HEAD (`lctx/src/main.rs:386–394`).
  Run IDs feed content comparison identity (`attempt.rs:223–242`). **Interface-checked**.
- **Consequence:** editing or adding an unselected helper imported by a selected example can
  change provider facts without changing selected bytes, HEAD, context or run identity. This is
  narrower than a claim that all dependencies are unhashed: ordinary site-packages, including
  unowned files, already have content/membership hashing.
- **Proposed correction:** include every actual analyzer-readable root's content and membership
  in the context, or enforce immutable materialization of the whole importable pinned tree and
  reject extra members. Decide once at acquisition/context; do not hash only selected files again.
  A deliberately different acquisition policy needs ADR + DESIGN.
- **Closure:** selected example importing an unselected helper; edit and previously-missing-helper
  addition must change identity or be refused before extraction. Retain relocation invariance and
  compare facts with clean recomputation. Runtime reproduction remains `not_run`.

<a id="F08"></a>**F08 — The model deployment and the hashed embedding spec have separate authorities.**

- **Owner / principles:** spec / controlled launcher / endpoint binding; FP-01/02/04,
  DP-01/09/15/24; A1/A2, G1/G7.
- **Evidence:** `specs/embedding/qwen3-embedding-8b.json:1` owns revision, tokenizer revision,
  engine and dtype; `justfile:89–92` repeats launch values. Rust hashes all fields
  (`embed.rs:27–56`) but sends/checks model name plus vector properties
  (`lctx-embed/src/lib.rs:26–49,70–95`). Python independently loads the packaged declaration
  and accepts the endpoint response under it (`embedder.py:140–167`). Generation/spec hash
  equality (`generation.py:421–430`) compares declarations, not the running deployment. **Interface-checked**; current
  launch values agree, and no assertion is made that the active service is misconfigured.
- **Consequence:** an ordinary model revision upgrade needs matching edits to independent
  configuration. Another endpoint advertising the same model name can be accepted under the
  declared old/new revision without establishing which deployment produced the vector.
- **Proposed correction:** derive controlled launch arguments from the owned spec; declare the
  supported endpoint/deployment contract. Either bind a verified deployment identity or explicitly
  limit the claim to the operator-controlled launch. Avoid an unneeded provider registry or a
  claim that a remote service can be verified solely from its model-name response.
- **Closure:** revision change moves launch config and cache identity together; mismatched endpoint
  identity is refused or expressly outside supported guarantees. Preserve existing request and
  vector-shape controls. Benefits to change locality remain Proposed until implemented.

**Applicable foundations:** FP-01/02/03 are violated at the duplicated access/cache/serving
contracts (F02/F03/F05/F06/F08); FP-04/05 at lost verdicts, string semantics and incomplete
identity (F02/F04/F07/F08); FP-06 at interpretation that requires knowledge across those owners.
Within the kernel, private biodivine handles, typed refusal and repository IDs satisfy useful
parts of FP-01/02/04. The ordinary Delta publication/reader path and deterministic lookup are
sound inspected mechanisms. Neither observation cancels the violations.

## 8. Library fit and total complexity

Workspace pins checked: biodivine-lib-bdd **0.6.3**, fcars **0.2.2**, bitvec **1.0.1**.
Ascent **0.8.1**, datafrog **2.0.1**, OxiDD **0.12.0**, z3 **0.21.1** / z3-sys **0.13.1**
are skill candidates, **not workspace dependencies**. No pin or feature was changed.

| Library / owner | Follow-up choice and fit | Required qualification / burden |
|---|---|---|
| biodivine / kernel | **Expand bounded decisions, effective-support normalization, and cube restriction.** Keep structural IDs and one union vocabulary. F01 below the library layer needs an aggregate preparation contract | `check_binary_op` bounds visited nonterminal pairs, allocates traversal state, and returns `None` on its cap. Transfer/hash/preparation costs still exist. Support shrinking preserves sorted order; candidate cofactor requires existing reconstruction gate |
| OxiDD / possible kernel replacement | **Defer migration**, not a blanket rejection of shared managers. There is no measured production case for a switch | Persisted sharing does not supply in-memory sharing (F01). Conversely `OutOfMemory` is an operation failure, not evidence the whole manager is poisoned. A future comparison must include manager capacity/lifecycle, GC behavior, total retained memory and stable-ID adapters; the skill's pressure/GC unknowns remain |
| z3 / primitive theory | **Keep deferred.** Exact finite literal evaluation for source F04 belongs locally; Boolean algebra stays in the BDD kernel | Escalate for a named query whose correctly linked theory constraints are impractical/inexpressible in the bounded finite lowering, not merely because "two places" appear. Require alias/value stability, supported Python arithmetic/operator semantics, sound translation, timeout→unknown, evidence, and native runtime qualification. No full Z3 capability catalog is an adoption rationale |
| Ascent / summary rules | **Time a bounded comparison after ARC-02/03**, before committing to repeated family-specific machinery | Pinned `ascent_macro/source/ascent_codegen.rs:169–208` implements `generate_run_timeout`; checks occur between generated SCC iterations, not within an expensive rule round. Timeouts do not yield deterministic budget outcomes. Define semantic tuple/lattice keys separately from proof paths: hashes identify witnesses but do not bound the number of recursive witnesses. A lattice trait alone does not prove finite-height convergence |
| datafrog / same comparison | **Retain as a credible candidate** if explicit deltas/joins and deterministic iteration ownership fit the contract | `Iteration::changed`, variables/relations and keyed joins provide semi-naive mechanics with explicit control; runtime-authored rules are not a prerequisite. Application still owns stratification, transfer, witnesses and work caps. It is not automatically superior to a small SCC worklist either |
| fcars / oracle | **Keep dev-only oracle.** Compare unordered concepts/closures, include sparse >64-attribute controls if extending bitset coverage | No automatic recommendation to replace production NextClosure. Concept-set agreement does not test FCA attribute fidelity, provenance, implication rendering or F04 |

For the Ascent comparison, use the **same semantic state and refusal contract** for Ascent,
datafrog and the smallest SCC implementation: identity/call variants, branch conditions,
acyclic and recursive bases, parallel proof paths, dynamic targets, contradiction, join/cardinality
limits, deterministic shuffle results and specific refusal propagation. Measure compilation and
execution only after semantic parity. Do not key recursion solely by an ever-growing proof hash;
choose bounded canonical witnesses or a separated witness relation according to the product claim.

Current Context7 discovery found Ascent `/s-arash/ascent` and its
[timeout contract](https://github.com/s-arash/ascent/blob/master/_autodocs/api-reference/generated-program.md).
The exact 0.8.1 macro source independently confirms the relevant mechanism. Two biodivine names
returned unrelated behavior-driven-testing packages; datafrog returned DataFog. Those misses are
not evidence of missing capabilities. Pinned offline corpus/briefs own the API claims here,
including `reason.fixpoint-choice`, `reason.bdd-implication`, `reason.oxidd-allocation` and
`reason.smt-escalation`; no mutable upstream example is treated as pin qualification.

**Other library placement:** preserve intentional Arrow contracts and DataFusion relational
construction. Arrow IPC is a useful transport candidate once a typed bundle decoder is owned;
it does not select semantic support policy. Delta's insert-only merge remains the right primitive
for F06; read its committed result. No new library is required for F02–F08. Generic mechanisms
cannot supply the missing domain contracts.

## 9. Alternatives and tradeoffs

| Alternative | Locality / composition | Cost and risk | Judgment |
|---|---|---|---|
| Implement source recommendations literally | Some good local BDD improvements, but new engine/IPC choices leave lost facts and claims untouched | Would retain alias-access false negatives, verdict/citation loss, and shared cache policy split | Revise before scheduling |
| Correct semantic seams using existing modules | One normalized access relation, typed facet/attribute values, support closure, one cache-fill result, complete context identity | Contract/schema migrations where meaning changes; some more explicit data, fewer independent classifiers | Preferred target; benefits Proposed, evidence above identifies concrete need |
| Adopt OxiDD/z3/Ascent broadly | Changes machinery across kernel/native/build lifecycle | Does not fix provenance/coverage/rendering; adds capacity/native-runtime/compile obligations | No blanket adoption |
| Minimal immediate corrections | Preserve verdict/citation fields; stop unsupported no-read positives; reconcile committed vectors; add aggregate admission | Smaller first changes while richer typed attributes/closure are developed; cannot claim full closure early | Viable dependency-ordered start, with precise remaining limitations |

No new service locator, provider registry, generic workflow framework, review gate or findings
register is justified. Modules/functions with explicit inputs are sufficient owners.

## 10. Verification and uncertainty

All runs below are dated **2026-09-25**. Evidence folders contain reproducible harnesses and
their scope. Tests/probes do not imply fixes landed.

| Command / inspection | Outcome | What it establishes / limit |
|---|---|---|
| `uv run --no-sync python -B docs/design_review/evidence/2026-09-25_library-fit-followup-kernel/run.py` | `passed` | Actual source BDD decision refusal, support-history mismatch, and shared-catalog expansion counts; no RSS or production-native load measurement |
| `uv run --no-sync python -B docs/design_review/evidence/2026-09-25_library-fit-followup-flow/reach_probe.py` | `passed` | Unchanged reach body produces query-order-dependent transfer variants with acyclic control; narrowed row inputs, no extraction/publication in this probe |
| `uv run --no-sync python -B docs/design_review/evidence/2026-09-25_library-fit-followup-flow/literal_probe.py` | `passed` | Real provider extraction + DataFusion derivation + public flow-model transformation produce qualified/alias no-read defect; bare/direct controls; no published or served generation |
| `uv run --no-sync python -B docs/design_review/evidence/2026-09-25_library-fit-followup-serving/probe.py` | `passed` | Current Python loses existing fixture facet verdict/citation; stored input generation is named in evidence, not freshly compiled |
| `git diff --check`; `uv run --no-sync python -B -` (read-only local Markdown link/anchor inspection); `git check-attr filter -- …/raw/…` | `passed` | Review/evidence/STATUS links and whitespace checked, eight finding anchors, STATUS within 60 lines; raw probe outputs match existing LFS policy |
| Pinned library / source inspection | `passed` as inspection | API and selected execution paths read; recommendations remain Proposed and unmeasured |
| Embedding race/token control; corpus helper edit/addition; RCA candidate end-to-end; engine bakeoff | `not_run` | Proposed focused closure cases, not substituted by source confidence |
| `just test-all`; `just pilot`; clean-wheel/native integration; Q01/Q03/Q05/Q09; structured evaluation | `not_run` | Review-only scope; integrated Stage 3 acceptance remains outstanding |

**Examined without an additional finding:** shared node validation rejects malformed closures;
condition handles do not escape as persistent IDs; exact-input results explicitly remain path-local;
`find_operations` distinguishes open from absent; a process loads one generation; deterministic
capability hydration does not perform a second search; ordinary publication validates before its
manifest append and snapshot reads pin versions. Existing ARC-01–03 remain unresolved, not hidden
by these clean paths.

**Not promoted:** a generic assertion that the entire dependency environment is unhashed; a
production publication bypass merely because a low-level append helper is public; a Pass C
ordering defect solely from input shape; an OxiDD speed/memory advantage without measurement;
a Python false summary solely from the reach cutoff; or a byte/token failure rate without a live
tokenizer. Shared same-library source-workspace concurrency and directory-symlink inputs are
further leads, not established findings in this review.

## 11. Authority changes and dispositions

**Transferred 2026-09-25.** This review's F01–F08, the source review's F01–F16 and ARC-01–03 are scheduled in the forward plan; [its §6](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition) is their single current disposition owner. The table below is this review's dated recommendation, not current status.

| Source | Disposition / owner | Required decision or reopening trigger | Closure / route |
|---|---|---|---|
| This F01 | **Deferred** — kernel/native preparation | Before raising catalog limits or extending summary catalogs | Aggregate preparation/retention contract + shared/disjoint controls; ADR if persisted refusal/meaning changes |
| This F02 | **Deferred** — operation response | Before treating lookup facets as reliable behavioral facts; next serving slice | Preserve per-value verdicts; response contract revision and protocol control |
| This F03 | **Deferred** — bundle/support/rendering | Before a serving evidence-closure acceptance claim; next generation contract change | Bounded support closure + resource parity; ADR/DESIGN for projection contract change |
| This F04 | **Deferred** — attribute schema/analytics/synthesis | Before claiming candidate-derived `+rca` behavior; next FCA/RCA extension | Typed attribute meaning + candidate/definite controls; ADR/DESIGN, reviewed migration if schema changes |
| This F05 | **Deferred** — extraction/core access normalization | Before relying on field/global no-read claims; next negative-premise slice | Shared resolved access contract + equivalent-spelling real-provider cases and publication trace |
| This F06 | **Deferred** — embed/cache workflow | Before embedding-cache conformance or kNN replay claims; next cache/view slice | Shared admission/committed readback, race/cap controls; implementation inside existing cache contract |
| This F07 | **Deferred** — acquisition/context | Before hermetic corpus/run-identity acceptance; next acquisition change | Full readable-input closure or immutable verified tree; ADR if policy changes |
| This F08 | **Deferred** — spec/launcher/endpoint | Next model/tokenizer/server upgrade or acceptance of another endpoint | One spec-derived launch + declared deployment identity; ADR/DESIGN if endpoint guarantees change |

For source F06/F08/F16 decision changes, follow **current** ADR rules: accepted records are
immutable except lifecycle metadata; supersede where a binding decision changes and amend DESIGN
with the new decision. This review does not alter accepted architecture or repair contradictory
historical amendment practices by adding another amendment.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence |
|---|---|---|
| A1 Localize change | **violated** | Equivalent access spellings and ordinary RCA label/model revision changes require knowledge of separate classifiers/configurations (F04/F05/F08) |
| A2 Encode meaning structurally | **violated** | Verdict and modality erased, evidence closure truncated, cache value authority split, context contents omitted, aggregate hydration omitted (F01–F08) |
| A3 Extend through composition | **violated** | Renderer and cache-fill additions reimplement/omit meaning rather than consuming one complete contract (F03/F05/F06); ARC-02/03 remain prerequisites for composable summaries |

**Bounded decision: Revise.** Accept the source review's direction to improve existing BDD APIs,
repair cyclic propagation, and keep fcars independent. Qualify its engine/transport choices and
broaden remediation to the semantic contracts above. No positive library-fit result offsets
the fidelity and evidence failures.

**Enclosing architecture:** needs revision for the named implemented scenarios; Stage 3 remains
functionally incomplete and unqualified. This is a dated design assessment, not release acceptance.

| Priority | Responsible component / next action | Source findings |
|---|---|---|
| 1 | Access normalization and serving contracts: preserve actual reads, value verdicts and claim citations; protect negative claims with real-provider controls | This F05, F02, F03; original F09; existing ARC-01 |
| 2 | Repair reach fixed point and establish explicit summary inputs/refusals; fix actual BDD decision/support behavior and aggregate preparation limits | Original F05/F01/F02; this F01; ARC-02/03 |
| 3 | Restore corpus input identity and one canonical cache-fill result/admission path | This F07/F06 |
| 4 | Preserve typed RCA attributes before extending that optional pipeline; unify deployment configuration on next model upgrade | This F04/F08 |
| 5 | Compare recursive engines on the settled contract; add literal reasoning and independent Boolean/Python/serving checks, then resolve remaining source-review items proportionally | Original F03/F04/F06/F07/F08/F10–F16, with §7.1 qualifications |

The next planning action is to reconcile these stable source IDs into the existing forward plan,
preserving ARC ownership and the distinction between proven defects, proposed mechanisms and
unrun closure checks. Implementation and integrated acceptance were not performed in this review.
