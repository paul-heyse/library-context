# Plan: the behavioral model, going forward

**Status:** Active; the sole product execution plan. **Revised:** 2026-09-27 (the operator
resumed Stage 3 with an exit-driven sequence, §3.0; baseline diagnostic, §1.2).
**Authority.** The [architecture map](../design/README.md), DESIGN and its section owners own
contracts and accepted targets; current ADRs own rationale. This plan owns execution order, the
current disposition of scheduled findings (§6) and deferred triggers. A change to a §B decision
lands its ADR and a design/target review first (binding under ADR-0040).

## 1. Current state and qualification boundary

Production is on `main`; the pre-Stage-3 repairs after `acbcee5` include
schema migrations and targeted checks. Increment 4 (Stages 2.9–3) remains in progress. Stages
0–2.9 built the whole-surface operation catalog, ty `flow` facts, conditions, five verdicts and
Stage 2 behaviors. **The remediation and remaining Stage 3 scope are functionally incomplete;
integrated qualification is outstanding.**

| Scope | State (2026-09-27) | Remaining boundary |
|---|---|---|
| L1/L1.5 conditions | **Partially implemented; focused tests passed.** Bounded BDD catalogs, exact entry-value links, literal string membership and non-bool integer equality, non-allocating decisions, effective support, cube factoring, restricted exact-input assessments with bounded link minimization, a ten-atom truth-table control and focused CPython literal lowering | Operation-wide compatibility; real-pilot W6 cost; W11's served round trips and Q09 |
| L4 pinned models | **Implemented; focused tests passed.** Typed transfer/effect/callback/resource/exception assertions and exact source applications. `typing.cast`/`typing.assert_type` have typed direct-return bodies; full completion additionally requires a bound-argument frame-release certificate. A Pydantic `TypeAdapter.validate_python` potential transform is dormant | Pure/helper, I/O, async/context, pydantic and HTTP/server families; a dynamic-schema representation before any validation effect |
| L2 fates | **Implemented and Tested in focused cases.** Ordered expression/predecessor/frame completion, exact-TypeError handlers and re-raise, handler/else/finalizer entry, persisted exact exception identity and pinned nullcontext/suppress lifecycle completion | Broader default domains, named-handler cleanup, unmodeled contexts, broader entry-result domains, callback fate and resource pairing remain open |
| L3 summaries | **Partially implemented; focused tests passed.** Direct, exact modeled, unique-assignment and local value paths retain ordered, origin-specific proofs. Normalized local arguments require every explicit evaluation and all fixed formal mappings, with certified defaults for immediate fresh nested calls, including a separate tracked-source read. Bounded nested closed expressions supply independently represented exact Boolean controls. The BDD kernel supports simultaneous substitution; the pure producer consumes multiple stable linked controls. SCC scheduling separates semantic progress and nondominated depth/expanded-proof-cost progress from retained witness alternatives in the bounded ADR-0053 value worklist (compiler101). Real three-argument/reversed-keyword calls, missing/raising arguments and default expression depth/work caps reach Delta/native; source-level later-predicate identity remains unknown | General reads/calls/defaults/predecessor/frame completion; call-specific multi-control stability and residual conjunction; multi-channel semantic/witness state; effect/exception/role summaries; original W5/W7/W12 cap traces and channel discharge |
| L7 serving | **Partially implemented.** FORMAT 9 has internal callable-formal support, checked native IPC decoding and bounded cited-finding support closure; a finalizer and a finding-only coordinates claim round-trip in focused tests. `inspect_value_paths` remains path-local and exposes source fact/origin IDs on both positive paths and open boundaries; operation facet values retain verdicts | Typed operation-wide compatibility/effect/role filters, full proof spans beyond the cited witness projection, clean-wheel query |
| Current checkpoint / Stage 3 acceptance | **Checkpoint tests passed; Stage 3 acceptance not established.** The 2026-09-27 `just test-all` passes 429 release Rust tests, the fresh Python fixture, 148 Python tests, Pyrefly, strict lints, rules, ADR/agent checks, 83 fixture parses and dependency/gold policy. See the receipt below | S1–S7 remain functionally incomplete; Q01/Q03/Q05/Q09 exit assessment, semantic-query packet, clean-wheel query, live W9/W16 conformance, controlled cost comparisons and assembled review remain `not_run` |

**Accepted schema migrations at this checkpoint:** extractor output 33; compiler output 102; catalog format 7.
Compiler101 changes bounded scheduler progress; compiler102 repairs repeated pinned-context observations and migrates the model argument-binding query. Neither adds Arrow columns.
The current contracts include `expression_evaluations` and ordered operand proofs, statement/frame
completion, condition-keyed return entry, native MRO completeness, a persisted exact exception
codebook and `summary_origin_coverage`. Snapshots are reviewed migrations; a fresh store rebuild
under ADR-0048 is completed for this checkpoint (§1.1). FORMAT 9 callable support is implemented; remaining S6 semantics are open.

**Resumed 2026-09-27.** The operator approved the exit-driven remaining sequence (§3.0,
"Remaining sequence"). The §1.2 diagnostic is the current reading of the served answer.

**Next dependency on resumption:** finish remaining channel contracts and supported synchronous context exits;
then compose stable controls and channel coverage through the multi-channel worklist. Direct
value-origin coverage exists, but call origins and uncertified approximated paths remain open even when a
positive witness exists. Coverage is not operation-wide and does not close another channel.
W5/W7/W12 default-cap traces, broader model activation and remaining FORMAT 9 semantics are outstanding.

**Measurement baseline** (pilot snapshot `162bda5a0fe39cc1cedadbfaa8efd1f9`, 2026-09-24, fake
embedder; Measured): 1,534 public declarations; 15,415 behaviors (4,276 established, 3,554
conditional, 19 refuted); unknown by reason: **6,137 `call_transfer`**, 1,347 `override_dispatch`,
42 `budget_reached`, 39 `abstract_body`, 1 `runtime_unreachable`; pilot compile 40.4 s. Stage 3
reports its exit against these counts. The compiler102 checkpoint below does not show a
`call_transfer` reduction; the frozen semantic exit assessment is still outstanding.

### 1.1 Operator-requested checkpoint (2026-09-27)

All Cargo/native commands use `CARGO_TARGET_DIR=/home/paul/library-context/target` rather than
this shell's inherited sibling-project target. The full gate uses `RUST_MIN_STACK=16777216`.

| Command | Outcome and exact scope |
|---|---|
| `just fmt`; `uv sync --frozen --reinstall-package lctx-semantics` | **passed:** accumulated Rust/Python formatting and lint repairs; native102 rebuilt in 31.05 s (`/tmp/lctx-stage3-checkpoint-native102-final.log`). Compiler101 changes scheduler progress; compiler102 repairs repeated pinned observations. No additional Arrow columns. |
| `just test-all` | **passed:** 429/429 release Rust tests in 102.945 s, rebuilt fixture, 148/148 Python tests in 42.65 s, Pyrefly, strict Clippy/Ruff, seven rule suites, ADR/agent lint, 83 fixture parses, dependency/fork/shear and gold checks. Snapshot updates disabled. `/tmp/lctx-stage3-checkpoint-test-all8.log`. The all-techniques guard and output ledger now cover compiler102/template20. |
| Checkpoint corrections | **failed, corrected:** compilation/lint and stale fixture/validator/contract expectations; repeated-context binding and duplicate-fact withholding; the all-techniques query digest; and the schema SQL regression test location required by the rule scan. Nine output snapshots were inspected before acceptance. The snapshot-generation run used `INSTA_FORCE_PASS=1` and is not qualification. The final gate above has no overrides allowing failures. |
| RCA freeze alignment | **passed:** the existing ADR-0058 typed RCA policy is now recorded by the variant-policy digest. ADR-0021's dated amendment discloses the correction; no target, threshold, selection parameter or other analytics parameter changes. The frozen-parameter test and `just gold` pass. |
| Fresh `just pilot build/store-stage3-checkpoint-2026-09-27` | **passed for real extraction, compilation, publication and serving smoke with fake vectors:** snapshot `8b4fb9ab6dcaa3698c27018593553129`, generation `cdcf4b4e519e8b79`, 20 briefs. The initial attempt failed on repeated `json.dumps.obj` observations; compiler102 repairs that existing binding contract. `/tmp/lctx-stage3-checkpoint-pilot2.log`; live embedding **not_run**. |
| Read-only `lctx query` over that snapshot | **passed:** 16,825 behaviors: 3,023 established, 2,031 conditional, 18 refuted, 11,753 unknown. Unknown reasons include 6,353 `call_transfer`, 2,436 `outside_provider_model`, 2,018 `override_dispatch`, 730 `missing_evidence`, 176 `scope_boundary`, 39 `abstract_body`, one `runtime_unreachable`. The 18 refuted rows are product data, not a rating of the five frozen negative evaluation items. The raw signature catalog still contains two independently attributed observations of every `json.dumps` formal. |
| `just docs-check` | **passed:** 135 canonical pages, ADR/agent lint and offline link/fragment checks. Corrected a stale dated STATUS link in the existing expression-completion review; no new design review was run. |

**Measured checkpoint cost:** 32.5 s extraction, 204.0 s total, 107.25 s validation and
4,258 MiB peak reported RSS. This was a shared-machine run with concurrent test compilation,
not a controlled regression or engine comparison. The validation cost remains a target for S8
measurement; no cost improvement is claimed.

This is qualification of the implemented checkpoint, not completion of S1–S8. Live embedding,
all-channel semantic requests and the frozen Stage 3 exit rule are not established by fixture
providers or the ordinary pilot. Work stops after this checkpoint as requested.

### 1.2 Stage 3 baseline diagnostic (2026-09-27; not qualification)

The qualified unknown→partial rubric rule and the evaluation-only requests were committed
before any Stage 3 packet was read (`2c6a59c`). Then
`uv run python scripts/structured_eval.py build/generations/cdcf4b4e519e8b79 eval/behavior/fastmcp-4.0.5.toml --stage 3 --requests eval/behavior/fastmcp-4.0.5.requests.toml --out build/structured/stage3-baseline-diagnostic.md`
**passed** (the packet was written). The reading below is the author's informal diagnostic of the
checkpoint generation. It is not the S8 assessment, and it does not rate the exit.

| Question | Positives, informal reading | Notes |
|---|---|---|
| Q01 | a–f partial; b, e and f only through the `FunctionTool.from_function` request | g (negative) not claimed |
| Q03 | a, b, d, e partial; f absent | d was *present* at Stage 2: the store is now `unknown`/`missing_evidence`. c, g not claimed |
| Q05 | a, b, e, f partial; c, d absent | b regressed like Q03.d. No re-raise row is served for `call_tool` (cause not investigated). g not claimed |
| Q09 | a–e partial; f borderline | Both `inspect_value_paths` requests return no path and no boundary. g not claimed |

About 19 of 23 positives read as partial. None reads as present, and no negative is claimed.
The numeric threshold is not the binding risk. Four findings affect the remaining queue:

1. **Decorator over-withholding.**
   - `flow.decorated` (`cpg-core::flow_model`) counts any decorator syntax. The behavior
     post-pass then makes every behavior of that operation or hop `unknown`/`outside_provider_model`
     ("a decorator may replace the callable binding").
   - This covers builtin `@classmethod`, `@staticmethod` and `@property`.
   - Against the last format-6 generation (`7219df40ce349931`), 914 shared rows moved from
     established/conditional to this reason; 2,436 rows carry it now.
   - It hides `FunctionTool.from_function` and `TaskConfig.from_bool` (Q01.b/c/e/f).
2. **Approximation withholding.** 612 shared rows moved to `missing_evidence`. These include
   the settings-snapshot stores behind Q03.d and Q05.b.
3. **Field-insensitive smear.** `from_function` builds `metadata = ToolMeta(...)` and then reads
   fields. So every option `derives` into every `metadata.*` sink: `title` into
   `str(...)@352` and `ValueError@317`, for example. These rows are `unknown`, but they are the
   main *misleading* risk for Q01.a/e.
4. **Pilot composition is nearly absent.** There are 26 `summary_flows` rows, all at depth 0,
   and nothing discharges behaviors (`cpg-core::behavior` marks every `through_call` value
   `call_transfer`).

## 2. Product target and principles

The product (ADR-0021) is an **evidence-carrying behavioral model of a pinned library's whole
public surface**. An agent asks which operations accept X, pass it to Y, under which
configuration, raising what, needing which lifecycle, and receives exhaustive answers where the
analysis is complete, ranked candidates where only discovery applies, and a named unknown where the
analysis stopped. Each claim carries evidence and a derivation; briefs remain one rendering.
[§3.9](../design/sections/behavior-model.md) owns conditions, places and verdicts;
[§9.9](../design/sections/behavioral-analysis.md) owns models, summaries and the registry.

| # | Principle | Without it |
|---|---|---|
| T1 | The whole public surface is the universe; analysis runs per callable, bottom-up | Silence about most operations |
| T2 | Relations first, sentences last: typed, persisted relations with in-row provenance | Knowledge locked in templates |
| T3 | The flow IR is our declared runtime model (§B5), whichever provider builds it | Checker views relabelled as runtime flow |
| T4 | Meaning comes from models, propagation from summaries; never "calls X, so can X" | Unsupported transitive claims |
| T5 | Conditions are Boolean functions over evaluation atoms in a bounded decision-diagram kernel, plus a typed theory for primitive places; no theory solver (§B10) | Merged evaluations drop feasible paths; budget cuts lose structure |
| T6 | Five verdicts, never a null; a negative verdict only inside complete coverage | Unsupported negatives |
| T7 | Materialize source facts at compile time; run bounded semantic selections in the pinned Rust executor | A second, unproven serve-time semantics |
| T8 | Discovery nominates, definitions decide; FCA, RCA, communities and embeddings never write membership | Statistical membership |
| T9 | Pre-registered behavioral question sets, plus mechanical known-answer fixtures | Circular evaluation |
| T10 | Providers observe; our semantic layer concludes under stated, tested rules | Provider artefacts become claims |
| T11 | Execution checks admission: within the stated model every observed execution is admitted; a violation is a counterexample; a finite pass is evidence, not proof | Translation defects stay invisible |

Kept unchanged: programmatic synthesis with no generative model in the pipeline or query path
(§B11); immutable, byte-identical generations (§B7, §B12); declared dependency families
(ADR-0002); the gold under `.claude/skills/` is never a compiler input; licence is never a
criterion.

| Layer | What | State |
|---|---|---|
| L0 | The CPG | Built |
| L1 | Flow facts (`flow` family, ty through `cpg-flow`) | Built |
| L1.5 | The condition kernel (bounded BDDs, typed primitive theory) | Built in part |
| L2 | Behavior relations, including handlers, callbacks, resources and exits | Built in part |
| L3 | Transfer summaries resolving `call_transfer` | Built in part |
| L4 | Models catalog | Built in part |
| L5 | Capability registry and membership | Stage 4 |
| L6 | Discovery (FCA, RCA, communities, views) | Built; consumers in Stage 4 |
| L7 | Serving (FastMCP tools, native executor) | Built in part |
| L8 | Structured evaluation | Built; stage exit rules continue |
| L9 | Validation lane (runtime soundness oracle, CrossHair, Pysa) | Built in part; order 8 extends it |

## 3. Stage 3 execution queue

Every intermediate outcome is a cited positive candidate or `unknown`. No absence becomes
`refuted_under_model` before the model channels and relevant source paths are closed. A slice
uses focused compilation, one positive and one withholding fixture, its schema/rule snapshots and
the shared publication validator; the last column names a targeted check, not integrated
acceptance. Findings are identified in §6; their priority follows the follow-up review's §12.

### 3.0 Consolidated execution

**Accepted target; implementation in progress (2026-09-26, ADR-0057 and ADR-0058).** This is the
operator-approved detailed execution queue. ADR-0059 refines class-protocol applicability for
S2b/S3a; the bounded nullcontext/suppress lifecycle subset is implemented and focused Tested; the remaining scope stays open. Orders 1–10 below retain their semantic requirements;
§6 retains finding disposition. The [composition review](../design_review/reviews/design_review_stage3-composition_2026-09-26.md)
F01–F04 remain applicable. Accepted targets do not certify their implementation.

**Current implementation checkpoint.** Current contracts include typed value-origin and execution-site
coverage, including complete empty escaping-exception domains under statement entry. Async and
generator scopes remain excluded. Fresh nested definition-header completion is reconciled with
source/Delta controls; immediate fresh nested calls now use availability/value/stability certificates. Shared callee proof admission replaces
the independent native adjacency policy and preserves typed proof-limit refusals. FORMAT 9 now carries private/nested callable formals for control-proof closure without exposing
private operations; full S6 remains open. Catalog format 7 requires authored model phases with independent identities and exact
provider-phase matching, and typed static/runtime/unresolved validation schema attribution distinct
from the effect subject. Runtime schema candidates retain a separate source binding or an explicit
reason; no production validation effect is activated. Extraction-only compilations leave behavioral
coverage unexamined when its published premise catalog is absent and reconstruct no entry links
against that absent catalog. Repeated library/corpus observations of an identical pinned definition or signature slot now select a deterministic cited representative without deleting raw facts; conflicting payloads or duplicated parameter facts refuse binding. S5 now preserves transfer alternatives, origin-specific predecessor joins, provider approximation,
exclusive receiver admission and typed transfer/scope through serving. Extractor output 32 separates
provider approximation from the explicit call-transfer obligation. An occurrence-specific source
parameter-identity certificate now links a bare return read to its unique immutable lexical
parameter. It preserves provider flags and discharges only the matching origin's value
approximation; entry/exit proofs remain mandatory. FORMAT 9 carries its source citations and
rejects missing, foreign, duplicate or uncited certificate support. Current FCA/RCA and Pass C
now retain typed attribute identity, endpoint modality/phase and occurrence evidence through
publication and serving. One renderer replaces label parsing; per-supporter and per-retained-pair
closure rejects missing/foreign/partial evidence. These are partial
S1/S2/S5/S6 changes, not an assembled Stage 3 receipt. STATUS owns the latest
checkpoint command/outcome; the [bounded review](../design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md)
keeps target acceptance separate from implementation scope and certificate correction F08.

| Step | Work, owner and deletion obligation | Acceptance / current state |
|---|---|---|
| S0 | Reconcile draft header work; update this plan and governing sections; record ADR-0058. Activate the already-triggered transfer and W10 repairs. Keep source finding IDs and one disposition owner | Planning checkpoint complete: tree reconciled, plan adopted and the transfer/W10 repairs implemented. Remaining product scope and acceptance are S1–S8 |
| S1 | Schema owns typed subjects/origins for completion, value, transform, effect, exception, callback, resource and role; phase-specific coverage separates complete empty results, unresolved alternatives and omitted witnesses. Shared structural proof admission replaces native adjacency and seed-specific classifiers as consumers migrate. Add model phase and dynamic-schema contracts | Value and execution-site coverage, shared value-proof admission, model phases/schema candidates and independent source/model invocation are implemented. General typed non-value subjects/payloads, complete empty channels and their shared proofs remain open; parameterless actions must not acquire fabricated value IDs |
| S2a | Complete definition-time header evaluation, then call-specific default availability, binding, value and stability certificates. Start with undecorated, unescaped synchronous nested definitions immediately followed by the sole direct call. Defaults are never evaluated at invocation; unused omitted formals still require availability | Fresh header completion implemented; source/Delta controls cover positional and keyword defaults, unevaluated body, missing defaults, decorators and rebinding. Immediate fresh default invocation passes source/Delta/native controls; broader domains remain withheld |
| S2b/S3a | Bootstrap the minimum synchronous protocol models, then compose ordered entry and reverse exits, suppression and replacement of pending outcomes, resource identity and distinct callback fates. Named-handler cleanup requires a proof or a specific boundary | Later entry failure still exits earlier managers; exit raises; suppression; finalizer override; callback not run; acquire without release. Pinned nullcontext/suppress completion and mandatory ordered return commitments implemented with source/Delta/native and independent runtime controls (2026-09-27); narrow entry-result identity is also proved; broader entry domains/models, concrete resource identity/pairing and distinct callback fates remain open. Named-handler cleanup has a specific unknown boundary; general cleanup completion remains unsupported |
| S3b | Activate every §3.3 family's supported exact source targets by channel and phase; unsupported phase/binding and no-consumer targets remain explicit candidates. Typed runtime schema precedes validation effects. Source-backed total return is independent of transfer and effects | Per-target positive/withholding pairs, exact pins/signatures, catalog equality and independent pure-model challenge. ADR-0060 action timing and narrow potential I/O activation implemented; remaining families/bindings/fates open |
| S4 | Call-specific stable controls, simultaneous bounded substitution and residual caller/callee conjunction. Extend semantic fixed-point state to all channels; preserve origins, modality, phase, parallel calls and per-alternative refusal recovery; effects before raises use prefix evidence | Finite/base-free/self/mutual recursion, shuffled bytes, swapped/captured formals, later predicates and original typed caps. Compiler101 extracts shared frontier/queue/alternative progress and retains nondominated depth/cost witnesses for the value consumer. Call-specific multi-control stability, residual conjunction, all-channel applicability/obligation keys and composition remain open |
| S4 engine | Compare current worklist, Ascent and datafrog on the actual shared semantic/refusal/witness contract. Keep ADR-0053 unless another conforming engine removes total machinery; measure build/preparation/run/memory after parity, record an ADR before replacement, delete superseded production path | No timeout treated as deterministic budget; no dual production engines. Comparison open |
| S5 | Compose claim-specific source/model/callee/handler/exit coverage and discharge only matching origins. Remove unconditional strongest-transfer pruning; preserve kind/condition alternatives through value flows and receiver consumers. Move existing RCA attributes to typed meaning/modality/evidence and one renderer; reject unknown forms | behavior_shapes part 2, positive/open siblings, empty complete channels, transfer/lambda/decorator controls, candidate/definite RCA and relabeling invariance. Transfer and source parameter-identity repair implemented with focused source/Delta/native controls. Current typed RCA/Pass C repair implemented and focused Tested (2026-09-27); claim-specific coverage and assembled qualification remain open |
| S6 | FORMAT 9 structural proofs/coverage/support; typed effect, role and exact-entry-input conjunctions on existing operation queries. Rust canonicalizes and evaluates; Python adapts. Conditions/scopes of conjoined witnesses must be compatible. Partition matched/excluded/source-open/unexamined; preserve path-local inspection. Delete FORMAT 8/native adjacency interpretation | FORMAT 9 callable support and bounded value/source-normal closure are implemented. Native actions/defaults/postconditions/invocations, full raw-source evidence closure and compatible effect/role/exact-input semantic queries remain open; matched/excluded/source-open/unexamined partitions must stay distinct |
| S7 | Extend isolated CPython/Hypothesis comparisons to actual compiler completion, summaries, coverage and served results; use release binaries. CrossHair distinguishes exhaustive/bounded/counterexample; Pysa TITO uses real rules and pinned Pyrefly on identical source. Close original W5 work/node, W7 reach-work and W12 pair-work default-cap producer→Delta→native traces | Independent challenges accompany S2–S6. Original default-cap traces are separate from injected limits. Open |
| S8 | After S1–S7: fmt, test-all including all-techniques digest, fresh-store pilot, structured Stage 3 packet with semantic queries, clean-wheel query, controlled live W9/W16 replay/conformance, cost measurements, assembled target review, docs-check and handoff | Current checkpoint fmt, full tests, all-techniques digest and ordinary fresh pilot are recorded in §1.1. S8 remains open until functional completion, semantic exit assessment, clean-wheel/live checks, controlled costs and assembled review; frozen exit rule unchanged |

**Done bar (operator, 2026-09-27): exit-driven.** A step closes on its named acceptance, plus
what Q01/Q03/Q05/Q09, `behavior_shapes` part 2 and the largest pilot `call_transfer` sources
need. Other widening becomes a §7 row with a trigger. `override_dispatch` stays a named unknown.

**Remaining sequence (operator-approved 2026-09-27).** Phases map to the S-steps above. Each
phase ends with a handoff; full gates run only at P7, apart from one diagnostic pilot after P5.

| Phase | Work (S-step) | Key acceptance |
|---|---|---|
| P0 | Rubric rule, evaluation-only requests, packet digests and guard (done, `2c6a59c`); baseline diagnostic (§1.2); these amendments; S6.0: the extension owns the native file list and row caps (`native_files()`). The remaining `ipc_input` positional tuples convert to cpg-schema rows with P6.1's loader extension | Guard test; no product semantics changed |
| P0.7 | **Precision repair (operator, 2026-09-27).** Builtin descriptor exemption from decorator withholding (compiler 103; behavior-model §"Value flows and call transfers"). The `missing_evidence` shift is diagnosed as ty's AMBIGUOUS reachability inside or after `try`, `with` and loops over unknown iterables. F07 withholds it deliberately; a precise repair needs completion-kernel reachability certificates for non-return sinks (§7) | `transferpkg.Descriptors` positives; stacked and shadowed `classmethod` stay withheld |
| P1 | **S5a discharge (ADR-0064; target review F01–F07 adopted).** (1) The summary producer refuses decorated local targets and decorated bases using `flow_model`'s one predicate (F02, a live defect). (2) Pure `summaries::discharge` decide and grade. Claim-keyed `behavior_discharges(behavior_id, origin_id, proof_kind, summary_id?, reason?)`; membership from the `flow_model` merge; an all-members grade after the merge (F03); proofs discharge certified value approximation (F04). Replace `semantic:call-transfer-never-established` and add sibling closure (F06). Validator equality from the existing finite outcome. Generation export, native citation admission and Python citation rendering | `through_decorated` has no summary. `local_wrapper` is established and cites its summary. An open sibling or a mixed row stays unknown. Order invariance holds. A doctored generation is refused. Pilot unchanged by construction (review O1) |
| P2 | **S7 foundation.** `lctx compile-fixture`; a composed Hypothesis/`sys.monitoring` harness against served results; the original W5/W7/W12 default-cap fixtures with no injected limits; `just oracles` | Each cap reason traced through Delta, bundle and native |
| P3 | **S3b logging and S1 contracts (review F08–F10).** Additional `Logger.*` methods (`warning` already applies at 87 of 85 source sites, so no receiver-binding miss). `SummarySubjectKind` CallableFormal/CallableEntry with `coverage_domain` as the single authority that generates the SQL checks (F09). CallableFormal complete-empty coverage only for closed immutable constant returns plus None fallthrough, with no escape (F08). `summary_effects` + steps with a distinct call-local subject and per-channel admissible step kinds (F10); value/transform/constant stay in `summary_flows` | Complete-empty for `_describe.name`; the `acc.append(p); return acc` counterexample stays incomplete; no fabricated value IDs |
| P4 | **S4/S5.** Generic `worklist::drive_component`, extracted together with the effect adapter (review F11); call-specific stability barriers (ADR); residual conjunction (append-only boundary reason); per-contribution refutation with `behavior_shapes` part 2 fixtures; call-crossing origin coverage; engine comparison on the shared contract | `labelled.name` refuted; `quiet` refuted; `loud` established |
| P5 | **S3b families.** Ranked: builtin constructors; isinstance/len/getattr/hasattr; `list.append`/`extend` effect-only; `functools.partial`/`wraps`; starlette `Middleware`/`Route`; io/pathlib/zlib. Explicit candidates with reasons: pydantic, contextvars, async and uvicorn. **S2b/S3a:** `open` acquire/release pairing and distinct callback fates | Per-target pins, fixtures, catalog equality, CrossHair for definite pure rules |
| P6 | **S6.** Export behaviors, conditions and every summary/action/execution relation. Native `operation_semantics` returns matched/excluded/source-open/unexamined partitions under exact inputs. MCP `Where`/`OperationSet`/`Operation.semantics`. The packet renders semantics. `just wheel-check`. Pysa TITO on identical source | Composition F04 and W11 operation-wide Q09; clean-wheel query |
| P7 | **S8** as above | Exit rule under the committed rubric; `behavior_shapes` part 2; assembled review |

**Contract boundaries and defaults.** Contracts/source observations belong in schema/providers;
pure semantics in analytics; acquisition, order and publication in core; bounded immutable selection
in native Rust; protocol/rendering in Python. No general interpreter, provider framework, alias
analysis, registry, solver migration or Stage 5 deferred execution is added. Existing pins and
limits remain defaults. Async models assert only proved call/construction phases. Missing mappings,
implicit protocols and incomplete coverage stay explicit unknowns.

**Scope additions for the design standard.** S1 represents complete empty channels without fake
value IDs (FP-04/05, DP-02/03, CI-04), shares actual proof admission (FP-02/03/04), and authors model
phase/schema meaning once. S5 preserves transfer alternatives and current RCA fidelity (DP-07/08,
CI-02/04/09); Stage 4 registry, behavioral FCA expansion and technique retention remain deferred.
S7 challenges production results independently, not merely shared reconstruction (DP-22/23, CI-12).
S6 and the evaluation packet exercise the new semantic interface and evidence closure (CI-11).

**Verification and completion.** During S1–S7 run focused release compile/behavior/probe checks
with `INSTA_UPDATE=no`; snapshots are reviewed migrations and codebooks append-only. Run `just fmt`
and the integrated product gates at S8. Rebuild current stores under ADR-0048 without copying old
analysis tables; verify the independent embedding-cache contract before reuse. The clean wheel runs
outside editable checkout imports; fake vectors never qualify live embedding. Runtime workers execute
only generated programs, never `fixtures/python/` or the analyzed pilot. Heldout stays sealed.

The fixed Stage 3 set has **23 positive and five negative items** (Q01/Q03/Q05/Q09): at least 12
positives present/partial, none incorrect/misleading, and no negative claim stated; behavior_shapes
part 2 also passes. Add evaluation-only semantic requests without altering targets or the rule.
A1–A3 and G1–G8/CI-G1–CI-G3 are settled independently at the assembled target review. A failed exit
never silently relaxes the criterion or imports Stage 5 work.

Small scoped commits stay on current main; preserve concurrent edits and never push unasked.
Delete migrated competing semantic authorities and retire documents only after their last current
consumer is gone. Each checkpoint updates this queue/§6 and a restart-safe STATUS without calling
focused passes integrated qualification.

**S4 scheduler foundation (Implemented and Tested, 2026-09-27; compiler101).** The existing value consumer
now uses shared `summaries::worklist` frontier, component queue and alternative-progress state.
The frontier retains nondominated depth/expanded-proof-cost representatives: an equal-depth
cheaper proof can reopen work, while an equal-cost alternate witness creates no recursive work.
A successful semantic alternative survives a later limited witness; unrelated refused alternatives
remain open. Canonical queue order, parallel occurrences and the original depth 8, proof 64 and
component pair-work limits are preserved. The old value-only frontier/queue/progress ownership is
removed. Formatting and strict-lint repairs include grouping return-value premise scope and
direct-proof inputs; these do not introduce new semantic channels or table contracts.
The three scheduler controls and existing finite regressions pass in the full checkpoint gate (§1).

**Pilot-triggered context repair (Implemented, 2026-09-27; compiler102).** The fresh real pilot
exposed repeated library/corpus observations of `json.dumps.obj` being counted as duplicate
formals. Shared signature-slot agreement now serves model formal validation, constructor binding,
call evaluation and the SQL model-argument projection. Repeated module/definition observations
bind one exact pinned target; incompatible payloads and repeated fact IDs remain errors. Raw observation facts stay
published; selected representatives retain real fact IDs. This repairs existing model application,
not the remaining family activation scope. Pure/SQL agreement and drift controls accompany it.

**Next S1/S4 dependency (Accepted target under ADR-0057/0058; Proposed implementation).**
Checked source/model targets and independent fresh header/binding admission are implemented
in compiler100 below. Lift typed non-value channel payloads through the scheduler, preserving
leaf origins, call occurrences and independent outcome obligations. Shared helpers alone do not
implement all-channel composition. Keep per-alternative refusals and coverage independent.
Migrate and delete value/return-shaped
assumptions for nonvalue consumers; retain current candidate relations used by RCA. Resource
identity, generalized outcome disjunctions or new authored role meaning need their own decision;
simple propagation of existing typed actions does not.

**S2b/S3a bounded checkpoint (2026-09-27; Implemented and focused Tested).** Catalog format 4
binds the CPython 3.14.7 class, allocation and initializer roles independently. Fresh synchronous
contexts retain ordered construction, successful entry registration before `as` assignment, reverse
cleanup, suppression only on Raise and exit replacement. Constructor mismatch, failed unpacking,
matching/nonmatching and return/break/continue controls distinguish outcomes. Unsupported
argument evaluation remains unknown; these models have no failing-entry assertion. A separate source/model certificate now proves a bare return of the unique `as` binding inside
its active context when the explicit entry argument is an immutable current parameter. Raw
provider flags remain unchanged; the distinct value witness binds both lexical reads, exact
site/argument, successful assignment, later exit and return-condition scope. Omitted value
proofs are rejected independently of completion. Broad entry domains remain open. Every production value summary requires a source-derived commitment
to all ordered entry/exit obligations, including empty ones and condition scope. Publication
reconstructs it; native checks missing/duplicate/swapped/omitted evidence. Typed kernel limits
survive proof admission. The original 64-step budget remains fixed; fixture finalizer counts adjust
for the now-explicit exit anchor. Fourteen isolated CPython lifecycle cases independently challenge
the focused source/Delta/native paths ([evidence](../design_review/evidence/2026-09-27_sync-contexts/README.md)).
Full S2b/S3a, S6 and integrated S8 acceptance remain outstanding.

**Named-handler boundary (2026-09-27; Implemented and focused Tested).** Entering a named
handler emits append-only `handler_name_cleanup` (code 33), including a selected handler before
a later return and a named handler in a finalizer. The specific refusal survives Delta/native;
an unselected named handler does not create it. This satisfies S2b/S3a's proof-or-specific-boundary
obligation; general binding/deletion completion remains unsupported. The focused six-case selection
passed five cases, then its SQL assertion typo was corrected and the affected source case passed
on replay. This does not close resource/callback or assembled-stage obligations.

**Reached-call input checkpoint (2026-09-27; Implemented and focused Tested).** Compiler 92
persists independent statement-prefix and invocation groups for a sole pinned bare imported
call. The shared binder/evaluator serves normal-return and invocation proofs; callee entry never
asserts callee return, effects or callback execution. Exact requiredness is mandatory except that
variadic formals cannot be required. Positive calls retain the 128-argument/64-step bounds;
oversized refusals retain their actual count and `invocation_argument_limit`. Publication
reconstructs both groups. Thirty-four focused cases and 13 isolated generated CPython programs
passed ([evidence](../design_review/evidence/2026-09-27_call-entry/README.md)). Native export,
action timing is now composed in the bounded checkpoint below; returned-resource identity and active context frontiers remain open.

**S5 modeled identity correction (2026-09-27; Implemented and focused Tested).** Compiler 93
restores `framed_modeled_identity` and nested-pass-finalizer positives through an independent
lexical input certificate composed with the existing exact model-call proof. Raw provider flags
remain unchanged. The shared lexical checker serves direct and modeled returns; the source
adapter owns raw-use/call-argument association. A value basis is mandatory independently of
completion, including after deletion of the whole model group. FORMAT 9 admits the same
commitment and rejects missing/foreign/duplicate/reordered support. An adjacent context-entry
value after a total call remains valid. Thirty-nine cases passed in the 40-case selection; an
accidental fixture-import edit was corrected, the fixture parsed, and its affected native case
passed on replay. Seven independent generated object-identity controls and 16 Python
schema/native cases passed ([evidence](../design_review/evidence/2026-09-27_modeled-identity/README.md)).
The original modeled-return test now passes unchanged; complete source/model/callee coverage,
approximate chains/assignment paths and full S6 raw-fact semantics remain open.

**Authored action triggers (2026-09-27; Implemented and focused Tested, ADR-0060).** Compiler 94 /
catalog 5 persist an assessment for every effect/callback/resource candidate. Potential partial
I/O can activate from reached invocation independently of normal return; completed serialization,
compression and registration retain unproved-outcome refusals. Authored subjectless effects differ
from unresolved required subjects. Shared admission checks exact model application, call, phase,
condition, binding/argument support, ordered invocation and any callee outcome; the normal proof
extends that same invocation and retains the original 64-step cap. A call-expression ID never
becomes a returned-resource identity. Publication reconstructs action and upstream source proof;
normal/Finally positive controls are contract-only, not production model activation. Independent
nine-program CPython partial-I/O evidence is retained in the [challenge](../design_review/evidence/2026-09-27_action-triggers/README.md).
Native action support, broader bindings/default domains, resource/callback
positive fates, complete coverage and all-channel composition remain open. This advances S1/S3;
it does not close S2b/S3b/S4/S6 or integrated acceptance.

**Pinned call defaults (2026-09-27; Implemented and focused Tested, ADR-0061).** Compiler95/catalog6
separate availability from normal return. Individually qualified JSON dump/dumps/loads and gzip
compress defaults support reached invocation; the same fallible calls retain outcome refusals.
Binding derives omitted-formal commitments independently of retained proof, preserving each
signature's fact IDs. Shared admission rejects absent, reordered, foreign, extra and oversized
default groups before allocation. All-signature agreement and known requiredness remain mandatory;
availability supplies no value, stability or source argument. Removing the authored premise from
the real source applications produces DefaultUnavailable and invalidates publication. The final
33-case release selection passed, plus seven generated pinned-runtime entry controls
([evidence](../design_review/evidence/2026-09-27_pinned-defaults/README.md)); native actions,
nested default-proof ownership and broader local/default/model domains remain open.

**S3 outcome contract (Implemented and focused Tested, 2026-09-27; compiler96, ADR-0062).** Fallible
models' Normal rules now have explicit normal-exit postconditions, separate from activated
assessments. Shared invocation/binding admission checks both; a mandatory undischarged exact-callee
outcome obligation must survive future composition and S6.
Returned-resource endpoints remain symbolic until independent identity/fate evidence exists.
This adds useful model-relative implications without weakening proved triggers or asserting
that normal completion is feasible. Value-independent source/local-callee completion and concrete
resource/transform fates remain required; this relation alone closes none of them.
The final 36-case release selection passed, including seven real source implications, bound
subject/obligation mutations, publication deletion/replacement and original 64→65 proof controls.
Explicit `open` retains its natural `summary_proof_limit`; symbolic-resource positives are
contract-only. Six generated runtime cases independently challenge entry/outcome/action distinctions
([evidence](../design_review/evidence/2026-09-27_normal-postconditions/README.md)). No native or
assembled Stage 3 acceptance is claimed.

**Frame-completion correction (Implemented and focused Tested, 2026-09-27; compiler97,
ADR-0063).** Catalog7 replaces unqualified normal return with a typed direct-return body and a
complete bound-argument release certificate. Closed values and caller-retained roots are distinct;
class-body custom namespaces and omitted-default release remain withheld. Invocation and qualified
Normal postconditions remain independent. Ordered operand/formal segments, original proof limits,
and mandatory root/nested frame obligations survive direct/chain/assignment summaries and native
serving, including raw identity paths. Shared publication reconstructs the source domain; native
checks the committed body, structural/digest closure and dependency ordering. Full raw-signature,
read-provenance and action closure remain S6. The 69-case release selection and refreshed 13-case
Python native replay passed; an earlier replay correctly rejected its stale fixture. Ten generated
CPython controls independently observe body return before cleanup and delayed caller continuation
([evidence](../design_review/evidence/2026-09-27_frame-exit/README.md)); none is claimed as a
compiler false-positive reproduction. Source/local-callee bodies must next reuse the completion
kernel with separate closed release evidence; broader cleanup remains open.

**Value-independent source bodies and fresh calls (Implemented and focused Tested, 2026-09-27;
compiler100, ADR-0063).** The existing statement kernel evaluates a zero-formal callable's
suite separately from definition headers and value-flow seeds. Persisted body outcomes distinguish
Normal fallthrough, Return, exact TypeError and unknown; closed local releases remain a separate
obligation. Every row still requires the exact function object to remain externally retained.
Defaults, captured/dynamic/decorated/async scopes and unknown owned values are withheld; a normal
local read keeps `frame_exit_cleanup` until linked to its executed closed initialization.
Docstring-only bodies have zero runtime statements; docstrings never become runtime release
inputs. Shared admission anchors terminals and ordered expression-root segments, while source
publication reconstructs all three relations. The fresh nested-call adapter separately proves
the live definition-binding retainer, exact zero-argument binding, successful header creation,
Normal/Return body and closed release/result domains. It consumes base bodies in an explicit
acyclic stage; caller enrichment never feeds its own premises. The original expanded 64-step
bound applies to expressions, summaries, invocation prefixes and Normal/Finally action proofs.
Normality under entry remains distinct from reached invocation. `source_call_bindings` now owns
the independent creation/binding contract; reduced normals reference it and a qualified body.
The existing reached-call owner has explicit Source/Model targets and does not manufacture a model
ID for source calls. A raising, captured or unknown body can have an admitted source invocation
without normal completion. Failed headers, unsupported default/binding forms and unreachable
callers withhold invocation; a separate callee-owned yield index excludes both reachable and
unreachable yields. Source/model invocation conclusions cannot satisfy completed evaluation.
Duplicate-aware binding/normal/call-site indexes avoid repeated global scans.

The final 87-case focused selection passes with snapshot updates disabled, including real source,
Delta and 25 native controls, source invocation variants/original cap boundaries, adjacent
value/default/action cases and current schema projections. Thirteen Python native tests and
14 independently generated CPython entry/outcome controls pass
([evidence](../design_review/evidence/2026-09-27_frame-exit/README.md#reached-source-invocation)).
Native exports only binding support for served normal certificates; source-invocation querying,
full S6 evidence closure and integrated Stage 3 remain open. Exact-false derived projections
remain absent while unknown conditions stay explicit. Next compose typed non-value subjects
through the shared scheduler, preserving invocation versus outcome obligations, origins,
modality and phase; broaden binding/default/release domains and model families as scheduled.

### 3.1 Contract repairs before extension

| Order | Work (§6 item) | Targeted acceptance |
|---|---|---|
| A1 | Native proof-kind admission from the schema contract; one owned typed decoder for generation tables (W1: ARC-01, RF/F13) | A finalizer-bearing generation loads and queries through the real native executor; an unknown kind is rejected; a schema drift test between generation and native reader |
| A2 | One resolved attribute-access contract (W4: RFU/F05, RF/F09) | Bare, qualified and aliased builtin access, computed names and shadowed builtins under the real provider; qualified/aliased literal reads withhold no-read claims; one corrected premise traced through publication and serving |
| A3 | Served claim fidelity (W2: RFU/F02; W3: RFU/F03) | A facet with established and unknown values keeps both in lookup and agrees with filtering; a finding-only claim is followed to its model and source span in the served generation, identically in structured and Markdown forms |
| A4 | Explicit summary inputs and typed refusals (W5: ARC-02, ARC-03) with the next normal-completion witness | The production transformation runs without extraction, embedding, Delta or server setup; an actual depth/BDD cap and an unsupported case keep specific causes through producer, snapshot and native response |
| A5 | `Model::reach` fixed point with a work bound (W7: RF/F05) | The query-order reproduction gives the same contributions A-first and B-first; the acyclic and finite-base controls hold; a dense SCC hits its cap with a boundary; shuffled input is byte-identical |
| A6 | Kernel decisions, effective support and aggregate preparation bounds (W6: RF/F01, RF/F02, RFU/F01) | The tests named in W6; a bounded refusal stays unknown |
| A7 | Analyzer input identity and canonical cache fill (W8: RFU/F07; W9: RFU/F06) | The helper edit/addition and cache race/over-cap controls named in W8 and W9 |

A1–A5 precede further proof-kind, native or SCC extension. A6 precedes raising catalog limits or
extending summary catalogs. A7 precedes a hermetic corpus/run-identity or cache-conformance claim;
it is independent of A1–A6.

### 3.2 Functional completion

| Order | Build and proof boundary | Targeted acceptance |
|---|---|---|
| 1 | **Call-site and predecessor evaluation.** For each argument and preceding statement on a proposed return path, distinguish an evaluated direct value, a cited normal outcome, a possible raise and an unresolved expression, tied to Ruff syntax, ty use/region facts and the call/return sites. Preserve evaluation order; a modeled target's normal return starts after its arguments. A true return region does not prove that an earlier call returned | A direct formal and a raising sibling argument diverge; a terminating branch before recursion and an unconditional self-call before return diverge; the shared validator rejects a forged completion row |
| 2 | **Exit and L2 fate.** Nested `try`/`finally` and `with` frame order, normal/exceptional completion and suppression; callback stored/invoked/forwarded/registered and resource acquire/release only with an execution and exit witness. Generators/coroutines stay at the Stage 5 deferred-execution boundary. ty's `end_of_scope_reachability` may add one append-only implicit-exit kind | Nesting, early-return, re-raise, suppressor, callback-not-run and unreleased-resource cases; lexical containment never substitutes for execution |
| 3 | **Proof identity for composition.** The canonical ordered proof identity (ADR-0045) extends to the call/model/flow/exit steps order 6 composes, retaining parallel paths without duplicating prefixes | Two calls sharing endpoints, reordered inputs, a forged or missing step |
| 4 | **Positive modeled results.** Admit an exact whole-return identity path only when the target set is closed and sole, modalities are definite, all arguments reach the call, the pinned target asserts normal return, the condition is admitted and the return exit is proven. An assignment predecessor follows only when its reaching definition is unambiguous and condition-compatible; otherwise a specific `summary_boundaries` reason | Direct, shadowed, nested-call, fallback, alternate-target, raising-argument and assignment cases; a forged summary is rejected |
| 5 | **Model families** (§3.3). Record each target's exact pin, formal mapping, independent channel coverage, invocation phase and a normal-return assertion only when source-backed. A family without an in-scope consumer stays a candidate model | Per-family positive/withholding fixtures; model-catalog equality; CrossHair for pure models in an isolated worker (a timeout or unexplored path is inconclusive) |
| 6 | **Finite interprocedural summaries.** Replace the recursive-member refusal with bounded SCC composition that admits a cited finite base path; compose value/transform/effect/exception/role paths with BDD conjunction and declared modality. Depth, node, pair-work and iteration caps write explicit boundaries; an open override blocks negative closure. The first value-channel engine is the ADR-0053 SCC-local worklist; SCC routine and ownership by W13. Revisit engine reuse when multiple channels share recursive rules | Acyclic, terminating base, self- and mutual recursion, parallel edge, open override and cap fixtures; shuffled input byte-identical; a cap's specific cause reaches `summary_boundaries` |
| 7 | **Claim discharge.** Reconstruct `summary_flows`, `summary_effects` and `summary_boundaries` through shared validators; `call_transfer` becomes established/conditional only with the matching proof; a refutation requires complete source/model/handler coverage | `behavior_shapes` part 2: logging-only and disabled-option wrappers; unrelated or ambiguous candidates stay unknown |
| 8 | **Independent challenge** (W11's independent controls). CrossHair `diffbehavior` for pure models; Pysa TITO with a real source→sink rule and the pinned Pyrefly binary; CPython 3.14 `sys.monitoring` with Hypothesis for flow, exception and exit admission, including an ignored predecessor followed by the cited return. Disagreement becomes a fixture or an explicit boundary | Targeted oracle cases while implementing; the preregistered full comparison at the integrated end. Oracles never write facts; the gold never enters analysis |
| 9 | **FORMAT 9 native serving** (callable-support subset implemented; remaining S6 semantics open). Validated structural condition/proof/summary tables and cited support closure, one immutable PyO3 executor per process, typed compatibility plus effect/role filters. Resolve the operation/formal and exact primitive origin before BDD evaluation; string membership and integer equality (W11) are needed for Q09. Unsupported proof, caps and cursor exhaustion return explicit unknown/truncated with work accounting | Editable-import and native query cases, then one clean-wheel generation-pinned query after functionality lands |
| 10 | **Integrated exit.** `just fmt`, `just test-all`, fresh-store `just pilot`, Q01/Q03/Q05/Q09 and the structured Stage 3 evaluation, the clean-wheel query and the increment-end design/target review | Each command `passed`/`failed`/`blocked`/`not_run`; `call_transfer` before/after, condition budget counts, compile time/RSS and serving bounds. A material fix repeats only the affected gate plus invalidated acceptance |

**Order 1 checkpoint (2026-09-26; partial).** A direct `return value` may now cross an earlier
`typing.cast(object, 1)`, `typing.cast(object, -1)`, `typing.cast(object, not False)` or `typing.cast(object, value)` call with a
sole closed target, two
source-ordered argument witnesses, an earlier unconditional module-level `from` import of the
callee, a non-approximate
ty call region and a pinned normal-return assertion. The proof contains the exact import
binding/region, resolution, argument, site, Pysa target and model identities; the shared validator
rejects removal of its `preceding_call_normal` step. An otherwise identical call with `1 / 0`,
`-(1 / 0)` or `not (1 / 0)` as a sibling remains `unsupported_control_flow` in the published/native answer, as do a possibly
deleted parameter or possibly unbound local argument, conditional local import and guarded module
import. Parameter and assignment names normally require one exact ty reaching definition of the same
lexical binding. For a direct parameter read inside an approximate `try` region, the shared
classifier has a separate lexical witness only when the reference resolves to the current
function's parameter and that function has no explicit deletion or exception handler. The
one-call modeled source now retains its raw value fact and this independent normal-read evidence
in a schema-migrated row; the finite proof cites both ([ADR-0055](../adr/0055-modeled-source-read.md),
[scoped review](../design_review/reviews/design_review_modeled-source-read_2026-09-26.md)).
The classifier also admits a parameter in an existing modeled-return
fixture without treating a shadowed builtin spelling as a builtin. A direct return on a
terminating branch survives disjoint recursion on the other branch, while an unconditional
self-call before a direct return withholds it. Two completed source-ordered calls now leave
two separate proof steps; a completed call followed by a raising sibling stays unknown. The
pure producer sorts incoming predecessor rows before its stop-at-return scan, so a shuffled
later call cannot hide an earlier unproved call. The modeled direct-return producer now uses
that same earlier-call witness: a preceding pinned total call is cited, while an opaque earlier
call leaves an explicit unknown even when the returned model call is exact. The assignment-then-return
route uses it too, including its source call; a later opaque call blocks that path. These positive and withholding controls passed
through the real provider and native generation. A local wrapper now uses the same earlier-call
check before composing the callee's cited summary; an opaque predecessor blocks it. One explicit
keyword mapping now passes the same proof, while `**` unpacking remains `call_transfer` unknown.
The closed-expression classifier now has a pure owner: `lctx_analytics::evaluation` composes
nested unary/numeric, Boolean-chain and conditional expressions from the placed syntax, with
independent normal/value outcomes, default depth/work bounds and source reconstruction. The
shared argument adapter serves preceding calls, model siblings and local controls; the previous
fixed-shape SQL classifiers were deleted. Default expression cap outcomes survive a local-call
origin through Delta and native, and selected raising operands remain unknown. See the
[foundation review](../design_review/reviews/design_review_stage3-foundations_2026-09-26.md).
An exact whole-return chain of two or three closed total `typing.cast` identity models now
composes from the ordered raw call path, one bound source argument per step and normal-evaluation
witnesses for every sibling. The innermost direct formal read must have either an exact ty reaching
definition or the bounded lexical parameter witness, cited alongside the raw flow fact;
a deleted formal has no parameter-origin path.
Its inner call proof is inserted where its outer argument evaluates;
the same pure relation is bounded to eight steps and 128 argument rows. The real provider,
shared validator and native generation were checked with two- and three-call positives and a
raising inner sibling; [the scoped review](../design_review/reviews/design_review_modeled-call-chain_2026-09-26.md)
records the boundary. This is narrower than order 1's exit:
callee forms other than an unconditional module-level import,
default values, unsupported predecessor protocols and the remaining handler/context entry
sequence remain to implement. Condition-keyed predecessor certificates now check supported
non-call statements, source-origin branches and first local initialization. A target's normal-return assertion alone never certifies its arguments.

**Order 2 checkpoint (2026-09-26; Implemented, partial).** Pure statement completion now
distinguishes normal, return, raise, break, continue and unknown. Pending finalizers preserve or
replace outcomes in frame order, retaining full operand and model evidence. Normal expressions,
unique local initialization outside loops, exact selected branches and nested try/finally are
supported; opaque exception construction and rebinding/finalization stay unknown. A first bare
handler can handle an exact invalid primitive raise. Ordered pinned typed handlers and active
re-raise now compose that exact TypeError; MRO nonmatch requires retained native completeness. The
[bounded review](../design_review/reviews/design_review_stage3-expression-completion_2026-09-26.md)
identified producer/native proof-budget mismatch and implicit execution (F01/F02); corrections
share the 64-step cap and restrict completion admission. The
[independent controls](../design_review/evidence/2026-09-26_expression-completion/README.md)
passed 120 generated cases and recorded two started implicit actions with no completion before
timeout. The subsequent typed-handler extension passed eleven focused pure/source/Delta/native
cases and 100 independent runtime controls. Handler/else/finalizer entry now composes through the shared try-body owner. Named-handler cleanup,
exception groups, broader synchronous context models, broader entry-result domains, callback execution and resource fates remain open. The S2b/S3a checkpoint above owns the implemented pinned lifecycle subset.

**Order 3 checkpoint (2026-09-26; partial).** Schema-owned normalized local arguments separate
syntax order, formal binding, normal completion and exact value. Up to 128 explicit arguments
are checked as a complete group, and missing required formals refuse. The tracked raw source
and normal-read witness have separate ordered proof steps. Reversed keywords and three-argument
calls are exercised through pure, source and native checks; shared reconstruction checks the
entire proof. Control-group admission is shared by analytics and native.
The bounded modeled-chain producer now interleaves each inner call at
the outer argument's ordinal, with two- and three-call real proofs and a shuffled pure input
control. General composed call/model/flow/exit step ordering across other channels
and parallel-path identity remain open.

**Order 4 checkpoint (2026-09-26; partial).** Closed, sole, definite pinned total identity
models now support exact nested whole-return chains with every explicit sibling evaluated and
the original source origin retained. A raising inner sibling and a nonidentity inner call keep
an explicit unknown. Effectful expressions outside the admitted total-call models, other model
families and complete path-specific predecessor evaluation remain open.

**Stage 3 exit:** `behavior_shapes` part 2 passes and Stage 3's pre-registered exit rule (Q01, Q03,
Q05, Q09) passes; then increment 4's assembled design/target review.

**Carry-forward constraints.**
- Keep exact source argument/target counts and the shared reconstruction validator for proofs.
  No generic `return x` positive after an unproved call.
- Finalizer proofs require completed ordered actions; unsupported implicit protocols and `with`
  stay unknown until execution and suppression are cited.
- A Pydantic `TypeAdapter` selects its schema at runtime. Do not populate the named-schema
  `validate(schema)` action with the adapter class name; a typed dynamic-schema case, with a source
  witness where one can be proved, precedes any validation-effect or negative-coverage claim.
- The BDD-incompatibility screen for earlier calls is interim; order 1's condition-compatible
  predecessor witness replaces it. Missing, approximate or capped evidence stays an explicit unknown.
- `inspect_value_paths` is path-local inspection, never an operation-wide verdict.
- Accepted snapshots are reviewed migrations; the current checkpoint gate and fresh pilot are
  recorded in §1.1; assembled Stage 3 acceptance remains outstanding.

### 3.3 Models catalog v1

Authored from pinned library source and official docs, append-only model ids, typed data with the
catalog digest in `compiler_digest`; no model cites `.claude/skills/`.

| Family | Targets |
|---|---|
| Pure identity/value | `typing.cast`, `typing.assert_type` (built); `str`, `dict`, `list`, `tuple`; `functools.partial`, `functools.wraps`; pilot-driven builtins (operator, 2026-09-27): `int`, `set`, `isinstance`, `len`, `getattr`/`hasattr`, and `list.append`/`extend` as effect-only receiver mutation with no content flow or alias claim |
| I/O and serialization | `open`/`io`/`pathlib`, `json`, `gzip`/`zlib`, logging |
| Async, timeouts, context | asyncio and anyio (`fail_after`, `move_on_after`, `to_thread`, task groups); `contextvars`; `contextlib` |
| Validation and settings | pydantic `BaseModel`, `Field`, `TypeAdapter.validate_python` (dynamic schema, above); pydantic-settings `BaseSettings` (environment prefix) |
| HTTP and servers | httpx timeouts; the starlette and uvicorn entry points |

## 4. Stages 4 and 5

### Stage 4: the capability registry

1. **Registry** in `cpg-schema` (TOML with `deny_unknown_fields`, compiled to Arrow): append-only
   concept ids, labels with their sources, `broader`/`related`, scope notes, facets. Definitions
   are a Rust enum AST compiled to DataFusion SQL and digested; definitions over conditions use
   the kernel's compatibility and implication.
2. **Integrity:** `broader` is acyclic (DataFusion `WITH RECURSIVE` with `UNION`); `related` is
   disjoint from the `broader` closure. No per-language label rule.
3. **Vocabulary** seeded by a one-off script whose output the operator reviews: Stack Overflow tag
   synonyms (candidates only; they merge opposites), Wikidata and EDAM `closeMatch`, method
   stereotypes. Author 20–40 concepts.
4. **Membership:** `concept_members` materialized with role bindings, condition, verdict and
   witness; every member cites a definition digest and a witness; a test shows FCA never writes
   membership.
5. **`lookup_concepts`** lists every concept while the catalog is small; ranked lookup (bm25s,
   PyStemmer, vectors, RRF) waits until it outgrows one page.
6. **`explain`** returns the stored witness chain: rule id, premises and spans; each derived row
   stores its rule id and proof height.
7. **FCA over behavioral attributes** per structural scope, as candidate facets. RCA only if the
   structured evaluation shows value, and only after W10 gives attributes typed meaning.
8. **Query schema authority** (schemars → JSON Schema → pydantic) only if Rust needs the request types.

**Exit:** the structured evaluation of concept queries, with targets pre-registered in
`eval/behavior/` before this stage's output is read.

### Stage 5: frameworks, protocols, lifecycle

1. **Framework models:** registries dispatched by name; middleware `call_next` chains; lifespan;
   ContextVar places and state; function-object metadata such as `__fastmcp__`.
2. **Deferred execution:** a coroutine or generator's creation is distinct from its execution,
   suspension and completion; the read phase distinguishes "when called" from "when it runs"
   (Q07, Q08, Q11).
3. **Protocol operations**, on their trigger: operators, truthiness and formatting lowered to
   semantic operations with possible implicit calls, when an evaluation item turns on a dunder.
4. **Protocols as partial orders** mined from official usage by generalizing Pass C, labelled
   *observed pattern*, never *enforced protocol*.
5. **Rules parameterized by user code** (Q02, Q11): conditional records keyed on predicates about
   the user's annotations, coroutines and generators.
6. **Graph-FCA offline** on a bounded projection, judged by the structured evaluation; never in
   the pipeline.

**Exit:** Q02, Q06, Q07, Q08, Q11 and Q12 under Stage 5's exit rule. **Then, to close
increment 5:** unseal `eval/heldout/` and verify its manifest; write each task's target before
running anything; run the structured packet on the final generation and assess it in the same
rubric; decide the §B11 generative-model trigger; an assembled design/target review.

## 5. Evaluation and tooling

| Stage | Pre-registered questions (`eval/behavior/fastmcp-4.0.5.toml`) | Exit rule |
|---|---|---|
| 3 | Q01 (`tool` options), Q03 (`tasks=`, strict validation), Q05 (error masking), Q09 (transport option conflicts) | No positive item incorrect or misleading; no negative claimed; at least half the positives present or partial |
| 4 | Concept queries, written before Stage 4's output is read | Added then |
| 5 | Q02, Q06, Q07, Q08, Q11, Q12 | As Stage 3 |
| End of increment 5 | `eval/heldout/` | The same rubric |

The packet is `just structured-eval <generation> <stage> [embed_url]`; the author assesses it with
the rubric present/partial/absent/incorrect/misleading, per item, never as a percentage; no API
agents evaluate content. From Stage 3 on, a served `unknown` counts as partial only under the
qualified rule in `eval/behavior/README.md`. Evaluation-only requests
(`eval/behavior/fastmcp-4.0.5.requests.toml`) are rendered but never scored. The structured evaluation asks whether answers are useful and correct;
the validation lane asks whether the analysis admits what really executes.

| Library | Decision | Trigger or qualification |
|---|---|---|
| biodivine-lib-bdd 0.6.3 | **Expand** (W6, W11): bounded decisions, effective-support normalization, cube restriction | Keep structural ids and one union vocabulary; `check_binary_op`'s `None` is unknown |
| Ascent 0.8.1, datafrog 2.0.1, a small SCC worklist | ADR-0053 worklist is adopted for the first value relation; **compare** all three on the shared all-channel contract at S4 (W12) | Same semantic state and refusal contract for all three; timeouts are not deterministic budgets; measure compile/run cost after parity |
| OxiDD 0.12 | **Deferred migration** | A measured production case (pilot evidence that biodivine is too slow or large), compared on capacity, GC, retained memory and stable-id adapters |
| z3 0.21.1 on Z3 5.1.0 | **Deferred** | A registered query whose correctly linked theory constraints are impractical or inexpressible in the bounded finite lowering; it would sit behind `primitive_theory`, never become the kernel |
| fcars 0.2.2 | **Dev-only oracle** | Add a sparse context above 64 attributes if bitset coverage grows; concept-set agreement does not test attribute fidelity (W10) |
| rustworkx-core | **Keep bespoke** keyed ordering (W13) | A consumer for centralities beyond the current ones |
| Pysa, CrossHair, Hypothesis + `sys.monitoring` | **Oracles** (order 8) | Pysa's call graph is Pyrefly's, so only its TITO result is independent |
| PyStemmer, DataFusion `WITH RECURSIVE` (compile time), schemars | Stage 4 adopt | schemars only if Rust needs request types |
| Graph-FCA | Stage 5 offline spike | Never in the pipeline |

Avoid: crepe, DDlog, cozo, differential-dataflow; LinkML, OWL/RDF stacks, taxonomy induction,
BERTopic or UMAP in the pipeline; rust-bert, ort, fastembed.

## 6. Findings disposition

**Stage 3 channel review F17 — open.** Owner: schema model/frame contracts and analytics
expression/completion, with finite-summary and native consumers. ADR-0063 records the accepted
correction to unqualified pinned normal return. Its bounded compiler97 implementation and focused
qualification received **Accept scoped** in review §32 (2026-09-27): exact
binding/release-domain mutation rejection, preserved invocation/postconditions, original proof
bounds and root-certificate survival through Delta/native are checked in that domain. Compiler99
adds the separately qualified fresh zero-argument source-call domain above (**Accept scoped**, review §34).
Compiler100 separates reached source invocation from body outcome (**Accept scoped**, review §35);
broader source/release domains and full S6 raw-fact closure remain open. Source review and ten independent
runtime controls are in [review §31](../design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md#31-source-review-callee-frame-cleanup-before-normal-completion).


**Stage 3 channel review F18/F19 — corrected in compiler100, scoped Tested (2026-09-27).**
Owner: analytics source-invocation preparation and shared schema binding support. Independent
callee-owned yield indexing excludes reachable/unreachable generator bodies from eager entry
(F18); shared duplicate-aware binding/normal and call-site indexes remove the new global rescans
(F19). Source inspection plus the final 87-case source/Delta/native selection, duplicate support
controls and independent generated runtime cases supply closure in this domain
([review §35](../design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md#35-bounded-implementation-follow-up-source-invocation-independent-of-outcome)).
This is no whole-corpus performance measurement or deferred-execution implementation.


This table is the **single current disposition owner** for the findings below. Source reviews
keep their dated evidence and link here; STATUS links here. Qualified ids name the source:
**ARC** the retired 2026-09-25 core-3.0 calibration reviews (`design_review_v3_stage3_architecture_calibration_2026-09-25.md`
F01–F03 → ARC-01–03; `design_review_v3_nested_finalizers_calibration_2026-09-25.md` F01 → ARC-01;
`git show f54b09d:docs/design_review/reviews/<file>`); **RF** the
[reasoning review](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md);
**RFU** its [follow-up](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md),
whose §7.1 qualifications govern where the two disagree. Evidence is Interface-checked or Tested
as stated in the source; corrections are **Proposed**. A deferred item keeps its trigger; closure
needs the named evidence, never an accepted ADR or a moved row.

The [channel review](../design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md)
**F16 is corrected in the bounded pinned-default source/Delta slice (2026-09-27, focused Tested)**:
`cpg-schema::call_execution` checks count bounds before allocation and requires an exact root
default group, including when the independent omitted-formal domain is empty. Huge-count,
extra-marker and whole-group-deletion controls pass in the 33-case selection. Native action
admission and nested default-group ownership remain open under S6/S1.

The [channel review §21](../design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md#21-bounded-implementation-follow-up-synchronous-lifecycle-and-return-obligations)
**F11/F12 are corrected within the bounded synchronous slice (2026-09-27, focused Tested)**:
completion/schema/native owners bind ordered return obligations and preserve condition-limit
reasons; the context binder distinguishes unknown/bounded signatures from proved argument mismatch.
The 57-case contract/source/native selection and subsequent rebuilt-native replay provide closure
evidence for those corrections. The later bounded entry-value extension is recorded in §3.0.
The [channel review §22](../design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md#22-bounded-implementation-follow-up-context-entry-value-identity)
**F13 is corrected within that slice (2026-09-27, focused Tested)**: schema/analytics/native
owners require a value basis independently of completion and preserve source/model versus raw
identity. The 45-case selection and rebuilt-native replay close that correction; complete raw-fact
semantics remain S6. Broader entry domains and enclosing Stage 3 remain open.

The [channel review §23](../design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md#23-bounded-implementation-follow-up-reached-call-inputs)
**F14 is corrected within the call-input slice (2026-09-27, focused Tested)**: schema and
analytics preserve the actual oversized count on refused calls and apply the cap to positive
proofs only. The 34-case selection includes the 130-argument source and shared admission control.
The separately encountered modeled-finalizer positive regression is corrected by the §3.0/S5
source/model identity checkpoint.
The [channel review §24](../design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md#24-bounded-implementation-follow-up-modeled-return-identity)
**F15 is corrected within that slice (2026-09-27, focused Tested)**: schema/analytics/native
require the value basis from declared path depth, independent of retained proof kinds. The
current-native replay rejects complete model-group/certificate omission and admits a context
value with a modeled predecessor. Source equality preserves raw approximation; the original
modeled-finalizer positive and independent runtime identity controls pass. Complete S5/S6 remain open.

| Item | Component | Consequence and intended correction | Disposition · dependency | Closure check |
|---|---|---|---|---|
| <a id="ARC-01"></a>**W1 · ARC-01**, [RF/F13](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F13) | `cpg-schema` (`SummaryFlowStepKind`), `lctx_semantics` loader | Production loading forwards checked IPC bytes to the native decoder, which validates exact `cpg-schema` schemas and reads named fields; the tuple constructor remains for small synthetic tests | **Focused implementation passed** (`8781246`, `00073d3`): real finalizer-bearing generation queried through native executor, unknown proof kind and schema drift refused; integrated acceptance pending | A1's acceptance; an unknown-kind rejection control; a generation/reader schema drift test |
| **W2 · [RFU/F02](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F02)** | Python operation response (`operations.py`) | Lookup now returns typed `{value, verdict}` entries while preserving separate facet completeness; the prior response discarded each value verdict | **Focused implementation passed** (`1dfde60`): typed value verdicts remain separate from facet completeness; integrated serving acceptance pending | A mixed established/unknown facet agrees between lookup and filtering; the generated response schema shows the verdict |
| **W3 · [RFU/F03](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F03)** | Bundle support projection, Python rendering | FORMAT 8 exports cited finding/invocation, ordered witness-source and member-fact rows with row/byte caps ([ADR-0049](../adr/0049-served-support-closure.md)); structured and Markdown use one hydrated closure. A member without a generic source span says `fact_only`/`unavailable` | **Focused implementation passed** (`a3d7f4d`, `d9d8a40`): finding-only coordinates claim reaches model and source span; missing closure refused; deterministic rebuild and targeted serving tests passed. The checkpoint gate includes these cases and the ordinary pilot passes (§1.1); assembled Stage 3 acceptance remains open. Fact-only expansion waits for a source-span consumer | A finding-only coordinates claim resolves to its model and source span; structured and resource forms agree; missing support is rejected or unknown |
| **W4 · [RFU/F05](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F05)**, [RF/F09](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F09) | `cpg-flow` / `cpg-core::flow_model` access boundary | Core flow resolves bare, qualified and aliased builtin access; computed names become dynamic and shadowing is respected. The corrected no-read premise survives publication and `get_operation` | **Focused implementation passed** (`5044fee`, `d4a84ad`): real-provider controls and a published/served premise trace distinguish literal reads from a genuinely unread field; integrated acceptance pending | Real-provider controls for all three spellings, computed names and shadowed builtins; a genuinely unread field stays distinguishable; one premise traced through publication and serving |
| <a id="ARC-02"></a>**W5 · ARC-02** | `lctx-analytics::summaries`, `cpg-core` | `cpg-core` acquires typed relations and the finite analytics producer composes them into flows, steps, refusals and boundaries without a session ([ADR-0050](../adr/0050-finite-summary-outcomes.md)); no new crate or provider framework | **Focused implementation passed** (`67ba159` plus ADR-0050 slice): direct identity and preceding-call withholding use pure production inputs; real compile parity passed. A narrow normal-completion witness is now tested in pure production inputs and a real Delta/native generation (order 1 checkpoint) | An admitted case and an independent withholding control run the production transformation with no extraction, embedding, Delta or server |
| <a id="ARC-03"></a>**W5 · ARC-03** | Summary producer, boundary publication | The producer owns typed admitted/refused outcomes; the same outcome publishes and validates boundaries. Append-only reasons distinguish depth, BDD work/node/atom limits; generic missing evidence does not erase `call_transfer`. [ADR-0054](../adr/0054-origin-specific-summary-identity.md) hashes each unaggregated contribution and carries it through modeled/direct/assignment/local seeds, summary proof identity, boundary key and native positive/open responses; the obsolete SQL-only complement is removed | **Partial, focused tests passed (2026-09-26):** actual nine-call depth and unsupported predecessor refusals reach native. A real 129-atom return condition yields `condition_atom_limit` through Delta, FORMAT 8 and native response, with no positive summary. Actual predecessor conjunctions retain specific work/node-limit causes in the pure producer. A real `mixed_origin` source now has two distinct origins on one raw return fact: one published summary and one `call_transfer` boundary, both traced through Delta and native in one generation. Both positive and open native paths expose the corresponding source IDs. Work/node-cap Delta/native publication remains open; no negative was emitted | An actual depth/BDD cap and a distinct unsupported case keep specific causes through producer, snapshot and native response; a two-origin real-source trace preserves both causes; neither becomes a negative; any codebook change is append-only |
| **W6 · [RF/F01](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F01), [RF/F02](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F02), [RFU/F01](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F01)** | `cpg-schema::condition_kernel`, native generation preparation | F01 non-allocating `check_binary_op` decisions and F02 effective-support normalization are implemented; the native catalog counts per-root retained closure as well as stored nodes (RFU/F01) | **Focused controls passed (2026-09-26):** a pair with >50k materialized result nodes decides compatibility; an over-preflight contradiction decides; an admitted pair reaches the production task cap and stays unknown. A valid 84,000-root shared-tail catalog crosses the production retained-node limit before hydration while under the root and stored-node caps; native load reports the same limit on a repeated-root fixture. A tiny native shared-tail catalog distinguishes 3 stored from 4 retained nodes. Integrated qualification and real-pilot cost remain open | F01: an admitted pair over 50k result nodes decides; an over-preflight contradiction decides; a task-limit control stays unknown. F02: redundant composition stays under the limit and live support equals hydrated support. RFU/F01: shared-tail and disjoint controls refuse before large allocation; native diagnostics distinguish stored from retained counts |
| **W7 · [RF/F05](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F05)** | `cpg-core::flow_model` | A sorted reverse-dependency worklist now computes the least source fixed point per receiver mode, with a one-million-work cap; unfinished uses and dependent parents publish `flow_reach_boundaries = budget_reached`, known source conditions widen to unknown, and dependent negative premises are withheld ([ADR-0051](../adr/0051-value-reach-worklist.md)). This deliberately uses a whole-use worklist in place of the premised SCC-local schedule; pilot cost can trigger that optimization | **Partial, targeted tests passed:** A-first/B-first/reversed cyclic transfer and low-budget controls, shared schema/rule snapshots, real compile and bundle cases. Reversing real extracted flow rows before two analyzed compiles leaves canonical Arrow IPC bytes equal for published contributions, value flows and negative premises. A pure-model fixture crosses the production one-million-work cap and leaves cyclic dependents open with `budget_reached` (2026-09-26). The production-cap snapshot/native trace and fresh pilot cost remain open; no integrated qualification | A5's acceptance |
| **W8 · [RFU/F07](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F07)** | `cpg-extract` acquisition/context | The corpus identity includes content and membership of every analyzer-readable root, including unselected helpers; ordinary site-packages were already hashed | **Focused closure passed (2026-09-26):** content edit/addition changes the corpus identity and relocation preserves it. Warm extraction after editing an unselected imported helper or adding an unselected analyzer-readable helper produces exactly the same fact tables as a clean relocated extraction of each changed tree. Fresh pilot/integrated qualification remains open | Editing and adding an unselected helper changes identity or is refused before extraction; relocation invariance holds; facts equal a clean recomputation |
| **W9 · [RFU/F06](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F06)** | `cpg-core::embed` | Both cache-fill entry points now share token admission, batching, insert-only merge and versioned readback of the committed winner. The focused fake-embedder race and over-cap controls passed; live replay remains | **Focused cache semantics passed** (`6c92ddc`): both fill routes share token admission, batching, insert-only merge and committed readback; live embedding replay pending | Two attempts with distinct valid vectors return exactly the committed values; an over-cap embedder is rejected through both entry points before insertion. Fake-embedder doubles qualify cache semantics only |
| **W10 · [RFU/F04](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F04)** | Attribute schema, `lctx-analytics::concepts`, Stage F | Typed semantic attribute keys replace English identity/parsing. Separate object incidences preserve source facts, call modality/phase and both handoff endpoints; one renderer serves synthesis/facets. Publication and FORMAT 9 enforce cited supporter/pair closure | **Implemented; focused Tested (2026-09-27, compiler 88/template 20)**: source→Delta→serving, parallel/candidate/definite/relabeling controls and malformed support refusals passed. [Bounded review §19](../design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md#19-bounded-implementation-follow-up-typed-current-fcarca-and-handoff-support) F09/F10 corrected; integrated acceptance pending. New behavioral FCA and technique retention stay Stage 4.7 | Semantic IDs and finding membership survive display changes; implication supporters and retained handoff pairs have attributed evidence. Source validation checks the retained subset, not full lattice completeness |
| **W11 · [RF/F03](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F03), [RF/F04](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F04), [RF/F12](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F12)** | `condition_kernel`, `primitive_theory`, validation lane | F04 exact string membership and non-bool integer equality are implemented (`True in {1}` remains unknown). F03 cube factoring retains the equality proof; exact-input assessment uses bounded restriction and proof-link minimization. F12 has a ten-atom truth-table control and a recorded CPython 3.14.7 finite-literal lowering matrix. Synthetic and real-source Delta/native/MCP controls keep membership and equality refutations path-local; source-origin cases beyond the focused fixture and operation-wide Q09 remain | **Partial, targeted tests passed** (`96081b7`, `dd3bdcf`, `0db9eb9`, `7d02b42` plus 2026-09-26 CPython/native matrix): literal theory, cube/restriction, ten-atom truth table, independent finite-literal controls and native two-path refutations; a real source-origin Delta/MCP round trip passed on 2026-09-26; broader source lowering and operation-wide Q09 remain open | F04: a `transport.py`-shaped fixture with the bool/int control. F03: an over-budget DNF factor now factors with the equality gate; existing theory fixtures unchanged. F12: truth tables over ≤10 atoms in kernel tests; order 8 records disagreements as fixtures |
| **W12 · [RF/F06](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F06)** | `lctx-analytics::summaries` | ADR-0053's SCC value worklist retains origin-specific proofs, depth and pair-work refusals. ADR-0057 adds bounded simultaneous BDD substitution, normalized source-ordered arguments, required-formal checks and shared whole-control-group admission. Closed expressions supply exact Boolean values separately from completion; direct-formal forwarding still requires a caller-fixed linked guard | **Partial, targeted tests passed (2026-09-26):** finite/no-base SCC, origin/shuffle/low-cap controls; real three-argument and reversed-keyword value paths; opposing, missing-required and raising arguments stay open. The pure multi-control contract passes, but a real later callee predicate remains unknown until call-specific stability is established. New default expression-depth/work caps survive Delta/native; these do not close the original BDD/work/pair-cap obligations. [Foundation review](../design_review/reviews/design_review_stage3-foundations_2026-09-26.md). The origin-coverage review F01–F03 are corrected: call crossings and approximated summaries remain open, and witness omission is independent. The [semantic-frontier review](../design_review/reviews/design_review_stage3-semantic-frontier_2026-09-26.md) F01 is corrected by per-semantic-alternative refusal replacement. Compiler101 shares the queue/frontier/progress state, retains nondominated depth/expanded-proof-cost representatives and prevents a later limited witness from undoing the same alternative's success. Equal-cost witness alternatives do not drive recursion. Residual conjunction, other channels, engine comparison and pilot cost remain open | S4/S7: exact stable argument/predicate composition, per-origin default-cap traces, shuffled published bytes and measured pilot delta |
| **W13 · [RF/F07](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F07), [RF/F08](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F08)** | `cpg-schema` recursive CTEs; `lctx-analytics::summaries` SCC | F07: unknown ancestry uses `UNION` over term id; the type layer uses `UNION` over function/term id. Both are set membership, so recursive distinct removes repeat paths without losing cited provenance. F08: [ADR-0052](../adr/0052-iterative-scc-schedule.md) chooses pinned petgraph's iterative `kosaraju_scc`; analytics retains canonical condensation ordering | **Focused functional controls passed (2026-09-26):** both type-term closures terminate on cyclic rows with unchanged membership/attributes, and real analytics concept/type-layer variants pass; a 30,000-edge SCC chain, order-independence and real component-order compile pass. The all-techniques digest passes at the current checkpoint; pilot timing is recorded in §1.1. Controlled cost comparison and assembled Stage 3 acceptance remain open | Cyclic type-term fixture terminates with unchanged identity; existing SCC and component-order tests; a recorded stack-safety decision |
| **W14 · [RF/F10](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F10), [RF/F11](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F11), [RF/F14](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F14), [RF/F15](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F15)** | `cpg-extract`, `cpg-flow`, `lctx-analytics` Pass B, `cpg-core` derivation | F10 resolved Pyrefly function flags/body kind are persisted and drive negative premises (schema migration); F11 ty settings come from run context and a virtual release root, with ty's implicit package-submodule definitions separately coded; F14 Pass B visited state includes suppression. F15 now transports all twenty SQL-only behavior table derivations directly through declared Arrow batches, strict casts and canonical sorting; pure semantic consumers retain typed rows | **Focused implementation passed**: F10 (`4921a27`, schema migration), F11 (`5044fee`, `e67dc76`, append-only codebook migration) and F14 (`df84905`). F15 trigger executed in this consolidation: focused source/model/native publication and reconstruction passed (2026-09-26); integrated qualification pending | F10: aliased-decorator and Protocol fixtures. F11: a provider operation actually affected by settings (e.g. resolution). F14: a fixture where the clean path is found second. F15: identical snapshots; independent rederivations kept |
| **W15 · [RF/F16](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F16)** | DESIGN §6.3; `cpg-core::delta` | §6.3 previously claimed merge migrations no code performs; `verify` rejects drift. [ADR-0048](../adr/0048-schema-rebuild-policy.md) chooses a fresh rebuild of the current library from pinned inputs for schema changes. Historical snapshot reads and retaining old binaries are not required; the old store may be held temporarily for rollback. Reuse the global embedding cache only after independent contract verification | **Policy implemented and Tested (2026-09-27).** The compiler102 pilot rebuilt a new store from pinned inputs, including `function_implementations` and subsequent migrations, and published/served its current generation (§1.1). Strict drift rejection remains covered by the full gate; live embedding reuse and assembled Stage 3 acceptance remain separate. Revisit only for a real historical-read consumer or unavailable pinned input | A fresh compile produces a current snapshot and `embedding_specs` with the new schema; old analysis tables are not copied; `verify` still refuses drift |
| **W16 · [RFU/F08](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F08)** | Embedding spec, `just embed-serve`, Rust/Python clients | The controlled vLLM launch derives model, revision, tokenizer revision and dtype from the hashed spec. DESIGN §11.1 limits deployment-identity claims to that operator-controlled launch; an arbitrary endpoint's revision remains unverified even if model name, shape and spec hash match | **Focused correction passed** (`2d48196` and §11.1 contract, 2026-09-25); broader endpoint attestation is **Deferred** until an external-endpoint guarantee has a consumer | A revision change moves launch configuration and cache identity together under the controlled launch; arbitrary endpoint identity is explicitly unsupported |

Both reviews' tool-placement conclusions are in §5. W15's policy is decided by ADR-0048; the
fresh-store execution is recorded in §1.1. No other review finding is closed by documentation cleanup.

## 7. Deferred, each with a trigger

| Item | Trigger |
|---|---|
| Points-to sets, memory versions, strong and weak updates | A Stage 3 summary whose field effect needs aliasing |
| Per-claim assumption records | A served claim misread because of an assumption the `approximated` flag does not show |
| A per-iteration unrolled loop model | An evaluation item graded `partial` for a loop condition |
| Pyrefly-typed receivers for dynamic access; per-run largest-scope and residue reports | A refutation on a field read through a typed non-`self` receiver, or a report consumer |
| Transfer/decorator/lambda fidelity | Trigger met; correction scheduled in S5 above. General deferred execution remains Stage 5 |
| An unmapped argument at a multi-callee site; a lambda read's phase; inherited methods counted as field reads | A pilot or fixture row appears |
| ty type inference as a second type provider; ty patches; the CPython bytecode CFG (`bytecode` 0.19) | A question Pyrefly's types cannot answer; a measurably degraded scope; an effectful-frame case runtime inputs cannot exercise (order 2) |
| Analytics technique retention (ADR-0020's keep-rule outcome); deleting FCA, RCA, communities or kNN variants | Stage 4 gives them consumers; decide on that evidence |
| Stage F brief-builder restructuring | A rendering consumer needs it; briefs remain one rendering |
| Graph-FCA in the pipeline | An offline experiment yields templates the evaluation rates useful |
| On-demand RCA at serve time | Materialized membership proves too coarse |
| Lance / LanceDB | More than ~10⁵ vectors at 4,096-d, or filtered ANN with managed FTS |
| spaCy in compile | Regex directive tagging misses conditions the evaluation needs |
| A neural reranker | An ADR under §B10 after a measured need |
| Native-extension bodies | A pilot question needs one |
| Closed-hierarchy `self` dispatch (`override_dispatch`, 2,018 pilot rows) | A Stage 4/5 item graded partial solely because of dispatch, or an operator request |
| Cross-call exception composition (callee escape through caller handlers) | A sync evaluation item not blocked by dispatch or async needs it (Q01.e/f sit behind dispatch; Q05 is async) |
| `**kwargs` binding TypeErrors | A sync evaluation item (Q09.a/e/f are async) |
| New condition roots; discharge for argument/store (`derives`/`stores`) sinks | Pilot evidence after P7 that these sinks dominate a graded item |
| pydantic field storage, dataclass `__init__`, `inspect` predicates; rich, subprocess, `sys.exit` models | A sync consumer, or Stage 5 |
| Role channel producer | An authored role meaning with a consumer |
| F17 broader release/default/binding domains | A graded item or pilot bucket needs them |
| Reachability certificates for non-return sinks under ty's AMBIGUOUS regions (`try` bodies, `with` exits, loops over unknown iterables): 730 pilot `missing_evidence` rows, including Q03.d and Q05.b | A graded item whose rating depends on it, or a completion-kernel extension that already covers the shape |

## 8. Risks

| Risk | Mitigation |
|---|---|
| Per-site atoms lose simplification | Sound by construction; the typed theory restores sharing for proven-stable primitive places; rendered sizes measured on the pilot |
| BDD blow-up on unstructured conditions | Per-operation node and pair-work caps widening to `unknown`; W6's aggregate preparation budget; deterministic order |
| The validation lane executes code | Generated programs only, outside `fixtures/python/`; isolated worker, no network, timeouts; the analyzed library is never executed |
| Summaries lose precision or fail to terminate | Finite domains with explicit caps and typed refusals (W5); `unknown` rates reported |
| ty churn and salsa skew | Exact pins; parity tests on upgrade |
| Scope creep | Stage exit rules; §7's triggers; nothing starts before its stage |
| Disk and GPU | One pilot store; live GPU legs only with ≥30 GB free and vLLM stopped afterwards |

## 9. Standing conventions

- Small commits to `main`, each naming its stage/item and any ADR and stating the observed check
  outcome; a preliminary gate cannot certify the Stage 3 end. Never push unasked, force-push or
  `reset --hard`.
- Judgment calls go in the commit message; a scope change updates this plan; an architectural
  change gets an ADR.
- Snapshot changes: read the `.snap.new`, then `cargo insta accept --workspace`; a schema snapshot
  change is a declared migration. Never `cargo insta review`. A schema migration needs a fresh
  store rebuilt from pinned inputs (ADR-0048). Moving `build/store` aside is a temporary rollback
  measure, not a historical-read promise. The new compile produces `embedding_specs`; reuse the
  global `embedding_cache` only if its separate contract verifies. Do not copy old analysis tables.
- Codebooks are append-only (`verdict`, `boundary_reason`, `condition_atom`, effect kinds, concept
  ids, model ids). `libraries/fastmcp/analytics.toml` is frozen; an edit needs an ADR.
- `fixtures/python/` is never executed; `eval/heldout/` stays sealed until increment 5's end.
- Pitfalls already hit: `just test-all` stops at its first failure, so rerun before reporting;
  edits made through Python or shell bypass the format hook, so run `just fmt` first; the Python
  suite runs against the `analysis_shapes` fixture generation, and the analysis diff snapshot
  counts its behaviors, so attribution changes move both; `pkill -f` can match its own command
  line (use `[v]llm`).
- Operator housekeeping: the moved-aside stores `build/store-pre-*` and `build/store-stage3-*` are
  kept until the operator deletes them.
