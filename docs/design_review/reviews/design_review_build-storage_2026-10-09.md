# Build storage and workspace lifecycle

**Principal design/target review · 2026-10-09**

**Decision: Revise.** The shared Cargo build directory is a sound default for this workspace. Checkout-local final artifacts, Cargo’s own scheduling and fingerprinting, explicit worktree isolation, and attempt-owned profiling directories support useful reuse without requiring a separate build service.

The storage lifecycle does not yet support the complete target: normal builds, tests and native synchronization; justified isolation; debugging; and replayable evidence without indefinite growth exhausting their shared filesystem. Three causes need correction:

1. Profiling automatically protects an entire run indefinitely, coupling compact receipts to large raw captures.
2. Ad hoc isolated Cargo roots can outlive their worktrees and current consumers without an explicit retirement boundary.
3. Profiling’s free-space safeguard stops CPU sampling but leaves the larger rustc self-profile writer running.

These are lifecycle and execution-fit findings, not evidence that incremental compilation or shared caches should be disabled. The strongest immediate removal candidates are the eight old isolated build roots. The five currently cited large captures require continued protection until their consumers are retired or a portable archive and restore route is validated.

All policy directions below are **Proposed**. This review authorizes no deletion, configuration change, archival transfer or process interruption.

## 1. Scope, baseline and evidence

| Field | Value |
|---|---|
| Functional target | Support ordinary build/test/native-sync work, justified isolated revisions and benchmarks, debugging and evidence replay without unbounded retained growth exhausting shared disk |
| Tier · purpose | Design · target |
| Reviewer · date | Independent delegated design reviewer · 2026-10-09 |
| Standard | Core/template 3.3; efficient-architecture heuristics 1.0; code-intelligence profile 1.5; library-context binding |
| Baseline | `ebf1328ee0355b995ea3dc1af44df3e85cd6af8b`, plus the STATUS-listed uncommitted native-execution changes and the coordinator’s read-only audit script |
| Relevant owners | `.cargo/config.toml`, root `Cargo.toml`, `scripts/build_environment.py`, `scripts/worktree.py`, `scripts/runs.py`, `scripts/compile_profile.py`, `scripts/compile_profile_capture.py`, DESIGN §1.2, compilation-cost plan and agent-effectiveness follow-up plan |
| Method | Independent static inspection of decisive repository sources, the original inode census and live-reference survey; coordinator-supplied historical command attribution |
| Exclusions | Product behavior review, real-library qualification, operator databases, implementation, cleanup, configuration changes and interruption |
| Verification boundary | Product/remediation controls **not_run**. Release remains the ordinary verifier until BC3 qualifies its replacement |

The original census covered the three named roots from **21:36:03 to 21:36:42 UTC on 2026-10-09**, examining 254,723 unique regular-file inodes without recorded scan errors. Its regular-file figures exclude directory allocation, allocated symlink blocks and filesystem metadata. The coordinator is preserving that original snapshot alongside subsequent accounting refinements.

Evidence: [method, attribution and exact commands](../evidence/2026-10-09_build-storage/README.md), [audit implementation](../evidence/2026-10-09_build-storage/audit.py), [original inode census](../evidence/2026-10-09_build-storage/raw/inventory.json) and [original live-reference survey](../evidence/2026-10-09_build-storage/raw/live-references.json). The [follow-up census](../evidence/2026-10-09_build-storage/raw/inventory-followup.json), taken at **21:41:47–21:42:01 UTC**, is a separate live observation; the inventory figures in this review remain those of the original **21:36 UTC** census.

The census is a live observation, not an atomic snapshot. Modification time establishes neither creation date nor last use. The process survey shows observed references, not comprehensive permission to delete anything. Historical commands establish original use without proving that no later consumer exists.

## 2. Inventory and relevance

Figures below are **Measured inventory observations, 2026-10-09**, expressed in GiB and rounded. They do not measure compilation speed or predict reclamation after a future operation.

### Independent accounting boundaries

| Exact root | Unique regular-file allocation | Files whose observed names cover all hardlinks | Directory allocation | Disposition |
|---|---:|---:|---:|---|
| `/home/paul/.cargo/build/library-context` | 170.293 | 169.549 | 0.083 | **Retain** active shared cache; introduce reviewed maintenance |
| `/home/paul/.cache/library-context-build` | 130.504 | 130.504 | 0.142 | **Remove candidates**, subject to fresh ownership and activity checks |
| `/home/paul/library-context/build` | 108.586 | 108.410 | 0.015 | **Retain / archive / unresolved**, according to consumer |
| **Total** | **409.383** | **408.463** | **0.240** | Not a reclaimable-total claim |

There were no shared regular-file inodes across census buckets. Approximately **0.920 GiB** had hardlink names outside the observed buckets, primarily final outputs. Removing only the inventoried names would not free those inodes.

“Files whose observed names cover all hardlinks” is an unlink-accounting upper bound. It establishes neither deletion safety nor exact filesystem free-space improvement. Open deleted files, concurrent writes and filesystem allocation behavior remain relevant.

The separately observed shared sccache footprint was approximately **36–37 GiB**, with a configured maximum of **100 GiB**. It is outside the three-root total and has other potential host consumers. It is not the primary unbounded class identified here.

### Shared Cargo intermediates

| Exact path | Regular-file GiB | Incremental GiB | Relevance and disposition |
|---|---:|---:|---|
| `/home/paul/.cargo/build/library-context/release` | 149.280 | 101.441 | **Retain.** Active verification and current native work use release |
| `/home/paul/.cargo/build/library-context/local-test-candidate` | 18.291 | 12.044 | **Retain pending BC3 decision.** Qualification has not completed |
| `/home/paul/.cargo/build/library-context/debug` | 2.722 | 1.823 | **Retain with expiry review.** Check/dev work can legitimately reuse it |

The live survey observed Cargo and rustc references into the shared release root. Repeated crate variants and many incremental unit directories explain physical accumulation, but do not establish redundant bytes. Profile, target, feature, flags, test-harness and source differences can legitimately require different units.

Do not consolidate these by copying Cargo internals into one another, deleting hash variants based on names, or disabling incremental compilation merely because it occupies substantial space.

### Old isolated Cargo roots

All paths below are under `/home/paul/.cache/library-context-build/`.

| Exact child | Regular-file GiB | Original use recovered by coordinator | Disposition |
|---|---:|---|---|
| `graph-native-native-tests` | 87.860 | Explicit Cargo build-directory override for native tests | **Remove candidate**, highest space consequence |
| `graph-native-root` | 14.664 | Explicit root/package check override | **Remove candidate** |
| `graph-native-model` | 11.527 | Model build/test work | **Remove candidate** |
| `graph-native-compiler-build` | 7.999 | Compiler build work | **Remove candidate** |
| `graph-native-effects` | 3.455 | Effects-related check work | **Remove candidate** |
| `graph-native-manifest` | 2.055 | Manifest/package check work | **Remove candidate** |
| `graph-native-compiler-check` | 1.778 | Compiler check work | **Remove candidate** |
| `graph-native-model-check` | 1.164 | Model check work | **Remove candidate** |
| **Total** | **130.504** | Oct 5 ad hoc isolated compilation | Conditional upper bound only |

Recovered original commands show explicit build-directory overrides, sometimes accompanied by target-directory overrides. Cached dep-info retains paths to former graph-native worktrees. Those source worktrees no longer exist, and the scoped current selector search found no tracked selector for these roots. Their top-level contents are Cargo cache tags and profile outputs.

Together, these observations make them strong removal candidates. They do not establish that the roots are unused now. A future cleanup must refresh process references, environment/configuration selectors, ownership and current diagnostic consumers.

An independent `du -B1 -sx` observation for this parent matched the original census’s files-plus-directories total: **130.645 GiB**. No outside hardlinks were reported for these eight roots.

**Consolidation means directing future ordinary work to the established shared root and retiring unused isolated roots. It does not mean merging their internal artifact directories.**

### Managed runs and profiling evidence

`/home/paul/library-context/build/runs` contained **105.572 GiB** of regular files across 131 run directories. Ten retained runs accounted for **105.134 GiB**. Only approximately **0.438 GiB** remained outside retained runs.

Consequently, ordinary pruning of unretained runs cannot address the main footprint.

Five currently cited captures account for approximately **101.787 GiB**:

| Exact run path under `/home/paul/library-context/build/runs/` | GiB | Recorded process outcome | Disposition |
|---|---:|---|---|
| `20261008T181813.329Z-ddd158` | 30.157 | Cancelled | **Retain; archive candidate after validated replay/restore design** |
| `20261008T200802.033Z-6761de` | 21.743 | Cancelled | **Retain; archive candidate after validated replay/restore design** |
| `20261008T224204.565Z-5a9f8e` | 19.195 | Interrupted | **Retain; archive candidate after validated replay/restore design** |
| `20261008T225117.998Z-e84d9b` | 15.361 | Completed | **Retain; archive candidate after validated replay/restore design** |
| `20261009T003236.093Z-dfb25d` | 15.332 | Completed | **Retain; archive candidate after validated replay/restore design** |

The [compilation-cost plan](../../plans/rust-compilation-costs-plan_2026-10-08.md) and its current coroutine work cite these runs. Cancellation or interruption does not make their diagnostic evidence obsolete.

Across the inventoried runs, raw rustc self-profile files accounted for **82.821 GiB** and perf data for **15.742 GiB**. Summaries are not equivalent substitutes for raw evidence when current consumers require re-reporting, different timeline thresholds or source attribution.

No archive destination has been qualified. Moving files elsewhere on the same filesystem does not reclaim capacity. Compression savings must be measured. Current raw absolute-path consumers require a deliberate restore or portable-reference design before removing originals.

### Smaller and unresolved material

| Path or class | Observation | Disposition |
|---|---|---|
| `/home/paul/library-context/build/review-probes` | 2.051 GiB; approximately 0.176 GiB has outside hardlink names | **Unresolved.** Inspect current evidence consumers before release |
| `/home/paul/library-context/build/evidence-target` | 0.202 GiB | **Unresolved**, low immediate priority |
| `/home/paul/library-context/build/envs` | 0.161 GiB | **Retain** pending environment-owner review |
| `/home/paul/library-context/build/fixtures` | 0.038 GiB at census; live fixture observed | **Retain active state** |
| Other build material | Approximately 3.014 GiB outside runs in total | Classify by consumer; not a first cleanup target |
| `/home/paul/.cache/library-context-worktrees/graph-native-model-build` and `graph-native-compiler-build` | Coordinator observed roughly 3.8 GiB of additional Cargo outputs outside the main census | **Remove candidates**, separately inventoried before action |
| `/home/paul/.cache/library-context-worktrees/compiler-workspace-check` | Contains a non-Git Cargo source copy | **Unresolved; protect source** |
| PC3’s registered dirty isolated copy | Named current preservation requirement | **Retain** |

## 3. Responsibilities and contracts

| Owner | Responsibility and consumer contract | Assessment |
|---|---|---|
| Cargo configuration and profiles | Select output locations, optimization, features and reuse settings | Coherent ownership; preserve shared default and BC3 transition |
| Cargo/rustc | Fingerprints, scheduling, incremental units and compilation | Delegate these capabilities; do not invent artifact-key interpretation |
| sccache | Bounded reuse of cacheable compiler work | Distinct from Cargo freshness and rustc incremental state |
| `build_environment.py` | Resolve intentional versus inherited paths and expose effective selection | Useful explicit routing; own-worktree selection applies through recipes/`just env --`, not automatically to bare Cargo |
| `worktree.py` | Preserve source changes, independent revisions, environments and selected isolated build state | Existing ownership boundary is useful; retirement must consume cleanup certainty |
| `runs.py` | Attempt identity, execution observation, cancellation, retention and pruning | Existing composition owner; cleanup correction already belongs to AF1 |
| `compile_profile.py` / capture helper | Capture provenance and separate product from telemetry outcomes | Useful evidence boundary; automatic retention and diagnostic storage need correction |
| Current plans | Identify present evidence consumers and qualification obligations | Consumer retirement must explicitly release the evidence obligation |

The lifecycle needs to distinguish:

- Rebuildable compiler cache from non-recomputable observations of a historical execution.
- Shared active cache from an attempt/worktree’s isolated mutable outputs.
- Compact receipts from bulky raw evidence and regenerable reports.
- Process exit from confirmed cleanup.
- A retention reason from its review date and release condition.
- Re-reporting captured evidence from recompiling the same source.
- Path identity from a portable evidence reference.

These distinctions need ordinary owned operations and modest metadata in existing run/worktree boundaries. They do not justify a universal storage registry or daemon.

The code-intelligence profile constrains preservation, but no product fact family, analytic or served answer is redesigned here. Provider attribution, pinned analysis inputs, private evaluation data and product evidence closure must remain unchanged. CI-G1–CI-G3 are therefore **not applicable to the bounded storage review**, rather than certified by it.

## 4. Growth and physical execution

**Implemented evidence, inspected 2026-10-09:** `.cargo/config.toml` selects shared intermediates with fine-grain locking. Root profiles enable workspace incremental compilation while imported dependencies use O3 without incremental compilation. These are distinct reuse scopes.

Isolation multiplies physical compiler state when separate roots are selected. That can be justified for an independent revision, incompatible mutable state or controlled benchmark. It is avoidable when isolation is only a transient command choice whose outputs remain indefinitely after its task ends.

**Implemented evidence:** `compile_profile.py:610–618` automatically touches the run’s retained marker. Attachment also retains the run at approximately line 1143. `runs.py:722–771` supports manual release and age/count-based whole-run pruning. There is no raw-versus-receipt lifecycle or byte-based retained-set policy in those inspected operations.

**Implemented evidence:** `compile_profile_capture.py:399–404` requests `default,args,llvm` self-profile events and monomorphization statistics for selected compiler units. Sampling uses 99 Hz CPU-clock samples with 16 KiB DWARF stacks and 45-second rotation at lines 125–149. Rotation makes closed chunks observable; it does not bound cumulative retained bytes.

The 8 GiB free-space floor applies to the perf observer after rustc has launched with self-profile flags. Inspection of `capture_rustc`, approximately lines 468–586, establishes that stopping the sampler does not stop rustc’s self-profile output. The larger diagnostic writer therefore remains outside this safeguard.

The census establishes the present distribution, not a growth rate since October 1. Multiple profiles, isolated roots, compiler variants and detailed captures are credible accumulation mechanisms. Their exact creation history, reuse value and future growth require additional evidence if quantitative claims are needed.

## 5. Change and failure scenarios

| Scenario and kind | Expected owner and route | Current consequence / proposed boundary |
|---|---|---|
| Ordinary edit and focused release test — instance | Cargo’s existing shared root and fingerprints | Preserve reuse; do not routinely clean or impose CPU/thread caps |
| Independent dirty revision — contextual binding | Worktree owner selects shared or justified own build root | Preserve source isolation; own build root receives the task’s retirement condition |
| New diagnostic capture format — domain extension | Compile-profile owner declares required sidecars, reader revision and replay contract | Current whole-run retention cannot express independent raw/report/receipt lifetimes |
| Replace perf with another sampler — mechanism substitution | Capture adapter preserves exact target identity and telemetry/product separation | No consumer should need to reconstruct cleanup or retention rules |
| Re-report an interrupted historical capture — evidence composition | Exact capture and pinned readers, with partial state retained | Summaries alone cannot satisfy this operation; archive must restore required sidecars |
| Disk pressure during profiling — failure/growth | Diagnostic owner stops bounded optional telemetry and reports its outcome | Current floor leaves rustc self-profile writing; it does not protect the complete diagnostic operation |
| Retire an isolated task root — lifecycle policy | Known owner, released consumer obligation, confirmed cleanup and fresh selector check | Current legacy roots require historical reconstruction; age alone is insufficient |
| BC3 installs qualified local-test settings — policy transition | Compilation-cost plan migrates profile selection and retires the candidate | Retain current candidate until that decision; profile cache retirement follows its actual consumers |
| Concurrent native sync/build — composition | Existing environment ownership plus Cargo coordination | Storage maintenance must refuse or defer around live owners; no global serialization of healthy builds |

A legitimate diagnostic investigation may need full LLVM events and historical raw replay. The remedy must preserve that capability as explicit detailed work. Conversely, ordinary attribution should not acquire that storage cost automatically.

## 6. Findings

<a id="F01"></a>

### F01 — Whole-run retention conflates durable receipts with bulky replay evidence

**Principles:** FP-01, FP-04, FP-07; DP-05, DP-19–DP-22; A2/A4.

Automatic retained markers protect all profiling material indefinitely. Manual release removes that protection for the whole run. Neither operation expresses which bytes a current consumer requires, which reports are regenerable, or when that obligation ends.

The consequence is visible: retained runs dominate the run footprint, while ordinary prune can address only about 0.438 GiB. Repeated profiling therefore accumulates storage independently of the useful receipt history. Deleting raw data indiscriminately would instead break current replay consumers.

**Proposed correction:** keep retention decisions with the existing run/compile-profile owners. Distinguish compact receipt, raw replay evidence and derived reports; record the reason/current consumer and release condition. Permit raw evidence to expire independently only after its consumer releases it or a validated archive preserves the declared replay contract. Existing retained runs remain protected during transition.

**Closure evidence:** inspect the governing operation and exercise one cited completed capture and one partial capture through archive/restore or deliberate consumer retirement. Re-report must preserve source/settings identity, required sidecars and partial/completed meaning. Missing raw data must produce an explicit unavailable/archived state. A summary cannot be silently promoted to raw replay capability.

**Disposition:** required storage-lifecycle decision. If scheduled, transfer this source ID to one active plan’s disposition table; this review creates no competing execution ledger.

<a id="F02"></a>

### F02 — Isolated compiler roots lack an explicit lifetime tied to their task

**Principles:** FP-01, FP-05–FP-07; DP-09, DP-19–DP-21; A1/A4.

Eight roots retain 130.504 GiB of regular files after their original ad hoc compilation tasks and former source worktrees. Recovering their meaning required historical commands and dep-info rather than a current retirement contract. No durable owner decision was found in the matched original calls.

This is not a finding that isolation itself was unnecessary. The defect is that a temporary physical placement can become indefinite cache retention without a consumer or retirement boundary.

**Proposed correction:** keep the shared root as ordinary default. Justified own roots attach to an existing worktree/run owner, state their purpose and release condition, and are retired after source/evidence obligations and cleanup are settled. Make ad hoc overrides follow the same small contract. No additional registry is needed.

Treat the eight roots as the first removal candidates. Do not merge their internal caches or infer last use from modification time. Shared-root maintenance is a separate, deliberate operation performed only when relevant writers are quiescent.

**Closure evidence:** a future disposable task demonstrates creation, effective-path reporting, preservation while active/dirty/retained, and retirement after release. The legacy-root decision records fresh selectors/activity, content classification and independently reclaimable accounting. Test own-worktree recipe routing and its documented bare-Cargo distinction.

**Disposition:** required storage/workspace decision. Existing worktree cleanup-certainty changes remain with AF1.

<a id="F03"></a>

### F03 — Profiling’s storage safeguard does not govern its dominant diagnostic writer

**Principles:** FP-07; DP-18–DP-22; A4.

Detailed self-profiling is enabled before rustc starts, while the free-space floor governs only perf. Raw self-profiles account for 82.821 GiB in this inventory, substantially more than perf data. The safeguard therefore cannot establish that profiling leaves sufficient shared disk for its compiler, fixtures or concurrent work.

**Proposed correction:** make ordinary attribution use a lighter, byte-bounded observation route. Require explicit selection of detailed compiler/LLVM recording for a named question, with adequate storage assessed before adding instrumentation. Record mode, storage premise and telemetry limits.

A sampler can stop independently and report partial telemetry. Rustc self-profile output cannot simply be disabled halfway through that compiler invocation. Detailed recording must therefore state its storage support limit honestly: an admission estimate is not a hard capacity guarantee. If stronger isolation is required, qualify an appropriate separate storage arrangement and its failure semantics before claiming it protects shared disk.

Preserve ordinary command outcome semantics. Do not silently terminate a healthy compiler to satisfy a telemetry limit or report a truncated capture as complete.

**Closure evidence:** focused controls establish mode selection, low-space behavior before instrumentation, aggregate sampler-byte stopping and truthful partial outcomes. Demonstrate that the chosen detailed-storage arrangement handles its stated limit. Verify compiler outcome remains separate from collector outcome.

**Disposition:** required diagnostic-policy decision; detailed-capacity qualification remains open.

### Existing recovery obligation

The [agent-effectiveness follow-up plan §3.1 / AF1](../../plans/agent-effectiveness-followup-plan_2026-10-09.md#31-run-ownership-recovery-and-stored-observations) already owns separation of process exit from cleanup certainty, protection of unresolved cleanup, and migration of run, profile and worktree consumers.

The present `runs.py` prune path selects records with a termination field and no retained marker. Historical termination alone does not establish cleanup. Storage retirement must consume AF1’s correction; this review does not introduce another recovery mechanism or duplicate its mutable disposition.

## 7. Architectural and gate judgments

### Foundations

| Foundations / supporting rules | Verdict | Reason |
|---|---|---|
| FP-01/02, DP-08/15/17 — coherent owners and explicit tool boundaries | **Satisfied within existing build composition** | Cargo, environment, worktree, run and profile owners have useful responsibilities |
| FP-03/06, DP-13/14/16 — composition and local reasoning | **Satisfied with identified lifecycle corrections** | Existing modules suffice; no service or generic workflow framework is warranted |
| FP-04/05, DP-02/05/19/21 — lifecycle meaning and governed behavior | **Violated in current assembled lifecycle** | Cleanup certainty is not represented by historical termination; raw-evidence and receipt obligations are not independently governed |
| FP-07, DP-20/22 — execution fit and coordinated retained resources | **Violated** | Retention grows without consumer expiry or byte policy; the diagnostic floor misses the dominant writer |
| DP-09 — valid reuse keys | **No violation established in Cargo reuse** | Artifact multiplicity does not prove invalid or redundant reuse |
| Product CI principles | **Not applicable to this bounded judgment** | Product facts, analyses, serving and evaluation semantics were not reviewed or changed |

### Gates

| Gate | Verdict | Evidence / required action |
|---|---|---|
| G1 Authority | **Pass, bounded** | Cargo remains artifact authority; existing run/worktree owners should govern lifecycle policy |
| G2 Semantic fidelity | **Fail for current cleanup interpretation** | Historical termination does not encode cleanup certainty; AF1 owns correction |
| G3 Validity | **Unresolved for safe retirement** | Fresh ownership/selector/activity checks and AF1 protection must precede removal |
| G4 Hidden behavior | **Pass for this review’s inspection; unresolved for proposed maintenance** | Audit is read-only; maintenance effects must be explicit |
| G5 Consistency and recovery | **Fail in current assembled lifecycle** | Existing AF1 obligation; capacity limits do not cover the complete diagnostic writer set |
| G6 Transformation and reuse | **Unresolved for archive substitution** | Current absolute raw-path consumers require validated portability/restore before replacement |
| G7 Truthful claims | **Pass for bounded review claims** | Inventory, static implementation evidence, historical receipts and Proposed remedies remain separate |
| G8 Library leverage | **Pass, bounded** | Delegate compiler caching/scheduling and sampler/reader mechanics; implement only project lifecycle semantics |
| CI-G1 / CI-G2 / CI-G3 | **Not applicable** | No product fidelity, served evidence or evaluation qualification is claimed |

| Judgment | Verdict | Scenario evidence |
|---|---|---|
| A1 Localize change | **Violated for isolated-root retirement** | Current ownership requires reconstructing command history after the task ends |
| A2 Encode domain meaning explicitly | **Violated in assembled lifecycle** | Cleanup and evidence-lifetime distinctions do not fully govern retirement behavior |
| A3 Extend through composition | **Satisfied, bounded** | Existing owners can compose the corrections; preserve their responsibilities |
| A4 Fit execution to supported workload | **Violated** | Accumulating roots and retained diagnostics threaten ordinary work on the same disk |

## 8. Library fit and alternatives

**Interface-checked, 2026-10-09:** the pinned Cargo source at revision `3d7cf6e93` initializes both target and build directories and removes both in a broad no-package clean. Its implementation explicitly avoids acquiring a lock for that operation. Consequently, a broad clean is not a safe concurrent cache-retirement primitive. [Pinned Cargo clean source](https://raw.githubusercontent.com/rust-lang/cargo/3d7cf6e93/src/ops/cargo_clean.rs).

Pinned unstable documentation’s global-cache cleaning controls concern downloaded sources, archives, indexes and Git material. They do not establish an intermediate-artifact LRU for this shared build root. The coordinator’s Context7 documentation lookup is a discovery aid; exact-version source governs this boundary.

sccache and incremental compilation cover different work. The coordinator’s resolved sccache documentation establishes that incremental Rust work and final linking are outside its cacheable scope. Clearing one layer cannot be assumed harmless because another exists.

| Alternative | Benefits | Costs / decision |
|---|---|---|
| Preserve every cache and capture indefinitely | Maximum historical local availability | **Reject as complete target:** no end to retained growth |
| Routinely clean all Cargo outputs | Simple space reset | **Reject:** destroys useful reuse and can race active shared work |
| Disable incremental compilation | Removes that storage class | **Not selected:** rebuild tradeoff unmeasured; does not address evidence or isolated-root lifetime |
| One isolated root per task | Strong mutable isolation | **Use only when justified:** multiplies compiler state and requires retirement |
| Shared default plus explicit task-owned exceptions | Preserves ordinary reuse and necessary isolation | **Preferred Proposed direction** |
| Age/count-only whole-run pruning | Already available and simple | Retain for ordinary released runs; insufficient for large retained captures |
| Compact receipts plus consumer-governed raw evidence | Preserves diagnostic meaning with independent lifetimes | **Preferred Proposed direction**, requiring archive/restore qualification |
| General cache daemon or storage registry | Could automate broad policy | **Not justified:** one operator and existing owners favor manual scoped maintenance first |

## 9. Proposed operating policy

These are initial **advisories and review triggers**, grounded in the observed footprint and largest approximately 30.157 GiB capture. They are not measured capacity guarantees, CPU/test concurrency limits or permission to delete protected material.

| Class | Proposed starting policy | Preservation boundary |
|---|---|---|
| Shared filesystem headroom | Restore at least **64 GiB free** before new disk-heavy work; aim for **100 GiB** before detailed profiling | Concurrent writers and unmeasured peaks can exceed this allowance |
| Shared Cargo intermediates | **200 GiB review trigger**, with deliberate idle maintenance when pressure or retired profiles justify it | No automatic hash-directory eviction; active release and pending BC3 remain protected |
| Isolated task roots | Review after **7 days without an active consumer**; target retirement within **30 days after consumer release** | Calendar age or mtime cannot release live, unknown, dirty or retained material |
| Raw profiling hot set | Initial **64 GiB / 14-day review target** for unreferenced closed evidence | Cited captures are explicit counted exceptions until archived or released |
| Compact receipts | Keep while a current plan, review or replay consumer requires them | Retirement follows consumer meaning, not raw-byte age |
| Derived reports | Regenerate where raw evidence and exact readers remain available | Preserve reports that are themselves cited observations |
| sccache | Keep its existing bounded configuration during initial remediation | Reassess separately with host consumers; no repository-only purge of a shared cache |

The five cited captures already exceed the proposed raw hot-set target. They must remain visible in total accounting as exceptions. They cannot be excluded from the footprint merely because they are protected.

Start with manual, scoped maintenance and existing command owners. Add automation only after the rules prove useful and a concrete repeated operator burden justifies it.

## 10. Ordered recommendations and focused acceptance

| Order | Recommendation | Owner / acceptance |
|---|---|---|
| Immediate | Publish the inventory and consumer classifications; preserve the original census | Review/evidence author; publication checks only |
| Immediate | Prepare a concrete removal proposal for the eight old isolated roots and separately inventoried output-only worktree caches | Fresh ownership, selector, content and process checks; no source/evidence loss |
| Immediate | Preserve cited captures and pending BC3 material | Current plan consumers remain authoritative |
| Structural prerequisite | Consume AF1 cleanup certainty in prune and worktree retirement | Existing AF1 owner and focused real-process recovery controls |
| Structural | Separate receipt/raw/report retention and release conditions | Run/profile owner; completed and partial replay/retirement cases |
| Structural | Make detailed capture explicit and govern diagnostic admission/bytes | Profile owner; low-space and partial-telemetry controls |
| Subsequent | Review shared profile/cache retirement after BC3 and other consumer changes | Cargo/profile owner; current effective paths and required rebuildability |
| Conditional | Validate portable archival storage | Exact restore and re-report with required sidecars/readers; measured space benefit |

Priority by immediate capacity consequence starts with the old isolated roots. Safe automation’s prerequisite is cleanup certainty and an explicit consumer-release contract. Those are different orderings.

Acceptance should remain focused:

- Exercise disposable run/worktree retirement with live, cleanup-unknown, retained, dirty and released cases.
- Exercise one completed and one interrupted capture through the selected replay/archive lifecycle.
- Verify profile selection and effective build/target paths remain truthful.
- Run affected script controls and applicable documentation/agent leaves after implementation.
- Do not run product qualification solely for this report or its small evidence script. BC3 and existing native/product acceptance retain their own schedules.

## 11. Rule impacts

This review proposes these changes; it does not apply them.

| ID | Current rule / location | Proposed change and dependencies | If retained unchanged |
|---|---|---|---|
| **RC01** | Blanket preservation of shared caches/intermediates and captures in AGENTS, STATUS and compilation-cost plan | Preserve active/current-consumer state; permit deliberate owner-checked retirement of rebuildable dormant roots. Enables F02 | Legacy roots require continuing explicit exceptions; lifecycle target remains incomplete |
| **RC02** | Profiling runs automatically retained; raw evidence preserved through compilation-cost plan §11 and profile/run code | Separate compact receipts, raw replay evidence and reports; release raw only through consumer retirement or validated archive. Enables F01 | Retained diagnostic footprint continues accumulating |
| **RC03** | Default selected-unit `default,args,llvm` recording and simultaneous sampling | Make detailed LLVM/compiler capture explicit; ordinary attribution uses lighter bounded observation. Enables F03 | Every selected recording retains the larger diagnostic writer |
| **RC04** | 8 GiB profiling floor described as stopping perf telemetry | Replace the operational premise with aggregate diagnostic admission, byte-aware sampling and explicit detailed-storage limits. Enables F03 | The floor remains truthful only as a perf safeguard, without complete disk protection |

No toolchain, dependency pin, ordinary compiler/test parallelism or BC3 qualification rule needs changing for the preferred direction. Those are preservation constraints, not reasons to retain obsolete physical roots indefinitely.

Scheduled findings must land in one active plan table using the stable IDs from this review. AF1 remains the existing cleanup/recovery disposition owner.

## 12. Verification, uncertainty and decision

| Claim / activity | Evidence strength and outcome |
|---|---|
| Three-root storage distribution | **Measured**, 2026-10-09; coordinator’s read-only inode census **passed** with no recorded scan errors |
| Original use of isolated roots | Historical command attribution supplied by coordinator; establishes original compilation purpose, not current absence of use |
| Active process references | Read-only `/proc` survey **passed within stated visibility**; live shared Cargo and fixture references observed |
| Retention and diagnostic writer behavior | **Implemented evidence**, independently inspected 2026-10-09 |
| Exact Cargo broad-clean behavior | **Interface-checked**, pinned source inspected 2026-10-09 |
| Safe deletion of any candidate | **not_run**; fresh pre-action checks required |
| Archive portability and replay | **not_run**; destination and restore contract unqualified |
| Proposed retention/headroom settings | **Proposed**; not a measured safe-capacity envelope |
| Product, BC3 or native qualification | **not_run by this review**; existing receipts and owners retain their limits |
| Documentation publication | **passed**, coordinator's `just docs`, 2026-10-09: 367 pages, zero link errors. Full `just docs-check` **failed** on an already-stale concurrent ADR index; no unrelated index repair was made |
| Evidence-script source checks | **passed**, coordinator's scoped Ruff check/format check, Python 3.12 syntax and JSON parsing, 2026-10-09 |

The evidence does not establish exact redundant compiler bytes, last-use dates, a sustained growth rate, compression savings, or deletion safety. These uncertainties affect the selection and size of cleanup actions. They do not remove the demonstrated lifecycle and diagnostic-writer defects.

**Bounded decision: Revise the build-storage/workspace lifecycle.** Preserve the shared Cargo default and useful run/worktree composition. Correct evidence retention, isolated-root retirement and diagnostic storage admission through their existing owners.

**Enclosing architecture:** needs revision for the complete storage-lifecycle target; product architecture and release qualification are not certified by this review.

The next consequential decision belongs to the coordinator/operator: confirm the proposed lifecycle and rule changes, then prepare owner-checked retirement of the old isolated roots. Cited raw evidence remains protected pending deliberate consumer release or validated archival replay.

**Intended publication path:** `docs/design_review/reviews/design_review_build-storage_2026-10-09.md`.

**Retention disposition update, 2026-10-10:** the operator selected preservation of reports and receipts with retirement of the five obsolete raw compiler captures listed above. This supersedes their raw-local retention recommendation only. Hash-verified independent report/provenance bundles now exist under each run’s `compile-profile-reports/`; historical size readings and review judgments retain their original date/scope. The [storage plan §10](../../plans/storage-lifecycle-management-plan_2026-10-09.md#10-current-checkpoint) owns actual retirement receipts and current disposition.
