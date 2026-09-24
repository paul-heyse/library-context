# Design review: the behavioral-model pivot, Stage 0 ADR set (standard)

**Date:** 2026-09-24 · **Depth:** standard. ADR-0021 supersedes ADR-0004, ADR-0022 changes §B5 and
§B10, and the ADR-0010 amendment changes §B13 and §B14, so a `standard` review is owed before
acceptance (ADR-0001). · **Mode:** document review of the ADRs and the DESIGN amendments. Code is
read only where a claim cites it or depends on it.
**Target:** the working tree on top of `8d4c442`:
- `docs/adr/0021-behavior-model-scope.md` and `docs/adr/0022-behavior-model-semantics.md` (new,
  `proposed`);
- the 2026-09-24 amendments appended to ADR-0002, 0005, 0010, 0011 and 0020;
- ADR-0004's status change to `superseded`;
- `git diff -- docs/design/DESIGN.md`.

**Reviewer:** `design-reviewer` subagent (fresh context). · **Author:** the session executing
`docs/plans/behavioral-model-pivot-plan_2026-09-24.md`, taking every D-1 to D-9 as the plan
recommended (deviation log B1).

## Answers to the five questions

1. **Do the gate-level gaps close at the design level?** Partly. Three close:
   - **G1:** one registry decides membership, and discovery only nominates;
   - **G1:** predicates are materialized in Rust;
   - **G1:** the superseded decisions are named.

   Two more close in substance:
   - **G2:** a verdict lattice and a runtime view now exist;
   - **G6:** a view is part of the input's identity.

   New or remaining gaps keep G1 and G2–G7 open:
   - **G7.** The "whole public surface" claim sits beside an unchanged subsystem that bounds what
     Pass B follows (F1).
   - **G2.** Stage 1's control-fate vocabulary has no fate for most of what can happen to a
     parameter, so "no read" absorbs the rest (F2). The lattice contradicts itself on budget cuts
     and opaque conditions, and its refutation premise cannot be written as a rule (F6).
   - **G1, documentary.** The spine still names gold retrieval as the keep metric in two places.
     The freeze of the code parameters answers to a superseded ADR (F3).
   - **G3/G5.** `find_operations`' `complete`, its list of unknown operations and its cursor are
     undefined (F8).
   - **G4/G6.** The identity of the registry and the flow provider is unstated (F9). The query
     side of a view is unstated (F10).
2. **Do the new sections state current truth correctly?**
   - **Labels:** yes. Every new clause is `Proposed`, and the two brief tools are marked
     `Implemented`.
   - **Superseded text:** mostly kept as history, with three exceptions (F3):
     - §1.5's live brief invariants now sit under the "kept as history" heading;
     - §9.8's keep-rule bullet stands unrevised above the bullet that overrides it;
     - §11.2 and §13 give the LanceDB deferral two different triggers.
   - **§1.1 against §11.3/§B13:** they agree, except the Python condition-evaluator twin. §B13
     permits it, but no tool takes a condition, so it contradicts nothing yet and serves nothing
     (F12).
   - **§3.9's verdict rule is not enforceable as written.** A predicate's "region" is undefined.
     No predicate declares "the kinds [of boundary] the predicate names". And `boundary_reason`
     has no value for the dynamic access that §3.9's own example turns on (F6).
3. **Where would two implementers diverge?**
   - **Condition normal form (F5).** It has no polarity, no disjunction, no encoding for places or
     literals, and it has two place grammars. Measured on the pilot: 102 of 522 raising `if`s need
     a negated atom and 23 are disjunctions.
   - **Opaque conditions and budget cuts (F6).** Whether an opaque condition's record is
     `conditional` or `unknown`, and whether a budget cut is `unknown` or `not_analyzed`.
   - **The completeness premise (F6).**
   - **Stage 1's "no read" (F2).**
   - **`complete` (F8).** Read as "not truncated" or as "no unknowns".
   - **View identity on the query side (F10).**
   - **Which view governs the call graph under `TYPE_CHECKING` (F11).**
4. **Over-built for its stage (DM-58)?**
   - **The Python twin** has no consumer (F12).
   - **ContextVar places** land in Stage 2, though their questions are Stage 5's.
   - **SKOS per-language checks** for an English vocabulary of 20–40 concepts.
   - **Two reviews at one point:** a stage review and an increment review coincide (F14).
   - **The larger structural point (§8).** ADR-0022 decides Stage 2–4 semantics before the D-2
     spike and before the condition statistics exist, and that is where its gaps are. Leaving
     ADR-0022 `proposed` until Stage 2 starts is the simpler route.
5. **Does ADR-0021 inherit ADR-0004 correctly?**
   - **The pilot:** yes.
   - **The review cadence:** yes, though AGENTS.md:122 and ADR-0001's pointer still cite
     ADR-0004.
   - **The evaluation-only gold rule:** yes. Its keep-metric role changed, but §1.4 and §12(b)
     were not updated.
   - **The freeze:** only `analytics.toml`.
     - The code-parameter freeze (communities, PageRank, FCA, kNN, `selection::Params`, the
       variant policies) is not restated.
     - Nor is the disclosure clause ("naming the gold its author has seen").
     - `eval/gold/analytics-freeze.json` still makes every edit "an ADR-0004 amendment" to a
       record that is now superseded (F3).
   - **The "curated subset" of briefs** and §10.4's manual review "from increment 3" are neither
     retired nor rescheduled (O4).

---

## 1. Decision and scope

**Proposal.**
- **ADR-0021:** the product becomes an evidence-carrying behavioral model of a pinned library's
  whole public surface. Briefs become one rendering, and the increments are remapped onto the
  plan's Stages 0–5.
- **ADR-0022:** the model's semantics:
  - a stated runtime flow abstraction over bounded places;
  - closed conditions with three-valued compatibility;
  - five verdicts;
  - models as data;
  - materialization at compile time.
- **Amendments:**
  - declared extra dependency families (ADR-0002);
  - "a named consumer in the served model", with no solver (ADR-0005);
  - five tools, a pyarrow executor, `FORMAT` 3 and embedding views (ADR-0010);
  - dominators and SCC summaries (ADR-0011);
  - brief retrieval no longer decides, and the deletion exit is paused (ADR-0020).

**Status.** Every new claim is **Proposed**. `evidence: Proposed` on both new ADRs is correct.

**Affected revisions.**
- Working tree on `8d4c442`.
- The pilot measurements below are on snapshot `ec03626f63304c3c156dd87c727a9ead`, the deep
  review's snapshot.

**Observable outcome.** An agent can ask about any public operation. It gets:
- exhaustive answers over materialized rows where the analysis is complete;
- ranked discovery elsewhere;
- a named `unknown` where the analysis stopped.

**Baseline.** ADR-0004's compiler: 20 briefs, with Pass A–C findings for 20 of 1,534 public nodes
(deep review, Measured).

**Supported scope and non-goals.**
- **Stage 1:** the `behavior` family, per-callable fates, the operation catalog, `FORMAT` 3,
  `get_operation`, `find_operations`, `search_operations` and the source-body view. This is the
  in-scope behavior for the next step.
- **Stages 2–5** are stated as Proposed targets.
- **Non-goals** (§1.3): the caller's code, cross-version comparison, a solver, query-time
  traversal, native bodies, a general ontology, and points-to analysis.

**Constraints.** One operator (ADR-0001). Programmatic only (§B11). Evaluation is structured and
qualitative, with pre-registered targets.

### Method and coverage

**Read in full.**
- **The standard:** the skill, the charter's framework (§A–§H), the directive, the ADDENDUM,
  REVIEW_REFERENCE and the template.
- **The motivating material:** the motivating deep review, the plan and the deviation log.
- **ADRs:** 0021 and 0022; 0004 and 0001 in full; the appended amendments of 0002, 0005, 0010,
  0011 and 0020 (their accepted bodies are unchanged, which `just adr lint` enforces).
- **DESIGN:** the whole DESIGN diff, and these sections:
  - §1 in full;
  - §B1–§B14;
  - the §3.2 behavior rows;
  - §3.4.1 L720–734;
  - §3.5, §3.6, §3.7 and §3.9;
  - §4.2.4;
  - §9's opening, §9.2's head, §9.8 and §9.9;
  - §10 up to §10.2, and §10.4;
  - §11, §12 and §13.

**Code and data read myself (only where a claim depends on them):**
- `crates/cpg-core/src/analyze.rs:1100-1219` (the seed loops and the subsystem mask), plus `:523-530`
  for the mask;
- `crates/cpg-schema/src/public.rs`: the root filter at L52; no module-prefix filter (grep);
- `crates/cpg-schema/src/flows.rs` at HEAD, its module contract at L1–43;
- `crates/cpg-schema/src/rules.rs:302-305` (`semantic:one-embedding-spec`);
- `specs/embedding/qwen3-embedding-8b.json`;
- `libraries/fastmcp/analytics.toml`;
- `eval/gold/analytics-freeze.json` (the `adr` and note fields);
- `scripts/adr.py` (its status rules);
- `eval/heldout/README.md`, the head only. **`tasks.json` was not opened:** it is sealed.

**Measured myself** (2026-09-24; read-only; nothing imported from the library):
- **`lctx query` on `ec03626f`.** Of 1,534 public nodes, **1,107 are declared outside the 16
  subsystem prefixes** and 427 inside. The test is a qualified-name prefix match against
  `analytics.toml`'s `module_prefixes`. `reference_resolutions` holds 132,608 rows.
- **A stdlib-`ast` script over `build/envs/fastmcp/.../site-packages/{fastmcp,fastmcp_tasks}`:**
  - **Raising `if`s.** 522 `if`s raise directly in their body. Of them:
    - 186 are conjunctions of §3.9's positive atoms;
    - **102 need a negated atom** (`not x`, `!=`, `not in`, `not isinstance`);
    - 32 are ordering comparisons;
    - 23 are disjunctions;
    - 179 are other tests.
  - **Stores to `self`.** 285 direct `self.x = <parameter>` stores, covering 283 parameters in
    131 functions. **252 of those parameters appear in no call argument of their function.**
  - **`TYPE_CHECKING`.** 75 `if TYPE_CHECKING:`. 2 bind a name in `else`
    (`client/client.py:86`, `client/transports/inference.py:22`, both `FastMCP = Any`), and it
    is used only in annotations.

**Checks run.**

| Command | Outcome |
|---|---|
| `just adr lint` | **passed** (`adr lint: ok (22 records)`) |
| `just lint-agents` | **passed** |
| `just check`, `just test-all`, `just pilot` | **not_run**. This is a document review, and the tree holds uncommitted Stage 1 code outside the target (`crates/cpg-schema/src/behavior.rs` untracked; `flows.rs`, `findings.rs`, `codebook.rs`, `lib.rs` modified) |

**Not reviewed.** The in-flight Stage 1 code. The plan's tooling versions. The ty and Lance probe
facts (the deep review's).

**Guarantees not attacked, so asserted:**
- that petgraph 0.8.3's `simple_fast` over `Reversed` gives post-dominators with a virtual exit
  that joins exceptional exits;
- that the SCC fixpoint terminates under "path depth ≤ k and condition size ≤ c";
- that `ascent` and Pysa fit as the ADRs say;
- every performance statement. None is claimed as measured, which is correct.

---

## 2–4. Authority, contracts and derivation (compressed)

**Authority map**, reconstructed from the ADRs and DESIGN. ⟂ marks a cell the set leaves
undecided.

| Concept | Identity | Authority | Revision boundary | Update path | Derived | Gap |
|---|---|---|---|---|---|---|
| The universe | `public_paths` under `$roots` | ADR-0021; `analytics.toml` roots | Snapshot | ADR amendment | `operations`, coverage | ⟂ the subsystem's role in per-callable analysis (F1) |
| Stage 1 behavior relations | Today's declared SQL (`cpg_schema::flows`), persisted | `cpg-schema`; Stage D | Snapshot + `compiler_digest` | Derivation | `behaviors` file, `control_fates` | ⟂ the fate vocabulary (F2) |
| `control_fates` vs seed findings | (operation, parameter, formal) vs finding ids | The Pass B kernel, run twice | Snapshot | Recompute | `get_operation`, briefs | ⟂ one derivation or two (F7) |
| Flow IR | Syntax-anchored | The provider (spike) | ⟂ provider identity (F9) | Extraction | Behavior v2 | — |
| Places | §3.9 list; §9.9 access paths | Two grammars | — | — | Conditions, summaries | ⟂ one encoding (F5) |
| Conditions | "normal form" id | §3.9 | Snapshot | Derivation | Compatibility | ⟂ polarity, disjunction, literals (F5) |
| Verdicts | Codebook `verdict` | `cpg-schema` | Append-only | Codebook append | Answers | ⟂ relation to `modality` and `evidence_status`; budget; opaque (F6) |
| Models | TOML, append-only ids | `cpg-schema/models` | `compiler_digest` ✓ | Operator | Summaries | Gold rule stated ✓ |
| Registry and vocabulary | Append-only ids; definition digest | `cpg-schema` | ⟂ digested into what (F9) | Operator | Membership, lookup | ⟂ gold rule (F9) |
| `concept_members` | Materialized | Definitions only (rule) | Snapshot | Recompute | Filters | — |
| Views | Template id and version in `input_hash` | §B14 | `spec_hash` + `input_hash` | — | Vectors | ⟂ query instruction (F10) |
| Keep rule | §9.8 | The ADR-0020 amendment | — | — | Techniques kept | ⟂ criterion (F4) |
| Analytics freeze | `analytics-freeze.json` | **ADR-0004 (superseded)** | — | "An ADR-0004 amendment" | `just gold` | ⟂ live authority (F3) |
| Question sets | `eval/behavior/*.toml`, append-only | Pre-registered | Commit | `supersedes` | Assessment | File absent (F13) |

**Invariants.**

| Invariant | Enforcement boundary | Failure behavior | Status |
|---|---|---|---|
| `refuted_under_model` only in a complete region | `semantic:refuted-needs-complete-region` (plan 2.5) | Validation rejects the snapshot | **Not writable as stated** (F6) |
| Membership only from definitions | Rule: a member cites a definition digest and a witness; test: FCA writes none | Validation | Proposed; decidable |
| No SQL string at serve time | ast-grep `no-string-sql-in-lctx-mcp` (plan 1.6) | Scan fails | Proposed; decidable |
| One spec per snapshot | `semantic:one-embedding-spec` (exists: `count(embedding_specs) > 1`) | Validation and the builder | Tested; the "per model" wording diverges (F10) |
| Every public callable has behavior coverage | `semantic:behavior-covers-public` (plan 1.3) | Validation | Proposed; what `complete_under_stated_model` means for a fate is ⟂ (F2) |
| Predicate semantics only in Rust | The twin exception, held to `conditions.json` | Corpus test | The exception has no consumer (F12) |
| Runtime view | `flow_shapes` `TYPE_CHECKING` case | Test | The CPG layers keep the checker view (F11) |

**The absence lattice (DM-08).** These states collapse or conflict as written:
- A budget cut is `not_analyzed` in §3.9 and `unknown(budget_reached)` in §9.9 (F6).
- An opaque condition's record is `conditional` or `unknown` depending on which document you read
  (F6).
- "No read" covers "not read at a call argument" as well as "not read at all" (F2).
- A Stage 1 facet has no verdict, so "unknown" is undefined for it (F8).
- A dynamic access has no `boundary_reason` of its own (F6).

**Stage graph.** Unchanged through publication. The new stages add families (`behavior`, `flow`)
and analysis tables (`summary_*`, `concept_members`), and each ADR names its producer. Two effects
are undeclared:
- the serve-time twin's version (F12);
- the flow provider's identity (F9).

---

## 5. Journeys

**Ordinary extension: a Stage 1 facet (`raises_directly`).**
- It takes one declaration in `cpg-schema` (the facet vocabulary is served data, per plan 1.6),
  its materialization, and a known-answer case.
- Locality holds only if the pydantic `where` model validates facet names against the served
  vocabulary, not a per-facet Python type. That is not stated; §9 notes it.
- The facet must also declare what it states: "raises directly", not "raises" (F8).

**Meaningful change: an operator edits a concept's `altLabels`.**
- A models file edit moves `compiler_digest` (§9.9).
- A registry edit has no stated identity input, so `lookup_concepts` answers change under an
  unchanged snapshot identity (F9).

**Boundary: a condition reaches Python.**
- No tool takes one. `find_operations`' `where` is "typed facet filters; concepts from Stage 4"
  (§11.3:3147).
- The twin that ADR-0022 and §B13 provide for this crossing is never called (F12).

**Interruption: a 40-method SCC exceeds its summary budget.**
- §9.9 says `unknown(budget_reached)`; §3.9 says "cut by a budget" is `not_analyzed`.
- `find_operations` lists the operation either way. But whether a caller should retry with a
  larger budget, or treat the operation as out of scope, depends on which verdict it gets (F6).

**Adversarial, on the pilot:**
- **a. A parameter stored to a field.** Take an `__init__` parameter stored to `self._x`: 252
  such parameters appear in no call argument (Measured).
  - Stage 1's fates are "forwarded, transformed, literal, unfollowed, or no read" (§3.2:605).
  - The only relation that sees parameter reads is Pass B's, and it sees them only at call
    arguments (`flows.rs` L37–40).
  - So the fate is either missing or "no read": the false "accepted but unused" that F3 of the
    deep review warned about (F2).
- **b. `fastmcp.Client.call_tool`.** It is a public path, so it is in the universe.
  - Its module is outside the subsystem, and Pass B follows forwards only `inside` the subsystem
    mask (`analyze.rs:1193`, passed at `:1217`).
  - So its fates stop at its first callee, with no stated verdict for that stop (F1).
- **c. `Settings.get_setting(attr)`.** §3.9 says it makes "never read" `unknown`, but no
  `boundary_reason` names a string-keyed `getattr`. §3.9 L1095–1096 offers
  `unsupported_unpacking` and `unresolved_target` for computed names (F6).

---

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 Authority | **fail** (documentary) | **Resolved at design level:** concept authority (§9.9: one registry; "Discovery nominates, definitions decide"; the member-digest rule), predicate authority (ADR-0022 "Materialized at compile time"), and naming the superseded decisions (ADR-0021; DESIGN's *Superseded* notes). **Fails:** the spine names two keep criteria. §1.4:154 ("the development metric for keeping or removing techniques") and §12:3175 ("the pre-registered **development metric** for the §9.8 keep rule") contradict §9.8:2612–2614 and ADR-0020's amendment. §B4:297 says "a named consumer in the brief" against §9:1979. §9.8:2604–2606's rule bullet is unrevised. The code-parameter freeze answers only to ADR-0004 (`analytics-freeze.json`: "an edit is an ADR-0004 amendment"), which is superseded, and ADR-0021 inherits only `analytics.toml` (F3). **Latent:** `control_fates` and seed findings are two derivations of one fact (F7); there are two place grammars (F5) | F3 (sentences, one ADR-0021 paragraph, one data-file field); F7 decided before Stage 1 lands |
| G2 Semantic fidelity | **unresolved** | A lattice and a runtime view are declared, which closes the deep review's "no lattice". Open: Stage 1's fates have no complete set of outcomes, and "no read" absorbs field stores (F2). Budget cuts are two verdicts, and an opaque condition's verdict is undecided (F6). The refutation premise is not a writable rule (F6). The verdict's relation to `modality` (`potential`) and `evidence_status` is unstated; the plan listed §10.2 for this ADR, and it was not amended (F6). The runtime view covers the flow IR only (F11) | F2 before Stage 1; F6 and F11 before Stage 2 |
| G3 Validity | **unresolved** | Resolved: pydantic inputs, `limit` 1–50, `truncated`, the inherited `ToolError` contract, no recursion at serve time. Open: the `where` operators are not listed, and negation over a facet with unknowns is undefined. The unknown-operations list has no cap. A cursor presented to a different generation is not rejected by any stated rule. `explain` does not say whether premises are one level or a tree (F8) | F8 |
| G4 Hidden behavior | **unresolved** | Resolved: models are digested and authored "Never from `.claude/skills/`" (§9.9:2678–2679); views enter `input_hash`. Open: the registry and vocabulary files' identity, and the gold rule for concepts (the gold *is* a catalog of capability families, ADDENDUM Q15). The flow provider's revision and settings are absent from §B5 and ADR-0022 (the plan 2.3 has them); `compiler_digest` hashes only three crates' `.rs` (§3.4.1:684) (F9). The pre-registered question set that B3 records does not exist at review time, while Stage 1 code is in flight (F13) | F9; F13 before any Stage 1 output is read |
| G5 Consistency and recovery | **unresolved** (pagination only) | The publication and generation protocol is unchanged and passes: `FORMAT` 3's derived indexes sit outside the byte-identical manifest, keyed by the generation (§6.4). Open: `find_operations` takes `library`, not `snapshot_id`, and returns a cursor. A cursor held across a server restart pages into a newer generation, and nothing makes the mix visible (F8) | Bind the cursor to `snapshot_id` and reject a mismatch, or drop cursors (F8) |
| G6 Transformation and reuse | **unresolved** | Resolved: views are template ids inside `input_hash` (§B14:472–477), and the spec's `document_template` is `{text}`. Open: the spec's single `query_task` ("retrieve capability briefs") and the query side of views; "holds per model" against the rule's one spec per snapshot; §11.1:3000's document text and windowing for source bodies (F10). The registry and provider are missing from invalidation keys (F9) | F10; F9 |
| G7 Truthful capability claims | **unresolved** | Per-stage labels are right (§1.1's tool table, §1.5's exit table). But "the whole public surface" (§1.1:29–40; ADR-0021) sits beside "Unchanged by ADR-0021: … the subsystem and analytics config" (§1.4:166–167), and Pass B follows only inside the subsystem (`analyze.rs:1193,1217`). **1,107 of 1,534** public nodes lie outside it (Measured). Stage 2's Q4 needs `fastmcp.settings` and Stage 5's Q8 needs `fastmcp.client`, and the config's rule excludes both (F1). Stage 1 promises "each control's fate and verdict" (§1.1:49; §6.4), while the verdict codebook is plan 2.5's (F2). The replacement keep rule has no criterion, and "communities serve navigation" names no tool (F4) | F1; F2; F4 |

An unresolved gate is not a pass. G1's failure is documentary: sentences and one ADR paragraph fix
it.

---

## 7. Findings

The order follows the skill's severity ranking:
1. correctness and authority on Stage 1's in-scope behavior (F1–F4);
2. the Stage 2–4 semantics ADR-0022 decides (F5–F7);
3. serving, identity and views (F8–F11);
4. proportionality and process (F12–F14).

| # | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | **"The whole public surface" and "the subsystem, unchanged" are both in force, and the analysis follows the subsystem** | DM-04, DM-43, DM-59 · G7 | ADR-0021 Decision: "The universe is `public_paths`. Every analysis runs per public callable". Also "The pilot … with the same subsystem and analytics config ADR-0004 chose". §1.4:166–167: "Unchanged by ADR-0021: the pilot, the subsystem and analytics config". `public.rs:52` filters by `$roots` only. `analyze.rs:523-530` builds the subsystem mask from `in_subsystem`; Pass A takes it (`:1137`), and Pass B follows forwards only `inside` it (`:1193`, `:1217`). **Measured:** 1,107 of 1,534 public nodes are declared outside the 16 prefixes. The config's rule excludes `fastmcp.client`, `fastmcp.settings`, `fastmcp.cli` and `fastmcp.server.auth`. §1.5's Stage 2 exit (Q4: settings) and Stage 5 exit (Q8: the `Client` protocol) live there | Per-callable Stage 1 inherits the mask. For 72% of the universe, fates stop at the first callee outside the prefixes, and no verdict is stated for that stop, so "unfollowed" reads as a library fact. Alternatively, an implementer drops the mask, which changes the 20 seeds' briefs, and hence the frozen config's meaning, without an amendment. Either way, Stage 1's exit ("controls … answered for operations with no brief") is judged on a scope nobody chose | Decide in ADR-0021 and §1.4. **Either** the subsystem keeps only its brief role (seeds, selection, the gold correspondence), and per-callable analysis runs to the release boundary (§3.7's external nodes), **or** the subsystem stays the analysis region, "the whole public surface" is narrowed to it, and a stop at its edge is `unknown` with `scope_boundary`. The first matches ADR-0021's intent and needs no `analytics.toml` edit | **None exists.** Add a test: on `public_shapes` or `analysis_shapes`, a public operation outside the configured prefixes has a fate that follows a forward into another out-of-prefix module (or, under the second option, a `scope_boundary` stop with verdict `unknown`). The planned `semantic:behavior-covers-public` checks rows, not reach |
| **F2** | **Stage 1's fate vocabulary does not cover what can happen to a parameter, and its only negative fate absorbs the rest. Stage 1 promises verdicts that land in Stage 2** | DM-08, DM-42, DM-59 · G2, G7 | §3.2:605: `control_fates` is "forwarded to which formal, transformed, literal, unfollowed with a reason, or no read". The relations Stage 1 persists see parameter reads only as "an argument of an arc whose value reads a name … under one of its parameters' names" (`flows.rs` L37–40, HEAD). §9.2's "never guessed" list includes `**kwargs` passthrough and property access. **Measured:** 252 parameters stored to `self` appear in no call argument of their function. There is no fate for "stored in a field", "returned", "tested only" or "logged only". §1.1:49 and §6.4 promise "each control's fate and verdict" in Stage 1, while the `verdict` codebook and `semantic:refuted-needs-complete-region` are plan Stage 2.5 | `get_operation("fastmcp.FastMCP.__init__")` reports its field-stored settings as "no read", or with no fate. That is a negative claim without the completeness premise ADR-0022 exists to require, published in the first stage that serves behavior. It is the "accepted but unused" error the deep review's F3 named. §3.5:770's "Missing output is not negative evidence" is broken by construction | Make the Stage 1 fates cover every case: add `read_elsewhere` from C3's `reference_resolutions`, which exists (132,608 rows on the pilot): a load of the parameter's binding that no Pass B relation covers. "No read" then means "no load resolves to the binding", a lexical fact with `locals()`/`vars()`/frame access as named boundaries. Land the `verdict` codebook in Stage 1 with `established`, `unknown` and `not_analyzed`, and append `conditional` and `refuted_under_model` with their rule in Stage 2 (codebooks are append-only) | **None exists.** Add a test: a `behavior_shapes` (or `analysis_shapes`) function whose parameter is only stored to `self`, only returned, and only tested gets no `no_read` fate. Add a rule: no `no_read` fate where `reference_resolutions` holds a load of that parameter's binding, with an injected case |
| **F3** | **The spine disagrees with itself on what judges a technique, live invariants are filed as history, and the freeze answers to a superseded ADR** | DM-02, DM-59 · G1; ADDENDUM Q15 | (a) §1.4:151–155 and §12:3174–3175 still make gold hit@5 the keep metric, and §9.8:2604–2606's rule bullet is unrevised above :2612–2616's override. (b) §B4:297 says "Each algorithm has a named consumer in the brief". §B10:413 says a "search projection of published briefs". §11.2:3088–3089's LanceDB trigger ("a few thousand briefs") differs from §13:3250's (">~10⁵ vectors"). (c) §1.5:191 puts "no brief may reach publication by bypassing the analytics" and `semantic:brief-cites-analysis` under "kept as history", although briefs remain (D-6) and the rules still run. (d) `eval/gold/analytics-freeze.json` sets `"adr": "ADR-0004"`. Its notes make every config, parameter, selection and variant edit "an ADR-0004 amendment naming the gold its author has seen". ADR-0021 restates only "`analytics.toml` … editing it needs an amendment to this ADR". (e) Pointers: AGENTS.md:122 (cadence), ADR-0001:75, DESIGN:7 (no line budget), `docs/pins.md:74`, ADDENDUM Q17 ("in the brief") | A session applying §12(b) or §9.8's first bullet re-runs the keep rule on brief hit@5, against ADR-0020's amendment. A reader of §1.5 treats the brief-cites-analysis rule as retired. An edit to `communities::Params` or `selection::Params` has no live record to amend, since the superseded record is immutable, so the disclosure duty (ADDENDUM Q15) lapses silently | (a–c) Sentences: move §1.5's "In addition" block above the history heading, revise §9.8's rule bullet in place, align §1.4, §12(b), §B4, §B10 and §11.2. (d) An ADR-0021 paragraph restating the whole freeze (config, code parameters, selection, variant policies, disclosure clause); set the freeze file's `adr` to ADR-0021. (e) Pointer lines in ADR-0001 and AGENTS.md; the ADDENDUM row | **Test (new, cheap):** `check_gold.py` (or a pytest) asserts that `analytics-freeze.json`'s `adr` names an **active** record. The freeze itself: `just gold` and `the_parameters_match_their_gold_freeze` (exist). The sentences: prose, no mechanical oracle |
| **F4** | **The keep rule's replacement is an intention with no criterion or exit, and one new consumer names no tool** | DM-58, DM-59 · G7 | §9.8:2612–2614: "A technique is judged by its consumer in the served model, through the pre-registered behavioral question sets". There is no comparison, threshold or stage. The deletion exit is "paused". ADR-0020 stays `proposed` "until those consumers are evaluated", with no evaluation defined. §9.9 and ADR-0011's amendment say "communities serve navigation", but none of the five tools (§11.3:3142–3150) takes or returns communities. §9:1979 requires "a named consumer in the served model: a tool's output or a brief" | Communities, FCA, kNN, PageRank, RCA and the extra layers stay compiled, frozen and tested, with no condition under which any of them is ever judged or removed. This is the retained-machinery case the ADR-0020 review raised (its F5), now without even the brief metric | Pre-register the rule now, before any behavioral output: per technique, the stage whose tool consumes it (views → Stage 1 `search_operations`; FCA facets → Stage 4), the ablation (the stage's packet with and without the technique), and the outcome. For example: "kept if the operator rates at least one pre-registered item better with it and none worse; otherwise removed by ADR at that stage's end". Name a tool for "navigation", or drop it from communities' consumers | **None mechanical.** The stage packet's with/without arms are a record (prose). Say so in §9.8 |
| **F5** | **The condition normal form is under-specified: no polarity, no disjunction, no encoding for places or literals, and two place grammars** | DM-06, DM-15, DM-02 · G2, G1 | §3.9:1102–1107: the atoms are `is None`, `is not None`, `== literal`, `in {literals}`, truthiness, `isinstance(C)`; "a conjunction of atoms in normal form … by a canonical encoding" that is not given. §9.9's paths (`Parameter[name]`, `Parameter[self].Field[f]`, `Global[…]`, …) have no ContextVar or function-attribute form, while §3.9's places do. §9.2's supported predicate (today's `guards`, rendered in briefs) admits `not`/`and`/`or` and comparisons. **Measured** (522 raising `if`s): 186 are positive conjunctions, **102 need a negated atom**, 32 are ordering comparisons, 23 are disjunctions, 179 are other. The path that continues past any conjunctive guard is a disjunction of negations | `x != "sse"` is opaque for one implementer and `¬(x == "sse")` for another. Their condition ids differ, and ids churn when one moves to the other. Journey c (stateless + SSE) is `incompatible` under one reading and `unknown` under the other. Summaries joining two paths have no representation for "either" (condition size ≤ c presumes one). A place written `self.f` in §3.9 and `Parameter[self].Field[f]` in §9.9 is two keys for one place | Decide in ADR-0022 before Stage 2, using the measured distribution: atoms carry a polarity bit; a condition is a bounded set of conjunctions (one row per disjunct, or an id over sorted disjuncts); one place grammar shared by §3.9 and §9.9; type-tagged literals (`1`, `True` and `1.0` distinct), with `x in {a}` normalized to `x == a`; `isinstance` compatibility is `unknown` unless the class order proves disjointness. State how today's supported predicates map (opaque, or translated) | **None exists.** `specs/serving/conditions.json` (planned) plus a Rust unit test over normal-form equality, both with negation, disjunction and literal-type cases |
| **F6** | **The verdict lattice contradicts itself on budget cuts and opaque conditions, its refutation premise cannot be written as a rule, and its relation to the existing vocabularies is unstated** | DM-08, DM-07, DM-06, DM-02 · G2 | (a) §3.9:1121 says `not_analyzed` is "cut by a budget"; §9.9:2692 says "Widening yields `unknown` (`budget_reached`)". (b) The plan's D-5 and the deep review say "opaque atoms are `unknown`"; ADR-0022 says only that *compatibility* is unknown when an opaque atom decides, and says nothing of the record's verdict. (c) `refuted_under_model` requires "the region … holds no boundary of the kinds the predicate names". "Region" is undefined (`coverage` is keyed by release, module or callable, §3.7:844–847), and no predicate declares its boundary kinds. `boundary_reason` (§3.5:755) has no dynamic-access value, while §3.9:1095–1096 names `unsupported_unpacking` for a computed name. (d) `modality` already has `potential` ("holds if some condition occurs", §3.5:750) beside the new `conditional`. May and must are not distinguished. The mapping to `evidence_status` (§10.2) for briefs rendered from behavior is absent, although plan Stage 0.3 listed §10.2 for this ADR | The same budget-cut operation is `unknown` from summaries and `not_analyzed` from membership, and a caller cannot tell "retry with a bigger budget" from "out of scope". The planned rule cannot be one query, so it will be a partial check that passes refutations it cannot see. A `getattr(settings, name)` stop is emitted as `unresolved_target`, indistinguishable from an unresolved call. A forward inside an `if` can be `definite`+`conditional` or `potential`+`established`, so two rows with the same meaning carry different ids | (a) A cut before analysis starts is `not_analyzed`; a stop during it is `unknown(budget_reached)`. (b) One sentence: a record with an opaque atom is `conditional`, with the atom visible. (c) Each predicate kind declares its region as coverage keys (the callable plus its summary's callee closure; the release for attribute reads) and its boundary kinds; append `dynamic_access` (`getattr`/`setattr`/`hasattr` by a non-literal name, `importlib`, module `__getattr__`) to `boundary_reason`. (d) State that verdict and modality are orthogonal (a `potential` fact never yields `established` for a must-claim); give the verdict → `evidence_status` map; add `verdict` to ADDENDUM §3's vocabularies | `semantic:refuted-needs-complete-region` with an injected case (planned; only writable after (c)). The `codebook:boundaries.reason` snapshot moves with the append. **None exists** for (a) and (b): add a `behavior_shapes` budget case asserting one verdict |
| **F7** | **Briefs are called a rendering of the model, but seed findings remain a second derivation of the same facts** | DM-02, DM-23 · G1 | §1.1:37 "Briefs remain as one rendering". §9:2019–2021: "the control fates are computed for every public callable. Seed findings remain as the input to briefs". §11.3's `get_operation` returns both the record and the brief id. §10.1's finding conditions ("source-linked text plus predicate node references") and §9.2's supported predicate are a condition representation distinct from §3.9's | For a seed, `get_operation` shows a fate from `control_fates` and a brief whose `control` and `restriction` assertions come from a separate Pass B run. After Stage 2, a brief says "raises when `timeout < 0`" while the record shows the same guard as an opaque condition. A mask or budget difference shows "forwarded" in one and "unfollowed" in the other. No rule ties them | Select seed findings from `control_fates` and `guards` rows (one computation; briefs render it), or add a rule that each seed's control and restriction assertions agree with its behavior rows | **None exists.** Rule `semantic:brief-agrees-with-behavior`, with an injected disagreement |
| **F8** | **`find_operations`' `complete`, its list of unknown operations and its cursor are undefined, and so is `explain`'s depth** | DM-07, DM-08, DM-14, DM-37 · G3, G5 | §1.1:50 and §11.3:3147: "matches, `complete`, the operations whose answer is `unknown` or `not_analyzed`, `truncated`, next cursor"; `where` is "typed facet filters". There is no operator list, no negation rule, no cap on the unknown list, and no snapshot parameter. §11.3:3150: `explain` returns "rule id, premises and source spans". The plan's "proof height, in the Soufflé style" reconstructs trees at query time, against §1.3's "no query-time graph traversal" | `complete` is read as "not truncated" by one implementer and "no unknowns" by another. Under the first, `raises = ValueError` over Stage 1's direct raises returns `complete = true`, asserting that no other operation raises it. A NOT filter returns operations whose facet is unknown as matches. The unknown list returns up to 1,534 rows beside a 50-row cap. A cursor kept across a restart pages into another snapshot | Define `complete` as "`truncated` false and no universe operation `unknown`/`not_analyzed` for any facet the filter reads". Each facet names what it states (`raises_directly`). Negation never matches an unknown. The unknown list gets its own cap, count and flag. The cursor encodes `snapshot_id`, and a mismatch is the existing `ToolError`; or drop cursors (return `total`, refine `where`). `explain` returns one level of premises per call | Tests: `fastmcp.Client(mcp)` adversarial cases (planned 1.8), extended with a stale cursor, a NOT filter over a facet with unknowns, and an over-cap unknown list |
| **F9** | **Identity and the gold boundary are stated for models only, not for the registry or the flow provider** | DM-31, DM-32, DM-28 · G4, G6; ADDENDUM Q15 | §9.9:2678–2679 digests models and forbids `.claude/skills/` for models. For concepts, only "a definition … compiled to DataFusion SQL and digested" (:2704): digested into what, and what about labels, `altLabels`, `broader` and scope notes, which `lookup_concepts` serves? §B5 and ADR-0022 do not put the provider's revision and settings into `producer_id`/`context_id` (plan 2.3 does). `compiler_digest` hashes `.rs` of three crates (§3.4.1:684), so a new `cpg-flow` crate is outside it. The gold skill is a catalog of capability families with task aliases, and the registry is capability concepts with alt labels | Editing an `altLabel` changes `lookup_concepts` under an unchanged snapshot and run identity (DM-32). Concepts written with the gold's families in view make Stage 4's evaluation circular. A `ty` or builder upgrade reuses an old run's identity | One sentence each in §9.9 and §B5: registry and vocabulary files join `compiler_digest` like models; the authorship rule covers the registry and the vocabulary seeding; whichever provider is chosen, its revision and settings enter the extractor run's identity, and any new crate joins the hashed set | Tests like `every_compiler_source_is_hashed` over registered data files (planned for models; extend to the registry). A skill-path check over models **and** registry files (planned 3.1; extend) |
| **F10** | **Views are identified on the document side only; the query instruction, §11.1's document text and the one-spec wording are not reconciled** | DM-31, DM-32 · G6 | The spec: `"query_task":"Given a coding task, retrieve capability briefs of a Python library that solve it"`, `"document_template":"{text}"`, `"max_document_tokens":2048`. §11.1:2995–2997: the spec hashes the instruction. §11.1:3000: "Document text is the deterministic brief projection". §11.1:3031: "an over-cap document fails the compile". §B14:476 "holds per model", while the rule is `count(embedding_specs) > 1` (`rules.rs:303-305`). Plan tooling: "Qwen3-Embedding-0.6B as a second view" | `search_operations` embeds queries asking for "capability briefs" against source-body vectors. Changing that instruction changes `spec_hash`, which keys every cached document vector, although documents take no prefix, so the whole cache re-embeds for a query-side edit. A source body over 2,048 tokens fails the compile, because §11.1's chunking is by brief parts. "Per model" reads as permitting the 0.6B view the rule refuses | Amend §11.1. Either one neutral query instruction for all views, or the query instruction as part of the view (query side), outside `spec_hash`. Windowing for source bodies. Replace "holds per model" with "one spec (one model) per snapshot; views vary only the rendered text" | Tests planned in 1.7 (distinct keys; a second model refused), plus: a body over the cap is windowed, not refused |
| **F11** | **The runtime view is declared for the flow IR only; the CPG layers it composes with keep the checker's view** | DM-24, DM-13 · G2 | §3.9:1098–1100 and §B5:323: `TYPE_CHECKING` is false. §3.4.1:728–734: the exports seed prefers Pysa's `def`, so "under `TYPE_CHECKING` … the typed facade wins over the runtime body", and the other is `unreachable_in_context`. Summaries run over the Pysa call graph (§9.9), which is the checker's view. §3.9:1099 cites §4.2.4 for "our C3 static-branch marks", but §4.2.4 describes Pyrefly's pruning; the marks are the `static_branch` codebook (§3.5:760). **Measured:** 2 of 75 `TYPE_CHECKING` blocks on the pilot bind in `else` (`FastMCP = Any`, annotation-only) | On a library with a `def` in both branches, `public_paths` names the facade, the fates are computed over a stub body, and every parameter is "no read". A call in a runtime-only region is resolved by Pysa to its checker target, while the flow IR says the name is `Any`. The pilot impact is low; the general impact is not | One §3.9 sentence on where the views meet: behavior attaches to the runtime binding of a public name (the `unreachable_in_context` `def`), or such operations are `not_analyzed` with a reason. Correct the §4.2.4 citation | The planned `flow_shapes` `TYPE_CHECKING` case, extended with a public `def` in both branches (test) |
| **F12** | **The Python condition-evaluator twin has no consumer** | DM-58, DM-02 · — | ADR-0022: "only the condition evaluator has a twin, held to the shared corpus". §B13:455–458 and §3.9:1110 say the same. No tool takes a condition (§11.3:3142–3150). The compile-time consumers of compatibility (membership, summaries) are Rust | A second implementation of a semantic decision is built and kept in step with a corpus, and no request calls it. That is the one exception to the deep review's F12 closure, bought for nothing. If a condition filter arrives later, the twin's version is in no identity, so a changed twin changes answers over a byte-identical generation | Remove the twin from ADR-0022, §B13 and §3.9. Materialize the compatibility that questions need at compile time (e.g. an `incompatible_modes` relation per operation for Q9). Add a serve-time condition filter by ADR when a pre-registered question needs one, with the evaluator's version in the answer | None needed: this removes machinery. If kept: the `conditions.json` corpus in both suites |
| **F13** | **The pre-registered question set that the deviation log records does not exist, while Stage 1 code is being written** | DM-28, DM-59 · G4 | Deviation log B3: "A research subagent wrote `eval/behavior/fastmcp-4.0.5.toml`". At review time (2026-09-24) `eval/behavior/` does not exist on disk and is not in git (`ls`; `git log`; filesystem search). The untracked `crates/cpg-schema/src/behavior.rs` and modified `flows.rs` are Stage 1 work. ADR-0021 and §12:3210–3212 require the set "committed first". The plan's Stage 0 exit requires it | If a Stage 1 compile's `control_fates` or a test snapshot is read before the set is committed, Stage 1's exit is graded against targets that may encode the output. B3's claim is unbacked as written | Commit the set before any Stage 1 output is read. Record its commit in the Stage 1 packet. Correct B3 to say what exists | **None exists.** A check in the structured-eval packet script that the question set's first commit precedes the judged snapshot's compile (recipe tier) |
| **F14** | **Machinery ahead of its consumers** | DM-58, DM-57 · — | ContextVar places are in Stage 2 (§3.9, ADR-0022), but the deep review's Appendix B puts every ContextVar question (Q3's ContextVar half, Q6, Q12) in Stage 5. §9.9's SKOS rule "one `prefLabel` per language" serves a single-language vocabulary of 20–40 authored concepts. The plan's `lookup_concepts` stack (bm25s, PyStemmer, Qwen3, RRF) serves 20–40 items. Stage and increment reviews coincide at Stages 1, 3 and 5 (§1.2:66–77) | Stage 2 carries defs, uses, fixtures and parity cases for a place kind that nothing reads until Stage 5. Retrieval machinery ranks a list an agent could read whole. Two reviews are written for one point | Land ContextVar places with Stage 5's framework models. Drop the per-language rule until a second language exists. Have `lookup_concepts` return every concept (with scope notes) until the vocabulary exceeds a size stated in advance. "Where a stage end is an increment end, the increment's review replaces the stage's" | None needed: this removes machinery |

**Observations** (not findings):
- **O1. ADR-0004 is `superseded` while ADR-0021 is `proposed`.** `just adr supersede` does this at
  creation (`scripts/adr.py:369-370`), and `adr lint` accepts it. Until acceptance, §1's authority
  is a proposed record, and Stage 1 is being implemented on it. That is why this decision gates
  Stage 1.
- **O2. The held-out set was not built for this rubric.** It was authored as agent tasks with
  `expected_calls`, `expected_controls` and `limits` (`eval/heldout/README.md`; `tasks.json` not
  opened). ADR-0021 and §1.5 make it "the held-out structured evaluation" without saying how tasks
  map onto per-item targets and the five-grade rubric. Decide before increment 5, without opening
  the set.
- **O3. §B1 forbids what §B5 offers.** §B1:251 says "No second parser or type checker", while §B5
  offers `ty_python_core` as a provider. ADR-0022's Consequences name only the ADR-0002 family
  exception. One clause in §B5 should say that choosing `ty` needs the ADR-0012 amendment (§B1,
  §B8) first. The plan's Stage 0.3 table already expects it.
- **O4. "A curated subset" of briefs (§1.1:37, D-6) is undefined.** §10.4's "From increment 3,
  each brief gets one manual review pass" is neither retired nor rescheduled, although increment 3
  now closes with Stage 1. §B6's "ADR-0020, to be written in slice 3.5" is older staleness: that
  number became the keep-rule record.
- **O5. STATUS.md is dated 2026-09-23** and says increment 3 is at slice 3.2, while §1.2 says 3.1–3.3
  are done. The handoff is plan Stage 0.6.

**Applicability.**
- **Groups that carried the findings:**
  - 1: authority and the declared boundary (F1, F3, F7);
  - 2: absence, types and relationships (F2, F5, F6);
  - 3: identity and equivalence (F5, F8);
  - 7: dependencies and reuse (F9, F10);
  - 9: capabilities and loss (F1, F2);
  - 10: provenance and diagnostics (F6);
  - 12: proportionality and falsifiable claims (F4, F12, F13, F14).
- **Group 5 (derivation)** bore through the seed findings against the behavior relations (F7)
  and the view conflict (F11).
- **Groups that did not bear:**
  - 6 (mutable workspaces): the attempt and publication protocol is unchanged;
  - 8 (layout and performance): nothing new is measured, and no performance claim is made;
  - 4 (declarative composition): the registry's definition AST is the only declaration surface,
    and it is Stage 4's;
  - 11 (evolution): the codebook appends are declared append-only. F6's `boundary_reason` append
    follows the existing rule.

---

## 8. Alternatives and architectural leverage

| Alternative | Semantic duplication and extension locality | Correctness and operational risks | Cost | Performance evidence | Why selected or rejected |
|---|---|---|---|---|---|
| **Baseline** (ADR-0004's compiler) | Good within briefs | Answers about 20 of 1,534 operations; the rest is silence | Built | Pilot 32.2 s (deviation B2, `just pilot` passed); not re-run here | Rejected by the deep review (F2); this review agrees |
| **The proposed set:** ADR-0021 and ADR-0022 plus five amendments, all decided at Stage 0 | One registry and one predicate authority (good); two place grammars, two condition languages, and fates beside seed findings (F5, F7) | Stage 1 is claimed over a scope nobody chose (F1), with a fate vocabulary that manufactures negatives (F2). Stage 2–4 semantics are decided before the D-2 spike and before the condition statistics exist, which is why they are under-specified (F5, F6, F11) | Two new ADRs and five amendments; Stage 1 code already in flight | None claimed | Direction adopted. The set needs revision |
| **Simpler viable alternative:** accept ADR-0021 and the ADR-0010 amendment narrowed to Stage 1 once F1–F4, F8 and F10 are fixed. ADR-0022 stays `proposed`, with §3.9 and §9.9 labelled Proposed, and is completed at Stage 2's start with the spike's result and this review's condition counts. No Python twin. Stage 1 uses the `verdict` codebook with three values | Fewer decisions now. Each Stage 2 decision is made where its evidence is (provider choice, measured atom coverage) | Risk: a Stage 2 decision forces a Stage 1 schema change. It is small: codebooks append, and Stage 1 carries no condition ids | Less now; ADR-0022's `standard` review moves to Stage 2's start | — | **Recommended.** It unblocks Stage 1 on the decisions Stage 1 needs, and it matches ADR-0022's own revisit trigger ("the Stage 2.1 spike decides the flow-IR provider") |

**Abstractions justified by current needs:**
- the `behavior` family and per-callable fates (Stage 1's consumer is `get_operation`);
- the `operations` catalog;
- one spec with view ids;
- the verdict codebook, in its Stage 1 subset;
- the registry, at Stage 4, with the member-digest rule.

**What remains ordinary code:**
- the flow builder;
- the dominator and control-dependence code;
- the SCC summary kernel;
- the Rust condition evaluator.

None of these needs a DSL. The registry's definition AST is justified only because concepts are
data the operator authors.

---

## 9. Verification and measurement plan

| Claim or risk | Evidence label now | Check | Expected | Gap |
|---|---|---|---|---|
| Per-callable analysis reaches outside the subsystem (or stops with `scope_boundary`) | Proposed | Test on `public_shapes` (F1) | A fate follows an out-of-prefix forward | Test |
| No negative fate without a lexical premise | Proposed | Rule over `control_fates` ⋈ `reference_resolutions`, with an injected case (F2) | Rejects the injected `no_read` | Rule |
| The freeze has a live authority | Proposed | Test: `analytics-freeze.json`'s `adr` is active (F3) | Fails today | Test |
| Condition normal form is canonical | Proposed | Rust unit test plus `conditions.json`, with negation, disjunction and literal types (F5) | One id per meaning | Both |
| One verdict per situation | Proposed | `behavior_shapes` budget-cut case; `semantic:refuted-needs-complete-region` once regions are declared (F6) | Rejects the violation | Both |
| Briefs agree with behavior | Proposed | `semantic:brief-agrees-with-behavior` (F7) | Rejects the injected disagreement | Rule |
| `find_operations` bounds and cursor | Proposed | Client adversarial tests (F8) | Stale cursor refused; NOT never matches unknowns | Tests |
| Registry and provider move identity | Proposed | Digest tests over data files and provider settings (F9) | Changes | Tests |
| Views under one spec | Proposed | Planned 1.7 tests plus windowing (F10) | Distinct keys; long bodies windowed | Tests |
| Pre-registration precedes output | Proposed | Commit-order check in the packet script (F13) | The set's commit precedes the snapshot | Recipe |
| Stage costs and latency | Not measured | `just pilot` stage timings; p50/p95 of the new tools | Reported; no target | Per stage |

**Cost accounting.** Not material at Stage 0. The deep review's §9 cost items stand.

---

## 10. Exceptions and deferred items

No SHOULD-level exception is claimed. The MUST-level gaps (F1, F2, F5, F6) are recorded as
unresolved, not excepted.

### Deferred

| Item | Why not now | Reopen when |
|---|---|---|
| O2: how the held-out tasks map onto the structured rubric | Increment 5 | Before the held-out run; decided without opening the set |
| O3: the §B1 clause in §B5 | Only binds if the spike chooses `ty` | The spike's outcome |
| O4: "curated subset" and §10.4's manual review | Briefs are not changed in Stage 1 | Stage 1's `FORMAT` 3 build or the increment-3 deep review |
| F14's SKOS and `lookup_concepts` items | Stage 4 | Stage 4's plan |
| The `where` model's locality (§5, extension journey) | Stage 1 design detail | The first facet added after Stage 1 |

---

## 11. Decision and implementation changes

**Decision: Revise.**

**What stands:**
- The product decision and its reasons (ADR-0021 Options 1–3).
- One concept registry, with membership only from definitions.
- Materialization at compile time and a bounded executor with no serve-time SQL or recursion.
- Models as digested, operator-authored data kept away from the gold.
- One spec with view ids.
- The ADR-0002 declared-family rule.
- The ADR-0005 statement that the closed language is not a solver.
- The labels. Every new claim is Proposed, and the superseded text is mostly kept as history.

**What blocks acceptance:**
- **G1 fails (documentary).** The spine names two keep criteria, live invariants are filed as
  history, and the freeze answers to a superseded record (F3).
- **G7 is unresolved on Stage 1.** The analysis scope is undecided (F1), and the keep rule has no
  criterion (F4).
- **G2 is unresolved on Stage 1.** The fates manufacture negatives (F2).
- **G2, G4 and G6 are unresolved on Stages 2–4** (F5, F6, F9, F10, F11). These need not block
  Stage 1 if ADR-0022 stays proposed (§8).
- **G3 and G5 are unresolved on `find_operations`** (F8).

**The smallest route to unblocking Stage 1:**
1. Fix F1, F2, F3, F4, F8 and F10 in ADR-0021, the ADR-0010 amendment and DESIGN.
2. Commit the question set (F13).
3. Re-review compactly, then accept ADR-0021 and the amendments.
4. Keep ADR-0022 `proposed`. Make F5, F6, F9, F11 and F12's decisions before Stage 2, with the
   spike's result, and review them at `standard` depth then.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| **P1** | F1: decide the analysis scope against the subsystem (ADR-0021, §1.4, §9) | DM-04, DM-43, DM-59 | ADR text; a Stage 1 fate beyond the prefixes (or a `scope_boundary` verdict) | Test on `public_shapes` |
| **P1** | F2: fates that cover every case; `read_elsewhere` from `reference_resolutions`; the verdict codebook's Stage 1 subset | DM-08, DM-42, DM-59 | §3.2 and §3.9 text; Stage 1 fixture | Rule plus injected case; `behavior_shapes` case |
| **P1** | F3: the spine sentences; §1.5's block moved; ADR-0021 restating the whole freeze; the freeze file's `adr`; pointer lines | DM-02, DM-59 | Text; the freeze file | Test that the freeze names an active record; `just gold` |
| **P1** | F13: commit `eval/behavior/fastmcp-4.0.5.toml` before any Stage 1 output is read; correct B3 | DM-28, DM-59 | The commit | Commit-order check (recipe) |
| **P2** | F4: pre-register the keep rule per technique and stage; name the navigation tool or drop it | DM-58, DM-59 | §9.8 and ADR-0020 text | Prose (the stage packets' arms) |
| **P2** | F8: define `complete`, facet statements, negation, caps, the cursor and `explain`'s depth | DM-07, DM-08, DM-14 | §11.3 text | Client adversarial tests |
| **P2** | F10: the query instruction and views, windowing, one spec per snapshot | DM-31, DM-32 | §11.1 and §B14 text | Planned 1.7 tests plus windowing |
| **P2** | F7: one derivation for seed fates, or an agreement rule | DM-02, DM-23 | §9 text | Rule |
| P3 (before Stage 2) | F5, F6: condition polarity and disjunction, one place grammar, literal encoding; the verdict's budget and opaque rules, regions and boundary kinds, `dynamic_access`, and its relation to modality and `evidence_status` | DM-06, DM-08, DM-15 | ADR-0022 amended; §3.9, §9.9, §10.2, ADDENDUM §3 | `conditions.json`; unit test; the refutation rule |
| P3 (before Stage 2) | F9, F11: registry and provider identity, the gold rule for concepts; where the views meet | DM-31, DM-32, DM-24 | §9.9, §B5, §3.9 text | Digest tests; `flow_shapes` case |
| P3 | F12, F14: remove the twin; land places with their consumers; trim the Stage 4 machinery; one review per point | DM-58 | Text | None needed |

**Final check.**
- **Claims against evidence.** They match: everything new is Proposed.
- **Scope against guarantees.** They do not match yet. The design claims the whole surface while
  the analysis is bounded by the subsystem (F1). It claims verdicts for fates that cannot carry
  them (F2).
- **Extension paths.** They are clear for models and concepts. They are not yet clear for
  conditions (F5) or verdicts (F6), and those are the ones Stage 2 needs.

---

## 12. Disposition (2026-09-24, by the author)

| Finding | Disposition | Where |
|---|---|---|
| F1 | **Fixed.** The subsystem scopes briefs only. The behavior scan covers `public_paths` under the public roots and follows parameters into any release function; the Stage 1 code already ran it with an unrestricted `inside`. Oracle: `behaviors_cover_public_callables_outside_the_subsystem`, where `pkg.Catalog.remove` and `.load` are outside the fixture's prefixes | ADR-0021 Decision; DESIGN §1.4; `crates/cpg-core/tests/analysis.rs` |
| F2 | **Fixed.** Stage 1 emits no negative fate. A parameter with no fate is `not_analyzed` for its other channels, never "unused". The `verdict` codebook landed in Stage 1, so nothing waits on Stage 2. The proposed rule against `reference_resolutions` is **deferred** to Stage 2's `field_writes` and `value_flows`, which read those channels | DESIGN §3.2; codebook `verdict` |
| F3 | **Fixed.** DESIGN §1.4, §12(b) and §9.8's rule bullet now carry the brief-retrieval record; §B4 and §B10 name the served model; §11.2 defers LanceDB to §13's trigger; §1.5's live brief rules are no longer under "history". ADR-0021 inherits every freeze, and the freeze file names ADR-0021. `check_gold.py` refuses a freeze naming a superseded or rejected record, and `just gold` passes | DESIGN; ADR-0021; `eval/gold/analytics-freeze.json`; `scripts/check_gold.py` |
| F4 | **Fixed.** The criterion and the exit are in ADR-0021 and §9.8. Communities have no tool consumer (ADR-0011 correction), and the named consumers are FCA facet suggestions and vector views. Oracle: none mechanical (prose), as the review said | ADR-0021; ADR-0011; ADR-0020; DESIGN §9.8, §9.9 |
| F5, F6, F9, F11 | **Open until the start of Stage 2**, as the review recommended. They are listed in ADR-0022 and §3.9, and ADR-0022 stays `proposed` until a `standard` review after the provider spike | ADR-0022; DESIGN §3.9 |
| F7 | **Deferred**, trigger: Stage 2.6. When Stage F renders controls from `behaviors`, briefs stop being a second derivation. Until then §9 says the two differ in mask: the seed passes use the subsystem, the scan the release | this table |
| F8 | **Fixed.** `where` operators, no negation, the two facet classes and what `complete` means, the capped `unknown` list, a cursor bound to the generation and request, and `explain` returning the stored derivation only | DESIGN §11.3; ADR-0010 amendment |
| F10 | **Fixed.** One spec covers briefs and views, and one query instruction serves until its trigger. `semantic:one-embedding-spec` means one spec per snapshot | DESIGN §11.1, §B14; ADR-0010 amendment |
| F12 | **Fixed.** The Python evaluator twin is removed. The evaluator is Rust and compile-time only, and a serve-time filter needs its own ADR | ADR-0022; DESIGN §B13, §3.9; ADR-0010 correction |
| F13 | **Already resolved.** The set was committed as `2fb10b8`, after the review read the tree. It holds 20 questions and 134 items. No Stage 1 pilot output was read before that commit: the first Stage 1 compile of FastMCP ran after it | `eval/behavior/fastmcp-4.0.5.toml` |
| F14 | **Fixed.** ContextVar places move to Stage 5; the per-language SKOS rule is dropped; `lookup_concepts` returns every concept while the catalog is small. Stage and increment reviews are merged where they coincide (plan amendment) | ADR-0022; DESIGN §3.9, §9.9; the plan |
