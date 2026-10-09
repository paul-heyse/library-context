# Agent effectiveness plan: independent design and target review

**Date:** 2026-10-09, America/New_York. **Tier / purpose:** design / target.  
**Principal reviewer:** independent delegated design reviewer; the coordinator publishes this assessment.  
**Decision: Accept scoped, at Proposed target strength.**

The plan provides a credible route from the source review's defects to a corrected agent workspace. It keeps the useful existing owners, makes cleanup and execution observations explicit, removes terminal readers from supervision, and binds readiness to the environment interval it justifies. Its additional diagnostics reuse native capabilities and identified fixture content instead of introducing another datastore, scheduler or permanent connection.

The full selected target is covered, including downstream profiling/worktree consumers, both halves of AE-24, and the command retrospective. Two material authoring defects identified during this review are corrected in the final reviewed draft: immutable historical records now have an explicit durable recovery transition, and native MCP no longer promises sanitization that its implementation does not supply. Publication receipts also now bind to captured rendering inputs and reject concurrent input changes before replacement. These are corrections to the **Proposed target**, not closure of implementation findings.

Acceptance excludes sensitive or uncertain content through native MCP, product semantic acceptance, real-library qualification, operator activation and measured effectiveness. Native MCP's support limit is consequential: the direct native connection returns raw server errors and has fixture-admin authority; explicit per-call scope is not an authorization boundary. Expanding that scope requires the plan's stated protection/design decision. The existing implemented workspace retains the source review's **Revise** judgment until its scheduled corrections and acceptance evidence exist.

## 1. Review contract and evidence boundary

| Field | Scope |
|---|---|
| Subject | [Agent effectiveness follow-up plan](../../plans/agent-effectiveness-followup-plan_2026-10-09.md), final reviewed SHA-256 `2f065bd2da4178efcfd7805267901085cf4a8e735476f50a5fad5b7c376fd0ff` |
| Baseline | HEAD `2a9a3771ac7a7041e0e0c42d4010ade24ddee426`; initially clean except the untracked plan. The coordinator subsequently changed the plan and its source/disposition navigation while this review ran. The final target and those relevant routes were reread. No product implementation changed in the reviewed scope. Publication may subsequently add review/disposition links, the unchanged deferred loaded-extension identity route and actual documentation receipts; those metadata updates do not alter this target judgment. |
| Standard | Core/template 3.3, efficient-architecture heuristics 1.0, code-intelligence profile/review additions 1.5 and library-context binding, loaded through [standard.toml](../design_principles/standard.toml) |
| Functional intent | One operator, Codex primary with occasional Claude; short queries, concurrent tasks, long builds/controls, a mutable native extension, large logs and terminal observers that can disappear or stall |
| Method | Read-only source, owner, plan, retrospective and versioned interface inspection; no probes |
| Exclusions | Builds, tests, synchronization, services, installations, operator databases/registrations, real-library work, product diagnosis and quantitative productivity/performance claims |

The review applied `design-review`, its declared code-intelligence companion and the pinned `neo4j-surrealdb` source index. Repository evidence, rather than another project's receipts, supports the current judgments. The source review and retrospective were read as dated evidence; their historical commands were not rerun or reclassified as current acceptance.

**Known standards conflict:** DP-20 contains resource-budget requirements, while the operator explicitly excludes new harness workload/thread/concurrency/duration caps. The operator's instruction governs the review's permitted recommendation scope. Coordination, cancellation/drain and truthful outcomes remain assessed; this review does not certify a general resource-budget policy or treat unrestricted permitted resource use as a defect. The bounded observer buffer is an observation implementation property, not a new workload restriction.

## 2. Responsibilities, domain meaning and adjacent consumers

The important phenomena are command launch, native child exit, wrapper failure, cleanup certainty, live ownership, captured output, publication inputs, fixture attachment and diagnostic content scope. The final target gives each an identifiable operation and preserves their independent meanings.

| Owner | Target responsibility and consumer boundary |
|---|---|
| `harness.py` / `runs.py` | Process primitives and one durable attempt owner; launch/exit/supervisor/cleanup observations, recovery, presentation attachment and retention |
| `verify.py` | Boundary/step outcome, selected native commands, summary and rerun; consumes owned launch/environment/fixture observations rather than reconstructing them |
| `workspace_env.py` | Effective resource identity, shared/exclusive ownership, readiness observation and explicit preparation |
| `surrealdb_fixture.py` | Native server and attachment/content lifetime; shared server-end policy, checked diagnostic scope and sanitized harness SQL causes |
| `compile_profile.py` / `worktree.py` | Consume the run owner's liveness and cleanup semantics; profiling completeness and checkout-removal eligibility remain distinct decisions |
| `docs.py` / `freshness.py` | Publisher owns captured publication dependencies and receipt; freshness observes those dependencies without publishing or modifying them |
| Native SurrealDB / Codex / Cargo / nextest / Rust adapter | Own their protocols and argument grammar; repository guidance qualifies their effects and support limits |

This is an adequate small domain model for the target. It does not merely add output fields: §§3.1–3.5 specify when operations acquire responsibility, which observations permit decisions, and what their consumers may infer. AF1–AF4 move the actual deciding consumers together.

The adjacent-consumer investigation matters. Current `runs.py:112–118,674–719,744–759` derives terminal recovery/retention from `termination`; `worktree.py:408–412` blocks removal only for `state == running`; `compile_profile.py:1116–1119` derives partialness from run state and product fields, and `:1129–1143` checks attach eligibility. The plan explicitly names those consumers and prohibits process exit from certifying cleanup or capture completeness. Its single current reader is justified by retained profiling/capture consumers, with no bulk compatibility service or historical data deletion.

## 3. Contracts, composition and execution fit

**Run recovery.** §3.1 places responsibility immediately after successful spawn, before identity capture or receipt publication can fail. It preserves the child's actual exit while independently reporting supervisor failure and cleanup confirmed/failed/unknown. Ordinary legacy observation is read-only. Explicit recovery can atomically upgrade one selected record under owner exclusion, retain original/unknown fields and preserve captures. A live owner is contacted through its request protocol; unresolved/foreign identity never authorizes signalling a reused PID. This closes the plan-level preservation contradiction without inventing another receipt store.

**Output.** §3.2 retains direct file capture and moves terminal presentation to a detachable observer outside critical supervision, including startup/progress feedback. `verify --live` receives the same correction; nested run/verify composition retains one child owner. Observer identity is accounted for separately from product descendants. This addresses current `runs.py:364–399` and `verify.py:1157–1173`, where terminal writes can block lifecycle work or child drainage. Suffix seeking and offset following remove repeated full-log hydration for small observations. Large individual lines remain legitimately large inputs; the target makes no constant-memory claim for arbitrary requested lines.

**Environment.** §§3.3 and AF3 put readiness and discovery inside the effective shared-ownership interval, and writer recheck/publication validation inside exclusivity. Current `verify.py:1115–1125,1297–1335,1636–1640` and `workspace_env.py:466–484` expose the reported gaps. Pure Rust remains independent, readers overlap, nested holders reuse valid ownership, and preparation remains explicit. No global command barrier is needed.

**Freshness.** AF5 isolates Cargo/Hakari's mutable metadata operation in an owned disposable capture, rather than relying on `--diff`/`--dry-run` or rolling back the live lockfile. It keeps the existing resolver and feature-union implementation. Missing offline inputs produce a non-result. This is a credible tradeoff for a workspace metadata check: copy necessary manifests/layout/target-discovery inputs, not build caches, environments or captures.

The documentation receipt now describes the bytes/context actually consumed by staging, includes discovered membership, configuration/theme/publisher, actual tool identity and output-affecting Git annotations, and accompanies the same candidate publication. Concurrent change is checked before replacement; the prior site survives refusal. This addresses the specific existing route in `docs.py:415–442`, which captures pages before rendering and later observes other live inputs. A receipt for newer source beside older rendered content is explicitly prohibited. Freshness is only input agreement, not link or architectural acceptance.

These choices fit the supported workload qualitatively: large output is captured once, small observations read useful suffixes, stalled readers cannot hold the lifecycle owner, and coordination follows mutable resource lifetimes. The extra observer and metadata capture have concrete failure/isolation purposes. No latency, throughput, capacity or productivity improvement is established.

## 4. Revealing change and failure scenarios

| Scenario / kind | Expected change and affected consumers | Assessment |
|---|---|---|
| Terminal child exit with descendants or failed recording; lifecycle/domain distinction | AF1 run/fixture guard and run consumers; child exit preserved, cleanup retriable, pruning/removal protected | Credible Proposed route, including real disposable process/group cases |
| Recover a retained schema-1 profiling run; contract evolution | One selected owner-controlled upgrade, then current reader/profile/worktree consumers | Durable resolution now specified; no bulk migration or capture mutation |
| Replace live presentation or stop reading its sink; mechanism substitution | AF2 observer/file helper changes; lifecycle and full logs stay authoritative | Composition and execution fit hold at target level |
| Add a Python-collecting boundary; extension/binding | Requirement declaration feeds owned discovery/execution; no second readiness classifier | Local route, with controlled reader/writer interleavings |
| A child actually exits 127 or fixture ends without known cause; outcome policy | AF4 launch result and fixture policy feed both direct and family consumers | No sentinel reconstruction or invented product causality |
| Inspect known fixture schema/index/query plan; new diagnostic operation | AF6/AF7 checked content/lifetime, native envelopes, sanitized harness cause or qualified raw MCP output | Native MCP limited to synthetic/non-sensitive disposable content; scoped acceptance depends on this exclusion |
| Change a doc/theme/tool while rendering; publication failure | AF5 capture/recheck refuses candidate, preserves prior site, later observation reports drift | Coherent receipt/publication route and revealing acceptance case |

The synthetic-fixture limit is not a claim that arbitrary SQL is read-only, native MCP sanitizes errors, or fixture-admin credentials enforce namespace confinement. The plan explicitly retains those distinctions. Sensitive/uncertain retained content follows the sanitized harness route; diagnostic result handling still needs to honor its own requested-data contract.

## 5. Source traceability and execution/retirement

| Source obligation | Complete target route |
|---|---|
| Source F01 / RC03 | §§2, 3.1; AF1; run/fixture post-spawn exceptions, observed cleanup, legacy recovery, downstream consumers and retention |
| Source F02 / RC04 | §§2, 3.2; AF2; detachable live presentation for run and verification, full direct logs and observer cleanup |
| Source F03 / RC02 | §§2, 3.3; AF3; owned discovery/readiness and queued writer recheck/postcondition |
| Source F04 | §3.2; AF2; shared suffix/follow access for run logs and verification excerpts |
| Source F05 | §3.4; AF4; real launch versus child exit, one fixture-end classification, summaries/rerun and unknown causes |
| Earlier AE-24 | §3.5; AF5; isolated Hakari computation and publisher-owned input receipt |
| Source §8 capabilities | §4; every proposal chosen: AF6 syntax/SQL diagnostics, AF7 scoped native MCP, AF8 qualified native guidance; adapter patch and selector CLI explicitly deferred with triggers |
| Retrospective U1/U2/U3/E1/T1/T2/T3 | §5; AF8/AF6 examples retain the historical grammar, effect, channel and causal limits |
| RC01 | §§1–2, AF0/AF8; Codex-primary eligible host/user customization without gratuitous Claude parity |

The capability table also preserves the product gRPC stream-terminal boundary and the separate coverage owners AE-12/AE-25. No source recommendation disappears into a generic “tooling” task.

AF0 records decisions through existing ADR/owner routes; AF1–AF8 identify consumers and replaced paths; AF9 integrates and reviews the assembled result. Shared-file ownership and prerequisite contracts are explicit. Independent AF5/AF8 work can proceed without a global barrier, while MCP waits for reliable run/fixture lifetime and diagnostic scope. Retirement is concrete: terminal-implies-cleanup shortcuts, synchronous mirroring, whole-file tails, pre-lock admission/unowned discovery, integer-127 inference, duplicate fixture policy, live Hakari lock mutation and publication without a matching receipt. Native tools and retained captures remain because they have current consumers.

Plan §8 is the sole scheduled disposition owner for source F01–F05, selected recommendations and AE-24. Source navigation now links there, and the earlier workspace plan retains its historical receipts and other obligations. This preserves dated evidence without creating a competing active ledger.

## 6. Independent gates and architectural judgments

Gate verdicts concern the **specified target**, not implementation acceptance.

| Gate | Verdict | Basis |
|---|---|---|
| G1 Authority | pass, scoped | Existing owners retain run, boundary, environment, fixture and publication meaning; downstream consumers migrate together |
| G2 Semantic fidelity | pass, scoped | Launch/exit/supervisor/cleanup and historical/current recovery remain distinguishable; explicit selected upgrade preserves original child outcome |
| G3 Validity | pass, target | Owned readiness/use, checked fixture/content scope and negative/interleaving controls are specified; no live validation claimed |
| G4 Hidden behavior | pass, target | Hakari effects are isolated; freshness never publishes; compiling discovery, arbitrary SQL and native diagnostic effects are disclosed |
| G5 Consistency/recovery | pass, scoped target | One-writer recovery, unresolved retention, independent observer lifetime and captured receipt/candidate publication are specified |
| G6 Transformation/reuse | pass, scoped target | Legacy interpretation does not infer cleanup; publication agreement covers its dependencies; selected rerun and profiling semantics remain owned |
| G7 Truthful capability claims | pass, scoped | Proposed versus Interface-checked versus historical receipts are explicit; native MCP output and installed/configured/exercised limits remain visible |
| G8 Library leverage | pass | Native subprocess/file/flock/Cargo/Hakari/SurrealDB/Codex routes are retained; small remaining helpers serve actual gaps |
| CI-G1 Fidelity | pass, scoped | Heuristic retrospective classifications do not become adjudicated rates; cold semantic emptiness, wrapper success and mixed MCP success retain narrower meanings |
| CI-G2 Evidence closure | pass, scoped to plan evidence | Plan recommendations link dated source evidence and named current owners; product snapshot/served-claim closure is excluded |
| CI-G3 Evaluation integrity | n.a. | No protected evaluation or gold inputs change |

| Architectural judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | satisfied, Proposed | New boundary/content bindings and observer changes stay with their owners; shared policy migrations identify actual deciding consumers |
| A2 Encode domain meaning explicitly | satisfied, Proposed | Adequate distinctions govern target operations, not only output schemas; resolved F01/F02 below remove two consequential ambiguities |
| A3 Extend through composition | satisfied, Proposed | Native tools, owned contexts and independent observations compose without another lifecycle/command framework |
| A4 Fit execution to supported workload | satisfied, qualitative Proposed | Direct capture, suffix/offset access, narrow ownership and reused native execution remove the named amplification/backpressure paths; extra machinery has concrete isolation/lifetime purposes |

FP-01–FP-07 are satisfied for these target scenarios at Proposed strength. Relevant DP-01/02/03/05/08/09/15/18/19/21/22/23/24 and the scoped CI-01/04/06/11 properties have specified owners and acceptance routes. DP-10/13/14/16/17 support the selective/native realization. DP-20's coordination portion is assessed as above; its excluded resource-budget policy is not certified. Product graph/analysis-specific CI obligations are outside this harness target.

## 7. Stable authoring findings and their resolution

These IDs belong to this review and are distinct from the source review's F01–F05. They record corrections verified in the final target, not newly scheduled implementation closure.

<a id="F01"></a>

### F01 — Historical byte preservation originally prevented durable recovery

**FP-02/04/05; DP-19/24; A2; G2/G5.** The first reviewed §3.1 required schema-1 records to remain byte-for-byte unchanged while also making absent cleanup unknown, unresolved cleanup non-prunable and explicit recovery available. Existing `runs.py:695–708` persists recovery into `record.json`; the initial target named no permissible durable transition. A valid old run could therefore remain cleanup-unknown forever, or recovery could silently violate preservation.

**Resolved in the reviewed Proposed target:** §3.1 now distinguishes read-only observation from explicit selected recovery. The latter may atomically upgrade one record under owner exclusion, preserving original/unknown fields and child exit; AF1 tests the durable upgrade and unchanged captures. Static rereading settles this authoring defect. Implementation and runtime closure remain AF1/AF9 obligations.

<a id="F02"></a>

### F02 — Native MCP originally inherited a sanitization promise without a redaction boundary

**FP-02/04/06; DP-15/18/22; A2/A3; G4/G7.** The initial §4 per-statement route promised SQL/bind/credential-safe diagnostics across AF6/AF7 while selecting direct native HTTP MCP with no intermediary. Pinned 3.3 `surrealdb-mcp/src/tools/output.rs:77–85,118–137,192–210` returns native `err.message()` in text and structured content; `surrealdb-types/src/error.rs:398–400` returns the stored message. Unlike `surrealdb_fixture.py:815–820`, that boundary does not suppress native error bodies. A launcher-only change cannot guarantee response sanitization.

**Resolved in the reviewed Proposed target:** §4 separates sanitized harness `/sql` causes from native MCP output, confines native MCP to owned synthetic/non-sensitive disposable content, keeps transport-auth secrecy separate and routes sensitive/uncertain retained content through the harness diagnostic. AF7 exercises the support limit and makes no native-message redaction claim. Expanding that scope requires separately justified protection or a changed design. Implementation and live qualification remain not_run.

No material unresolved authoring finding remains in the final identified target. The publication capture/recheck clarification and actual installed-tool identity are additional preservation requirements now present in §3.5/AF5, not evidence that the publisher is already corrected.

## 8. Library fit, alternatives and total complexity

The plan chooses existing native capability where semantics fit: SurrealDB syntax validation and existing-server HTTP MCP, native Cargo/nextest structured evidence, Codex invocation configuration, the current Rust semantic adapter and existing profiling readers. Independent source inspection confirmed that the captured 3.3 HTTP handler uses `state.datastore`, `ToolScope` allows per-call namespace/database, and MCP output preserves per-statement distinctions and raw native error messages. Adapter 0.4.0 `server.rs:50` lazily starts/restarts its client, while `handlers.rs:115–132,221–252` distinguishes immediate references from loading-qualified rename. These are source/interface observations, not proof of installed live behavior.

Instructions alone cannot correct lifecycle/readiness/output defects. Bare tools and native continuation remain the simplest ephemeral route, but do not supply durable cross-session recovery. An extra scheduler, permanent MCP bridge, custom Cargo parser, universal receipt selector or mandatory semantic index would add consumers/state without a demonstrated gap. The chosen file helper and metadata capture have narrower purposes and existing owners; they do not recreate a query engine or resolver.

Native MCP adds client/session qualification burden and a real content support limit. Its benefit is interactive fixture inspection, not stronger production stream assurance. The sanitized HTTP route remains independently useful if MCP is blocked. That fallback does not complete AF7, as the plan states.

## 9. Verification, rule impacts and decision

**passed — static review, 2026-10-09:** read-only `cat`, `sed`, `rg`, `nl`, `git status --short`, `git rev-parse HEAD`, `git diff --stat` and `sha256sum` against the named plan, standards, owners and pinned sources. A guessed error-source directory was absent; the indexed `error.rs` file was located and read. That lookup miss supplies no absence claim.

**not_run:** probes, tests, builds, services, synchronization, installation, fresh Codex connection/semantic requests, documentation publication and turn-end. Only the assigned scratch review was written. Static evidence suffices for this target judgment; implementation acceptance cannot be supplied by this review.

AF1–AF8 specify meaningful fault/interleaving/negative cases rather than classification-only mocks. AF9 explicitly schedules affected controls/leaves and assembled qualification for the shared receipt contract, preserves unrelated failed/blocked product boundaries and leaves whole-plan acceptance open when those remain. Focused correctness, assembled acceptance, real-library qualification and measured benefit stay separate.

**Additional rule impacts: none.** Source RC01–RC04 are operator-confirmed choices recorded in plan §2 and routed through AF0; this review neither reconfirms nor applies them. The selected legacy-record transition is part of RC03's explicit contract evolution. Native MCP's narrowed content support follows the existing privacy/native-boundary requirement and does not authorize a new middleware/configuration regime. No newly discovered rule change is needed for the recommendations accepted here.

**Bounded decision: Accept scoped, Proposed target only.** The plan captures all selected source findings/recommendations and gives credible owners, transition/deletion obligations, dependencies and revealing acceptance. Native MCP sensitive/uncertain-content support and all implementation/product/release/effectiveness claims remain excluded with explicit revisit routes. Source F01–F05 and AE-24 remain open at [plan §8](../../plans/agent-effectiveness-followup-plan_2026-10-09.md#8-finding-and-recommendation-disposition-owner); the existing workspace is not accepted as corrected. The next authorized action is publication of this review and plan; implementation still requires the separately stated execution authorization.

**Intended publication:** `docs/design_review/reviews/design_review_agent-effectiveness-plan_2026-10-09.md`.
