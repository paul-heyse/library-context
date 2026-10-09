# Storage lifecycle plan target review

**Independent principal design/target review · 2026-10-09**

**Decision: Accept scoped, at Proposed target strength, after revision.** The storage architecture has appropriate lifecycle granularity and credible execution routes. It preserves warm reuse, delegates Cargo and native-cache internals, separates historical observations from rebuildable outputs, and makes unknown ownership protective. Initial review required revision of atomic parent/descendant admission and portable sampled-replay dependencies. Static re-review confirms both corrections in the target and its revealing acceptance cases. Implementation, operational qualification and source-review F01–F03 closure remain open; no operational action is authorized by this review.

## 1. Scope, baseline and method

| Field | Assessment |
|---|---|
| Subject | [Storage lifecycle management plan](../../plans/storage-lifecycle-management-plan_2026-10-09.md), including its run/profile/worktree/environment/shared-tool consumers |
| Tier · purpose | Design · target |
| Standard | Core/template 3.3, efficient-architecture heuristics 1.0, code-intelligence profile 1.5 and library-context binding |
| Baseline | Main `3258926dce66533bfb5ff9dbdbf7aaadf59f2ed8`, with extensive concurrent native source, ADR, DESIGN and STATUS changes preserved; the proposed plan was untracked |
| Reviewer | Independent delegated design reviewer, 2026-10-09 |
| Functional target | Automatic rules-based lifecycle for repository and cross-repository host assets, preserving immediate warm reuse, current detailed profiling and healthy compiler parallelism, while retiring released outputs safely and exposing residual capacity/ownership limits |
| Workload | Concurrent managed builds, environments, fixtures, profiling and shared-skill maintenance; hundreds of GiB across a finite shared filesystem; small lifecycle records, potentially large closed captures, independently disposable task roots |
| Method | Static owner/source inspection and scenario reasoning; installed native perf manual inspection; exact primary Cargo/sccache/uv documents linked from plan §3.5. No tests, builds, captures, cleanup, archive transfer, synchronization or repository edits |
| Exclusions | Product extraction/analysis/serving correctness, operator stores, BC3 qualification, real-library campaigns, live consumer completeness, quantitative speed/capacity/reclamation claims |

The prior build-storage review and its 2026-10-09 census establish historical findings and inventory, not present deletion permission. Their F01–F03 remain distinct from this review's IDs. The storage plan §9 owns their scheduled disposition; agent-effectiveness AF1 retains cleanup/recovery ownership. This review does not rewrite either source ledger.

## 2. Responsibilities and domain authority

| Owner | Responsibility and consumer boundary | Judgment |
|---|---|---|
| Policy configuration and lifecycle evaluator | Category rules, explicit temporary obligations, release/expiry and reasons; inventory remains derived | Suitable small semantic owner; executable values avoid competing prose defaults |
| Run/fixture/environment/worktree owners | Supply current execution, cleanup, retained-content and native-use facts; perform native effects | Reuse these owners; never infer cleanup from a copied terminal field |
| Profile owner | Raw/report/receipt components, reader identity, relocation, archive/restore and declared replay capability | Appropriate owner; revised F02 contract makes replay dependencies explicit |
| Host participation/shared asset owner | Repository membership, consumer obligations and maintenance use across registered consumers | Necessary for real shared assets; incomplete membership remains protective |
| Retirement operation | Managed admission, identity revalidation, same-filesystem retirement, journal and exact-object recovery | Appropriate operation granularity; revised F01 contract coordinates enclosing-root admission |
| Automatic schedule | Best-effort catch-up over eligible managed groups, with compact outcomes | Appropriate orchestration; no new source of semantic eligibility |

The model deliberately distinguishes physical identity from logical identity; execution outcome from cleanup certainty; immediate reuse from replay; named consumers from expiring temporary use; and local raw from archived availability. Missing metadata and unreadable participants are unknown, never an empty consumer set. A nonce plus device/inode and an external lock/journal protect a managed lifetime against stale paths and root reuse. These distinctions should govern owner operations, rather than become output-only labels.

Code-intelligence facts, graph projections, evidence-backed product answers and evaluation populations are not redesigned. No fact/fidelity or analysis-record table is invented for filesystem lifecycle records. CI-G1–CI-G3 are not applicable to this bounded review; preservation of sealed inputs and product state remains required.

## 3. Contracts and composition

Managed eligibility requires owner release or an explicitly declared temporary-use expiry, no live or dependent consumers, confirmed cleanup, no active use and elapsed grace. Pressure never manufactures release. New isolated roots are stable for one lifetime; legacy roots remain protected until quiescent adoption establishes a real boundary. This avoids guessing Cargo cache keys or last use from names/mtime.

AF1's implemented-and-focused-tested cleanup/reader slice must precede destructive run/worktree operations. Merely agreeing the schema is insufficient. Retained terminal evidence and kept stopped fixtures must survive parent deletion checks; all relevant Python/service resources include vLLM. Existing code validates the need: `worktree.py:357–425` currently selects live runs/fixtures, `:427–441` checks only the native environment route, and `:493–504` removes the checkout and own build root. `runs.py:744–771` still uses terminal records and a whole-run marker. The plan correctly schedules replacement instead of claiming these contracts already work.

Archive publication must preserve raw bytes and collector meaning, validate all entry scope/digests, and verify the declared replay capability before local redundancy is retired. The original collector log cannot be text-rewritten: `compile_profile_capture.py:272–314` recognizes absolute closed-chunk announcements, including signal-finalized collectors. The proposed prefix-bound relocation resolver and separate historical versus regenerated reports are justified.

## 4. Execution fit and alternatives

The chosen default preserves existing compiler/package/model caches and settings, while retiring whole owner-released groups. Small producer updates and asynchronous release hooks keep maintenance outside a build's critical path. Daily catch-up examines known groups and top-level discovery; deep inode accounting is explicit and dated. This is materially less work than a recurring full-tree census or artifact-level LRU, and avoids turning opaque compiler variants into application policy.

| Alternative | Assessment |
|---|---|
| Manual instructions and existing whole-run pruning | Smallest code change, but cannot deliver automatic component lifetimes, parent protection, shared obligations or recovery |
| Cargo/native cache GC only | Preserve it for opaque native caches; it cannot decide historical evidence consumers or worktree-owned task release |
| Per-artifact registry, central database, mandatory daemon or universal plugin engine | Adds needless state and coordination for one local operator; no demonstrated consumer |
| Proposed ordinary Python modules, small external records and systemd catch-up | Proportionate to actual cross-repository ownership and crash obligations, provided effects continue through semantic owners |
| Rebuild-on-demand cache eviction or lighter profiling defaults | Does not preserve the functional requirement for immediate warm reuse and selected detailed attribution |

Exact-source inspection supports §3.5's native-mechanism limits: whole-root [Cargo clean at revision 3d7cf6e93](https://raw.githubusercontent.com/rust-lang/cargo/3d7cf6e93/src/ops/cargo_clean.rs) removes target/build paths without acquiring the build lock; [Cargo global GC](https://raw.githubusercontent.com/rust-lang/cargo/3d7cf6e93/doc/book/src/reference/unstable.md) concerns source/download caches. [sccache 0.17 Rust support](https://raw.githubusercontent.com/mozilla/sccache/v0.17.0/docs/Rust.md) does not replace incremental/linker outputs. [uv 0.12.22 cache documentation](https://raw.githubusercontent.com/astral-sh/uv/0.12.22/docs/concepts/cache.md) forbids direct cache mutation and notes centralized-environment removal by prune. These capabilities do not supply evidence consumer semantics.

GNU tar/zstd remain unqualified archive mechanism leads, rather than certified portable semantics. Native perf build-ID/archive/symbol-root capabilities are important alternatives to bespoke dependency interpretation (F02). Existing installed manuals explain the capability, but exact selected tool contracts and actual replay remain implementation acceptance work.

The plan's pressure behavior is candid: the perf floor protects only the sampler, self-profile flags cannot be withdrawn after launch, and a protected set can exceed finite storage. Explicit adequate placement and capacity reporting are credible routes; polling is not a hard guarantee against ENOSPC. This acceptance must not be relabelled as safe peak capacity, automatic warm-cache eviction or a promise that every future build fits. Archiving can temporarily require two copies, and same-filesystem movement alone frees no capacity.

## 5. Revealing change and failure scenarios

| Scenario · kind | Owner and expected propagation | Assessment / settling evidence |
|---|---|---|
| Add a new diagnostic sidecar · domain extension | Profile declaration adds capability/dependency, archive manifest and its independent revealing case | No unrelated cache policy edits; F02 requires external as well as local inputs |
| Replace perf sampler · mechanism substitution | Capture/profile adapter changes; stable outcome, raw availability and reader contracts remain | Preserve child/observer separation and closed-chunk semantics; no universal provider framework |
| Retain/create a child while parent removal starts · concurrency | Parent lifetime admission and child owner must compose atomically | Revised §3.4 addresses F01; SM2 must exercise interleavings |
| Old path is recreated after retirement rename · recovery | Exact identity/journal resumes old object only | Proposed nonce/revalidation/tombstone contract fits; focused real filesystem acceptance remains not_run |
| One of two repositories releases a shared skill/tool · composition | Shared owner retains other obligation; missing participant remains unknown | Adequate Proposed route; bounded discovery never proves global exclusivity |
| Replay after toolchain/target/debug locations disappear · dependency/lifecycle | Profile capability pins all required inputs or says unresolved | Revised §5 addresses F02; SM4 removes original-location access |
| Pressure after healthy profiled compiler starts · failure/growth | Observer reports partial/capacity condition; compiler remains under its existing owner | Preserve detail/parallelism; no false total-byte bound or exit-code substitution |
| Missing project environment · repair | System-Python observation works; native effects report actual prerequisites | Adequate split; no silent environment synchronization |

## 6. Stable authoring findings

<a id="F01"></a>

### F01 — Descendant protection lacks atomic admission with enclosing-root removal

**Priority:** high preservation consequence. **Principles:** FP-03–FP-06, DP-03/19/20; A2/A3, G3/G5.

Initial plan §3.4 gives each object shared-use/exclusive-retirement ownership and rereads descendant obligations before parent removal. It does not require descendant creation, retain or use to participate in the parent's lock. Parent removal can read an empty descendant set, a child can then publish a hold or begin use under its own lock, and the parent can rename/delete both. The same gap affects newly created profile/fixture content and environment admission. A fresh scan alone cannot close it.

**Correction:** use the smallest hierarchy protocol sufficient for actual enclosing effects: shared admission for child creation/use/retain against managed ancestors; exclusive admission for parent removal through revalidation and publication/removal. Acquire ancestors before children in stable order. Compose with native resources consistently, reuse validated inherited ownership, refuse self-upgrades, and skip instead of waiting on busy maintenance acquisitions. `workspace_env.py:71–112,323–370` already orders environments before extensions and reuses inherited same/stronger ownership; preserve that route rather than creating a conflicting lock order. Unmanaged access remains an explicit adoption/support limit.

**Closure evidence:** a revised target defines atomic parent/child membership and lock ordering, including nested managed calls and native ownership. Implementation acceptance interleaves parent removal with new child hold/creation/use and native-holder arrival, verifies both valid outcomes without loss, and checks no deadlock/global serialization. These controls are not_run.

<a id="F02"></a>

### F02 — Portable sampled replay omits external object/debug dependencies

**Priority:** high evidence-preservation consequence. **Principles:** FP-02/04/05, DP-08/09/15/21/22; A2, G2/G6/G7.

Initial plan §5 preserves raw, local sidecars and exact readers, but does not identify the binaries, shared objects, debug information or source needed for supported sampled attribution/annotation. Existing consumers invoke native `perf report` (`compile_profile.py:891–909`), `samply import` (`:1236–1248`) and named-symbol `perf annotate` (`:1250–1269`); recording requests DWARF call chains (`compile_profile_capture.py:125–145`). Successful replay while original toolchain/target/debug caches remain available can conceal this dependency and falsely qualify a portable bundle.

Installed native documentation corroborates the boundary: `/usr/share/man/man1/perf-archive.1.gz:31,39` describes collecting build-ID object files for portable analysis; `perf-buildid-cache.1.gz:81` retains older entries for historical annotation; `perf-annotate.1.gz:39–41,354–376` describes object/debug/symbol-root/source inputs. These are read-only manual observations, not executed replay evidence or a transfer of system-perf version qualification to the private selected binary.

**Correction:** declare each actual replay/report/view/annotation capability and all required immutable inputs. Preserve matched object/debug/source inputs in the archive or protect explicit external dependencies with their owner and exact identity. Missing inputs keep that capability unresolved; historical text can remain available independently. Consider native perf buildid-list/archive/build-id cache and explicit symbol-root facilities before bespoke reconstruction. Do not promise a complete source rebuild where none was captured.

**Closure evidence:** the revised target makes dependency closure and capability-specific availability explicit. Implementation acceptance runs exact declared consumers with the original source/output/debug-cache locations unavailable, checks meaningful symbols/annotation and preserved partial state, and refuses closure when external dependencies are absent. Reader exit0 alone is insufficient. These controls are not_run.

## 7. Re-review and authoring disposition

**Static re-review, 2026-10-09:** the revised plan §3.4 requires shared admission on enclosing managed lifetimes for creation/use/retain/release/selector/descriptor publication, parent exclusive exclusion, outer-to-inner storage ordering with canonical-ID order for disjoint roots before existing native resources, inherited reuse, refused upgrades and topology revalidation. SM2 now requires real concurrent child admission versus removal and nested/reentrant acceptance. This closes F01 in the Proposed target; implemented safety remains not_run. Parallel sibling use stays shared, and a busy retirement skips rather than globally serializing healthy commands.

Revised §5.1 declares separate compiler, symbolized sampled and disassembly/source-annotation capabilities, with build-ID-matched immutable object/debug/source inputs bundled as required and exact protected readers identified. Missing closure refuses archive substitution; consumers must explicitly change any named promise. §5.2 and SM4 now require meaningful replay with original output/debug/source locations unavailable on disposable data, preserving completed chunks and partial state. This closes F02 in the Proposed target; actual archive/restore/replay remains not_run. Existing historical captures are not represented as newly qualified bundles.

The revised verification boundary and SM8 acceptance explicitly require one matching-source assembled `just qualify` passed before activation because shared run/profile receipt consumers change. Coordination may share that run with coincident AF9 work; an older prerequisite receipt cannot establish matching-source acceptance. This preserves the existing assurance rule without adding any product gate to this documentation turn.

| Stable finding | Authoring resolution | Continuing implementation owner |
|---|---|---|
| F01 | Closed in Proposed target by §3.4 and SM2; static re-review on 2026-10-09 | Storage plan §9 / SM2–SM3–SM8; real interleaving controls not_run |
| F02 | Closed in Proposed target by §5.1–5.2 and SM4; static re-review on 2026-10-09 | Storage plan §9 / SM4–SM8; exact isolated-location replay not_run |

These are dated authoring judgments, not a competing mutable implementation ledger. The storage plan owns continuing disposition and should link these stable IDs. The final reviewed target snapshot was the revised 2026-10-09 plan (SHA-256 `0c3a6f1b93c71e155981f89d37a22dc2d619d844b8ba4f88b92ac927fb2f09b7`), before publication/checkpoint-only edits. The additional schema/scope/overlap/prospective-policy validation and tracked-reference reconciliation in §3.2–3.3 preserve the same judgment; references supplement declared obligations and never establish consumer completeness.

## 8. Judgments, authority and next action

| Judgment | Final verdict | Evidence |
|---|---|---|
| A1 Localize change | Satisfied at Proposed strength | Existing owners absorb lifecycle facts; one policy evaluator and profile resolver prevent consumer reinterpretation |
| A2 Encode domain meaning | Satisfied at Proposed strength | Revised hierarchy governs descendant admission; capability declarations govern replay dependencies and availability |
| A3 Extend through composition | Satisfied at Proposed strength | Revised parent/native/child ownership composes consistently; new categories and capture sidecars remain with their owners |
| A4 Fit execution to workload | Satisfied at Proposed strength | Group granularity, shallow catch-up, asynchronous hooks, preserved hot paths and explicit finite-capacity limits |

| Gate | Final verdict | Evidence / required action |
|---|---|---|
| G1 Authority | Pass at Proposed strength | Owner facts, policy, observations and shared obligations have distinct authorities |
| G2 Semantic fidelity | Pass at Proposed strength | Revised capability-specific dependency/availability contract; no unsymbolized-success substitution |
| G3 Validity | Pass at Proposed strength | Revised hierarchy defines atomic descendant admission and its revealing controls |
| G4 Hidden behavior | Pass at Proposed strength | Observation is read-only; effect commands/scheduling are explicit; no silent sync/restore |
| G5 Consistency and recovery | Pass at Proposed strength | Hierarchical exclusion, exact-object journal, nonce/revalidation and AF1 prerequisite |
| G6 Transformation/reuse | Pass at Proposed strength | Complete declared raw/settings/reader/symbol inputs, preserved logs and bounded relocation |
| G7 Truthful claims | Pass at Proposed strength | Explicit dependency gaps, ENOSPC support limit and Proposed/Implemented/historical boundaries |
| G8 Library leverage | Pass at Proposed strength | Cargo/native owners retained; ordinary modules suffice; native perf archive/symbol mechanisms require qualification during SM4 |
| CI-G1/CI-G2/CI-G3 | Not applicable | No product fact, served answer or evaluation meaning changes are reviewed |

FP-01–FP-07 are satisfied at Proposed strength for the named scenarios after revision. Relevant supporting-rule verdicts follow their gate rows; no unrelated exhaustive checklist is implied.

**Additional rule impacts: none.** F01/F02 strengthen the plan's intended preservation/replay contracts without changing settled warm-cache, profiling or concurrency requirements. The plan already names the operator-confirmed source-review RC01/RC02, rejected RC03, revised RC04 and automatic-maintenance choice; those are plan inputs, not new review confirmations. SM0 retains their ADR/DESIGN route. The revised plan explicitly retains the existing shared-receipt qualification trigger at integrated SM8; documentation review does not run that gate.

**Final bounded decision: Accept scoped** the revised Proposed architecture, retaining its preservation constraints. This accepts the named managed-lifetime and declared-replay design, excluding arbitrary unmanaged writers, guaranteed fit of unbounded protected storage, unqualified legacy/shared consumer completeness, and actual operational archive/retirement behavior. Unresolved adoption stays protected; implementation/assembled review and live qualification are the revisit points. **Enclosing architecture:** not certified; AF1, product/native acceptance, BC3, live shared-consumer discovery and operational storage qualification remain with their owners. No performance or reclamation claim follows.

**Checks:** source/manual/primary-document inspection passed as read-only investigation on 2026-10-09 (`sed`, `rg`, compressed native-manual reads and read-only source retrieval). Product/remediation tests, builds, archives, captures, synchronization, cleanup and qualification **not_run**, per read-only authoring assignment. No evidence folder is required. Intended publication path: `docs/design_review/reviews/design_review_storage-lifecycle-plan_2026-10-09.md`.
