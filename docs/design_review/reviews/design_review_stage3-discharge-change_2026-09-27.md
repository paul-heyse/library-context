# Design review: Stage 3 P1 discharge implementation (change)

**Date:** 2026-09-27 · **Reviewer:** design-reviewer subagent (fresh context; not the author) ·
**Tier · purpose:** change · conformance (binding "Reviews in this repository": a bounded slice
inside ADR-0064's accepted boundary). **Standard:** core 3.0 (repository-owned, ADR-0040),
code-intelligence profile 1.1, library-context binding. **Revision:** `main` at `d8fa5f5`. The tree was clean at the start. During the review,
another session made uncommitted edits between 12:05 and 12:16: Logger models in
`models/external.toml`, `models.rs`, the `model_shapes` fixture, and `compile.rs`/`syntax.rs`
expectations. Those edits were not reviewed. The release binary and focused tests run here were
built from that tree. The edits do not touch the discharge path, but they change every
`external.toml` model id (O3).

This review is evidence, not authority. Findings keep the IDs below. Current disposition belongs
in the forward plan (§3.0 P1 row / §6) once the author schedules them (binding §4).

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | `220a0c8` (P1.1: summaries refuse decorated functions, compiler 104) and `d8fa5f5` (P1.2: claim-keyed discharge of call-transfer return claims, compiler 105, FORMAT 10) |
| Governing decision | [ADR-0064](../../adr/0064-call-transfer-discharge.md). The [target review](design_review_stage3-discharge-target_2026-09-27.md) supplies F01–F07. Its F08–F11 belong to P3/P4 and are out of scope |
| Supported scope | `returns` claims with `transfer = call` whose members are own-function parameter origins, discharged by `caller_return_summary`. Excluded: derives/stores (plan §7), refutation (P4) and field-mediated claims |
| Affected owners | `lctx-analytics::summaries::{finite, discharge}`, `cpg-core::{flow_model, behavior, attempt, validate, bundle}`, `cpg-schema::{behavior, codebook, rules, bundle}`, native `lctx_semantics` (`admit_discharges`), `lctx_mcp` (`Fate.discharges`), §3.9 and §9.9 prose |
| Extension scenario | Adding an argument-sink proof kind for `derives` (§5) |
| Enclosing limit | Stage 3 remains functionally incomplete and not assessed as an assembled whole. At this checkpoint no pilot summary crosses a call (target review O1), so P1 changes no pilot verdicts |
| Method | Read both diffs in full and the surrounding code at the cited lines. Four scratch probes with `lctx compile-fixture` (§10). Focused tests re-run. Not examined: native proof decoding beyond `admit_discharges`, and Stage E consumers of `behaviors` |

## 2. Conformance to target findings F01–F07

| Target | Required | Implemented (evidence) | Status |
|---|---|---|---|
| F01 claim-keyed evidence | `(snapshot, behavior_id, origin_id)` rows; membership from `flow_model`'s merge, accumulated over `claim()` merges; sibling closure; native citation check; Python citations | Relation `cpg-schema/src/behavior.rs:1534–1554`. Members are grouped by `(source_key, identity, through_call)` at each sink (`flow_model.rs:1735, 1782–1785`), with the same key as `merge()` (`767–779`). They travel as the index-aligned `value_flow_members` (`1880–1891`). Accumulated at `behavior.rs:1020–1029`. Rules at `rules.rs:3343–3391`. `admit_discharges` at `lib.rs:1165–1194`. Python renders `Fate.discharges`. Probe 2 serves `wrap`/`entry` with a proved citation | **Closed in substance.** Residual: nothing enforces that the cited members belong to the claim (F03) |
| F02 decorated refusal | One predicate. Refuse candidates, filter seeds, and refuse local callees | `flow_model::decorated` reuses `decorated_functions` (`flow_model.rs:381`). Filter at `finite.rs:1544–1580` covers direct (including context identities), modeled, assignment and local seeds, and the local callee | **Partially closed.** The modeled-chain producer (`finite.rs:1780`) is not filtered (F01) |
| F03 grade after merge | Order-independent; all members proved | `BTreeSet` members, `call_only` ANDed over merged rows, and the grade applied in the post-merge loop (`behavior.rs:1258–1272`) | **Closed for P1.** An unproved merged claim still keeps the reason of the first inserted row. This is latent for returns and becomes live for derives (F04) |
| F04 approximation | Flip before condition processing. Value approximation never overrides a `through_call` row. Condition approximation still applies | Through-call rows are already `unknown`, so `v.approximated && verdict != Unknown` never fires (`behavior.rs:1003–1006`). The flip precedes `condition.diagram()` and `condition.approximated()`. Probe 4: the certified `framed` claim (member `approximated = true`) is established; the uncertified `framed_local` claim stays open | **Closed in behavior.** No repository test covers it (F06) |
| F05 pure decide/grade | `decide` and `grade(members, decisions, flags) → (verdict, reason)` in analytics; core only applies the result | `discharge.rs:35–104` is pure and has store-free tests. `grade` returns only `bool`. The raw-flag policy (`call_only`) and the verdict assignment stay in core (`behavior.rs:1021, 1268–1271`). The `991`/`1200` override blocks are not deduplicated | **Partially closed** (F04) |
| F06 rule replacement and validation | Replace the unconditional rule. Equality check against the reconstructed outcome | Three rules replace `call-transfer-never-established`. `behavior-discharge-source-equality` reuses `expected` and `summary.boundaries` from `validate_summary_flows` (`validate.rs:672–698`) without a second producer run. Meta-tests exist for two rules; the compile test rejects an injected sibling omission. §3.9 and §9.9 are amended | **Closed** |
| F07 typed proof kind | Codebook with `caller_return_summary` only; refutation deferred | `DischargeProofKind` and `DischargeDecision {Proved, Open}`; no coverage column (`codebook.rs:1915–1930`) | **Closed** |

**Soundness.** The following risks were traced.

- **Conditional summaries.** A composed summary's condition is the seed contribution's condition. A conditional callee is admitted only when `specialize_local_condition` proves it holds (`finite.rs:2240–2268`). The claim condition is the OR over its members' conditions, so an all-proved OR is sound (the `twice` shape).
- **Async and generator functions.** Every seed family requires `DeclarationKind::Function` and excludes yield-bodied functions (`cpg-schema/src/behavior.rs:2509–2547, 2825–2917`). Async functions and generators get no summary.
- **Yield sinks.** They are graded but never proved (F05).
- **Field-mediated rows.** The block at `behavior.rs:1110–1230` is not graded and stays `call_transfer`. It cannot share an id with a graded row, because its `value` starts `via …`. This is sound and conservative.
- **Summary kinds.** Only `Value` summaries exist today, so the absence of a kind filter is latent (F02).

## 5. Extension scenario: an argument-sink proof kind (`derives`)

This is a traced route (Proposed); nothing here was executed. The trigger is plan §7: 4,619
`derives` claims at the pilot checkpoint.

| Step | Owner | Change | Assessment |
|---|---|---|---|
| Proof object | `codebook` | Append a kind such as `callee_formal_summary` | Additive; FORMAT unchanged because the column is utf8 |
| Decision | `discharge.rs` | A new per-origin decide from inner call → callee formal → callee summary. This needs call-link inputs that `Decisions` lacks. `grade` hardcodes `CallerReturnSummary` (`discharge.rs:96`), so `Decision::Proved` must carry the kind | Local to analytics |
| Grading | `behavior.rs` | Widen `graded` (`:1020`) to `Derives`/`Stores`. Argument claims merge several value flows (live F03 case) and carry `hop_reason` boundaries. `call_only` must decide which reasons a proof discharges | **Core internals edited** (F04) |
| Admission | `rules.rs:3360`, `lib.rs:1182–1189` | Both hardcode `summary.source_origin_id = origin_id`, which is false for a callee-summary proof | **The meaning is restated in two places plus the producer** (F02) |
| Closure and validation | `rules.rs:3378`; `validate.rs:672` | The sibling closure is sink-generic. The equality check reuses `grade` | No change |
| Serving | Python | `proof_kind` is rendered as text | No change |

The relation and codebook absorb the new kind, and serving is unaffected. Two things make the
extension edit more places than it should: the verdict policy lives in core (F04), and three
places each decide what a proof kind admits (F02). Both corrections are small if made before
the extension.

## 6. Correctness and fidelity gates

| Gate | Verdict | Evidence | Action |
|---|---|---|---|
| G1 Authority | unresolved | A single rule family now owns the call-transfer verdict (good). But "a summary proves this return" is decided by two different predicates: composition requires `kind = Value` and `output_path = ReturnValue` (`finite.rs:2231–2236`); discharge, the SQL rule and native accept any E/C row | F02 |
| G2 Semantic fidelity | pass (scoped) | Only `Value` summaries exist. Served verdicts matched runtime in the harness and the probes. Minor mislabel: yield claims get `caller_return_summary` rows | F05 |
| G3 Validity | unresolved | Citation shape, origin, verdict and sibling closure are enforced. That the cited members are the claim's members has no enforcement point | F03 |
| G4 Hidden behavior | pass | `Decisions` and `grade` are pure; no I/O added | — |
| G5 Consistency | pass | Discharges are written in the same attempt and validated before the `snapshots` append. FORMAT 10 and compiler 104/105 bumps | — |
| G6 Transformation and reuse | pass | The grade is set-based. `argument_flows` is written by the same SQL (`attempt.rs:871`). Its dependencies are extraction tables not written in between, and `behavior::run` does not mutate its fetched copy (`behavior.rs:278, 336–398`), so the output is unchanged | O1, O2 |
| G7 Truthful capability | **fail** | §9.9 (`behavioral-analysis.md:486`) says "decorated functions have no base summary", labelled Implemented and focused Tested. Probe 1 publishes and serves an established depth-2 `summary_flows` row for a decorated function | F01 |
| G8 Library leverage | pass | Keyed joins in plain Rust, as the target review §8 chose; no new machinery | — |
| CI-G1 Fidelity | unresolved | Served `returns` verdicts pass the CPython harness and every probe. Native `inspect_value_paths` serves every summary of an (operation, formal) as a path, separately from the boundary rows (`lib.rs:1450–1476`), so it would serve the decorated function's path. That is inferred from code: the probe's native load was not usable (O3). Latent Constant/Transform discharge | F01, F02 |
| CI-G2 Evidence closure | pass | A served claim cites summary ids in the same generation. Native refuses missing, foreign, unproved, malformed and duplicate citations (tested). Membership correspondence is F03 | F03 |
| CI-G3 Evaluation integrity | pass | Generated programs and fixtures only; no gold path | — |

## 7. Findings

| ID | Finding and scenario consequence | Principles · judgment/gate | Evidence/gap | Correction and owner | Closure evidence | Disposition |
|---|---|---|---|---|---|---|
| <a id="F01"></a>F01 · **medium-high** | **The decorated refusal misses the modeled-chain producer.** P1.1 filters direct, modeled, assignment and local seeds, but not `chain_arguments`. `@replace def decorated_chain(value): return cast(object, cast(object, value))` publishes an established depth-2 summary, paired with an `outside_provider_model` boundary, and the generation serves it. Served `returns` verdicts stay `unknown`: the boundary keeps the origin open, and the callee filter blocks composition (`through_decorated_chain` is open, reason 10). But the §9.9 claim is false, and native value-path inspection gets a path for a function whose binding the decorator replaced, which is the defect P1.1 set out to remove | CI-02, CI-06, DP-22, FP-05 · G7, CI-G1 | `finite.rs:1780–1868` has no decorated check. Filter at `1544–1580`. Probe 1 (§10). Native serving path `lib.rs:1450–1476` | `lctx-analytics::summaries::finite`: filter `chains` by `decorated.contains(function_node_id)`. Better, apply one filter to every seed family's function before any producer runs, so a later producer (P3/P4) cannot bypass it. Add the chain shape to the `transferpkg` control | The probe shape has 0 `summary_flows` rows and keeps its boundary. The `transferpkg` count loop includes a decorated chain | plan P1 (to assign) |
| <a id="F02"></a>F02 · medium | **Proof admissibility is restated and weaker than composition.** `Decisions::from_outcome` (`discharge.rs:35–57`), `discharge-cites-summary` and `admit_discharges` accept any established/conditional row for the origin. They ignore `kind`, `output_path` and `approximated`. Composition requires `Value` and `ReturnValue`. Plan P3 keeps value/transform/constant rows in `summary_flows`, and the schema forces a non-null `source_origin_id`. A `Constant` row citing a parameter origin would therefore establish "returns" for a function that does not return the argument. A `Transform` row would make the harness's identity assumption (`test_semantic_soundness.py:204`) wrong | DP-01, FP-04, CI-06 · G1, A2, CI-G1 (latent; the trigger is the first non-`Value` producer) | `finite.rs:2231–2236` vs `discharge.rs:48–50`, `rules.rs:3360–3375`, `lib.rs:1174–1189` | Add one predicate per proof kind in `cpg_schema::summary_contract`, e.g. `proves_caller_return(&SummaryFlowsRow)`: E/C, no boundary, not approximated, `ReturnValue`, `Value` (admit `Transform` only by an explicit decision). Use it in composition, `Decisions` and native, and mirror it in the SQL rule | A pure test in which Constant, Transform and approximated rows do not prove. A rule meta-test with a non-`Value` citation | plan P1 (before P3) |
| <a id="F03"></a>F03 · medium | **Nothing enforces that cited members belong to the claim.** The rules check each citation and the closure around a cited origin. None ties `d.origin_id` to the claim's parameter, sink site and `through_call`. Members also cross modules as a side vector aligned by index with `value_flows`. A later sort or filter of `value_flows`, or a doctored generation, that attaches another sink's proved members would publish an established claim that passes all three rules and native admission | DP-03, DP-04, CI-11 · G3, A2 | `flow_model.rs:652, 1880`; `behavior.rs:1027`; `rules.rs:3343–3391` | Carry each value flow's members with the flow itself, as a field or a map keyed by the flow, not by index. Add `semantic:discharge-member-of-claim`: each row's contribution has `parameter_node_id = b.parameter_node_id`, `through_call`, a `Return` sink, and `(module, start) = (b.site_module_node_id, b.site_start_byte)`. Owner: `cpg-schema` rules; `flow_model` representation | An injected row citing a proved foreign origin is rejected | plan P1 |
| <a id="F04"></a>F04 · low-medium | **The discharge verdict policy is split between analytics and core** (target F05 partially adopted). `grade` answers only "all proved". Which merged-row reasons a proof discharges (`call_only`), and the resulting verdict, are decided in core. Both depend on the override order in the claim loop (capture overrides; approximation is guarded by `verdict != Unknown`). An unproved merged claim keeps the first row's reason. The certified-approximation and capture rules can be tested only by a compile. For `derives`, hop reasons and a reason precedence must be added inside core | FP-06, FP-01, DP-08 · A1 | `behavior.rs:1003–1006, 1021, 1268–1271`; `discharge.rs:77–104`; ADR-0064 "grades a claim from its members' decisions and raw flags; core … owns no verdict policy" | Pass each merged row's reason or flags (call-only, captured, approximated, hop) into `grade`. Return `(verdict, reason)`, including a deterministic precedence when unproved. Core applies the result. Deduplicate the field-mediated override block through the same function | Store-free tests: capture blocks the flip; certified approximation is discharged; a hop reason blocks; two merged rows give the same result in either order | deferred; trigger: before any non-`Returns` kind is graded (§5) |
| <a id="F05"></a>F05 · low | **Claims that no proof can discharge get `open`/`call_transfer` rows.** `graded` covers `FlowSink::Yield` because both sinks map to `Returns`, yet no summary candidate is a yield. A served yield claim (probe 2 `gen`) shows the same open row as a candidate that stopped at a call. Field-mediated claims get no rows at all. The target review's absence lattice puts such claims ("not a candidate") under no decision row | DP-02, CI-04 · G2 | `behavior.rs:988, 1020`; probe 2 | `graded = kind == Returns && v.sink == FlowSink::Return && v.through_call`, or append a `not_applicable` decision if serving needs it | Probe `gen` has no discharge rows and stays `unknown`/`call_transfer` | plan P1 |
| <a id="F06"></a>F06 · low | **Test gaps against the named closure evidence.** (a) Target F04's positive (certified approximation established) and negative (uncertified sibling open) cases are asserted nowhere; the existing `framed_modeled_identity` fixture could carry them. (b) Modeled, chain and assignment discharges, now established for `model_shapes` `identity`, `nested_total_identity` and `indirect_identity`, are not asserted. (c) The mixed row is covered only by the pure test. (d) The harness checks `established` only, not conditional discharges | DP-23 · G3 | `compile.rs:303–355`; `compile.rs:4650`; `test_semantic_soundness.py:204–215` | Add (a) and (b) to the `model_shapes` compile test. Add (a) as a pure grade test once F04 lands | The listed assertions pass | plan P1 |

**Observations (not findings).**

- **O1.** `argument_flows` is now evaluated twice: once by `write_analysis_query` and again in `behavior::run` (`:278`). `BehaviorRows.argument_flows` has no consumer outside `behavior.rs`. Consider reading the written table and dropping the public field.
- **O2.** The Pass B invocation identity (`behavior::digest`, `:251–258`) folds in the flow-model and relation digests but not the summary/discharge producer, even though verdicts now depend on it. The snapshot-level compiler digest covers this, and no invocation-level reuse exists today. Fold it in if one is introduced.
- **O3 (review artifact, not a product finding).** The first probe's generation, which contains a `typing.cast` modeled frame, failed native load with `frame certificate has no exact pinned body contract` (`lib.rs:733–744`). A `model_id` hashes the whole catalog source file (`models.rs:915–921`). The concurrent uncommitted `external.toml` edit therefore gave the rebuilt compiler different ids from the installed native wheel's committed catalog. This is attributed to that mismatch; P1 was not implicated (`admit_discharges` had already passed).
- **O4 (cost).** Membership adds one `BTreeSet` per value flow. Validation adds one single-member `grade` per discharge row. Both are negligible next to the 25 s flow model and 107 s validation (target review §4, historical receipt).

## 8. Library fit and total complexity

| Capability | Candidates | Choice |
|---|---|---|
| Decide/grade | Plain Rust joins; Datalog | Plain Rust (target §8). Non-recursive keyed joins; confirmed |
| Membership | `flow_model` merge vs re-derivation in SQL | From the merge's owner, as chosen. Representation per F03 |

The change adds no new mechanism; total machinery is one pure module, one relation and three rules.

## 10. Verification

| Check | Label and date | Command | Outcome |
|---|---|---|---|
| Pure discharge, decorated refusal, `transfer_alternatives`, rule meta-test | Tested 2026-09-27 | `CARGO_TARGET_DIR=…/target RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo nextest run --release -p lctx-analytics -p cpg-core -E 'test(discharge) \| test(decorated_functions_have_no_paths) \| test(transfer_alternatives) \| test(the_analysis_rules_reject_their_violations)'` | **passed** (8 tests) |
| Native citation admission | Tested 2026-09-27 | `uv run pytest -q python/lctx_mcp/tests/test_native_semantics.py -k discharge` | **passed** |
| CPython harness, known shapes (including decorated `f6`/`f7`) | Tested 2026-09-27 | `uv run pytest -q tests/scripts/test_semantic_soundness.py -k known_shapes` | **passed** |
| Probe 1: decorated chain, modeled, plain chain, wrapper, yield | Tested 2026-09-27 (scratch) | `lctx compile-fixture <scratch>/probe/tree --package probepkg --seed probepkg.entry …`, then `lctx query` on `summary_flows`, `summary_boundaries`, `behaviors` ⋈ `behavior_discharges` | **passed** as a probe. It shows `decorated_chain` depth 2 established plus reason 10 (F01); `modeled`, `plain_chain`, `wrap` and `entry` established with proved rows; `gen` open with `call_transfer` (F05). Native load **blocked**: the compiler and the installed wheel had mismatched catalogs from the concurrent edit (O3) |
| Probe 2: model-free served fates | Tested 2026-09-27 (scratch) | Same, then `load()` + `get_operation` | **passed**: `wrap`/`entry` established, `cond` conditional, each with one proved citation; `gen` open |
| Probe 4: certified approximation (`try: return cast(object, value) finally: pass`) and local sibling | Tested 2026-09-27 (scratch) | Same, then `lctx query` | **passed**: `framed` established with a member whose `approximated = true`; `framed_local` unknown, open reason 10 |
| Integrated gates | — | `just test-all`, `just pilot` | **not_run**: end-of-scope acceptance per AGENTS.md. The author's commit receipts (workspace nextest, pytest, clippy, docs-check; 2026-09-27) are historical |

The probes were run in the session scratchpad. The only file this review writes is this one, so
their sources are summarized in the rows above rather than kept as evidence.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence | Action |
|---|---|---|---|
| A1 Localize change | satisfied for P1; unresolved for the extension | The decision is pure and store-free tested. The verdict policy for the flip still sits in core's claim loop | F04 |
| A2 Encode meaning structurally | unresolved | Claim-keyed evidence, one rule family and membership from the merge's owner are real improvements. But proof admissibility has two predicates, and membership correspondence is an unenforced index alignment | F02, F03 |
| A3 Extend through composition | satisfied, scoped | The typed proof kind and a sink-generic closure absorb the argument-sink kind. The remaining edits are named in §5 | F02, F04 before the extension |

**Bounded decision: Revise (small, no re-review needed if the corrections land as stated).** The
direction and most of the slice conform to ADR-0064. Target F03, F06 and F07 are closed. F01 and
F04 are closed in substance; F02 and F05 are partially closed. G7 fails because of this review's
F01: a decorated function with a modeled-chain return still has a published, served summary. Fix
F01, F03 and F05 before P1 is recorded as closed. Fix F02 before P3 adds non-`Value` summary rows,
preferably in the same follow-up. F04 is deferred until an argument or store sink is graded. F06
travels with F01–F03.

**Enclosing architecture.** Stage 3 remains functionally incomplete and not assessed as an
assembled whole. This slice certifies neither the S8 exit nor release qualification. It moves no
pilot verdict (target review O1).
