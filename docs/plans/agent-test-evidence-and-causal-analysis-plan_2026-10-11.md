# Agent test evidence and causal analysis — standalone implementation plan

**Proposed · 2026-10-11 · implementation not started**

## 1. Outcome, basis and boundary

Give Codex a reliable way to determine what executed, what failed, what evidence survived and
what remains unknown; inspect selected original evidence; compare relevant execution contexts;
and construct the smallest semantically valid next experiment. Preserve every failure and its
membership when grouping related observations. Evidence organizes causal investigation; it does
not automatically prove a common cause or reinterpret an earlier result after a later pass.

The [source review](../design_review/reviews/design_review_agent-test-evidence-and-causal-analysis_2026-10-11.md)
supplies F01–F04, recommendations, alternatives and RC01–RC04. **This plan alone owns their current
scheduled disposition in §8.** The review retains its dated judgment and evidence. This is an
independent tooling scope: it neither changes existing functional plans nor diagnoses/closes the
preserved workspace failure inventory, resumes paused validation, or establishes product acceptance.

**Inspected baseline, 2026-10-11:** `main` at `64013142`. The dirty `.config/nextest.toml` and
untracked workspace-failure review evidence belong to other work and remain preserved. The
current `just test-all` already resolves ordinary scope through verification and stable fixture
owners. Implementing this plan extends that same path; it does not create another test catalog.

The applicable [standard](../design_review/design_principles/standard.toml) is core/template3.3,
heuristics1.0, code-intelligence1.5 and the repository binding. The workload is thousands of
Rust cases and Python phases, substantial captured output, repeated selective investigation,
interrupted runs and concurrent mutable checkouts on one workstation. The target minimizes
repeated execution and repeated evidence decoding while retaining honest coverage and attribution.
Quantitative speed/productivity benefits remain unmeasured.

## 2. Foundations and chosen architecture

| Foundation | Retained authority and required enhancement |
|---|---|
| `scripts/verify.py` | Owns boundary catalog, selection, requirements, command plans and verification outcomes. Add invocation/case/capture contracts and selective read operations; persist step observations before waiting for a whole boundary. |
| `scripts/runs.py` / `scripts/harness.py` | Continue owning process identity, actual child exit, launch/supervisor errors, cancellation, descendant cleanup and direct file capture. Verification never reconstructs process liveness or replaces supervision. |
| Nextest / pytest | Own discovery, attempts, phases, skips, expected failures and runner results. Qualified adapters preserve their meanings and information loss. |
| Environment / fixture / service owners | Supply actual protected environment, stable service generation, logical attachment and operation identities. Readiness never synchronizes; no scratch database or new service lifecycle. |
| Storage lifecycle | Admission, named consumer obligations, reader protection, confirmed release and retirement govern originals and derived products. Receipt survival does not promise raw evidence survival. |
| Compile profiling | Remains the opt-in compiler CPU/query analysis owner. Ordinary capture reuses suitable provenance mechanics, not profiling dependencies or its whole lifecycle. |

**Chosen Proposed route:** native artifacts plus a thin, rebuildable verification read model.
Keep authoritative originals under run-owned storage. A qualified adapter prepares selective
access once per artifact generation. Query handlers consume that preparation and hydrate only
selected payloads. No telemetry database, permanent diagnostic service, semantic search,
replacement harness or mandatory graph is needed.

Use file-backed record/payload streams and compact case metadata with seekable offsets. For
pytest reportlog and supported Cargo JSON, native line byte ranges are already locators. For
completed Nextest JUnit, stream the XML once and materialize normalized case/attempt payloads
with locators back to the source artifact and element identity. Store large decoded output in
seekable payload ranges rather than inside a JSON object that must be loaded whole for every
excerpt. This copy buys selective access
without a private native-ZIP decoder. Keep payloads out of the small overview/index metadata.
Use standard-library JSON/XML/file facilities unless qualification reveals a concrete missing
capability; do not add a database solely to maintain this run-local access path.

Offsets identify a particular physical projection generation, not a semantic case. Stable
evidence references bind invocation, original artifact identity/generation, native case,
attempt/phase and adapter interpretation. Rebuilding may relocate derived bytes without changing
that reference. Preserve original normalized-text losses, native truncation and missing fields.

### 2.1 Alternatives and reconsideration

| Alternative | Decision and reconsideration condition |
|---|---|
| Boundary logs/receipts alone | Preserve as channels; insufficient for case/phase/context and interrupted-run access. |
| JUnit alone | Preserve optional interchange; final-session output and normalized text cannot establish complete live execution or original-byte fidelity. |
| Native replay alone | Useful rich whole-run investigation; insufficient for repeated selective cross-tool queries. Compose it instead of replacing it. |
| Pinned native recording reader | Reconsider if JUnit/projection duplication becomes material and a supported reader offers selective access with simpler total ownership/versioning. Do not parse private ZIP members as an assumed API. |
| `pytest-json-report`1.5.0 | Inspected alternative, not the selected producer: final-session reporting and additional dependency do not resolve prefix/start gaps. |
| Custom general reporter / telemetry database | Reject for this scope: duplicates native semantics and adds coordination/retention machinery without a demonstrated consumer. |

Future producer upgrades belong in adapter qualification; new boundaries continue through the
existing catalog; new supported result kinds extend the shared projection rather than every CLI
handler. Additional output channels are introduced only for a named coverage gap.

## 3. Contracts and data flow

All contracts below are **Proposed**. Keep their executable definitions together under
verification-owned tooling modules; neither prose nor a collection of loosely typed CLI responses
may become a competing semantic authority. Local module names remain implementation choices.

### 3.1 Invocation and durable observations

Allocate a unique invocation identity per launched step, scoped by run/boundary/step and actual
attempt. Persist the resolved argv array, cwd, selected Cargo and runner profiles, selection,
tool/version, requested/effective capture channels and owned artifact destinations before launch.
Capture native UUIDs and fixture/environment identities only from supported evidence, never
from `latest`, a mutable fixed path or timestamp proximity.

Resolve the invocation directory while the run is owned and keep that path explicitly. Existing
`runs.current_run()`/`summary_path()` depend on live-owner observation; later failure recovery
must not rediscover the path through those helpers. Atomically publish small invocation snapshots
and commit complete event/record prefixes. A partial final line is not an event. Persist command
completion before post-step work such as serving registration or fixture restart; record those
operations and final fixture/cleanup results separately. A later cleanup failure cannot erase
completed command/case observations or turn them into a different native result.

Native outcomes and raw exits remain distinct from repository `passed`/`failed`/`blocked`/
`not_run`, run termination and cleanup certainty. Capture status separately reports requested,
effective, available, missing, partial, truncated, unsupported and failed channels, with reasons
and readable watermarks. Passing tests with failed recording remain passing observations with
failed capture. Missing completion never establishes success or definite nonexecution.

Preserve existing run/summary records and boundary-rerun readers. Use additive sidecar contracts
and links; do not rewrite historical originals or silently reinterpret existing schema fields.

### 3.2 Context and executable attribution

Record relevant tracked and untracked source content identities, revision, lock/toolchain files,
effective runner configuration and non-secret execution settings once per actual shared context
lifetime. Reuse lightweight source mechanics from compilation profiling, with explicit included/
excluded paths, symlink/file-change failures and unavailable observations. Exclude protected
gold/heldout bytes, generated stores, environments, credentials and unrelated workspace data.
Do not copy the full workspace or run optional profiler-tool checks for every test invocation.

Observe context at launch and completion, and again before a later step when prior assumptions
changed. Sharing observations does not prove no intermediate edit occurred. Capture available
Cargo/native artifact identities and relevant build observations separately. A binary digest
identifies executable bytes; it does not itself prove their source closure. Unknown source-to-
binary provenance remains unknown. Identical HEADs with different dirty/untracked bytes must
remain distinguishable. Preserve explicit configuration overrides and omitted-secret coverage;
no exported context is advertised as sanitizing arbitrary native stdout/stderr.

Use existing service generation, attachment/attempt and operation identities for substrate
context. Native phase IDs are process-local: qualify them by invocation/process and applicable
service/attachment/operation. Shared timing or similar error text is not a causal edge.

### 3.3 Selective inspection and reference stability

Prepare each stable original once in a streaming pass. Publish a checkpoint containing the source
watermark, committed payload end and projection generation only after their complete records are
written. Ignore uncommitted tails after interruption; rebuild from originals. Publish checkpoints
in coherent batches/segments rather than rewrite the entire index after each native event.
For live JSONL, process only newly
committed complete records; expose exactly the committed prefix and source generation. Readers
hold existing storage admission and verify the referenced generation/range. Earlier prefix
references remain attributable after append. Mutable, externally replaced inputs require fresh
admission/validation; do not hash or decode every unrelated full payload on each selected read.

Compact case metadata provides lookup, outcome/phase membership, timing and source locators;
payload access seeks selected ranges. Pagination carries a pinned projection revision, filters,
coverage, omitted counts and continuation. Report/index reconstruction may write derived output
under its owner, but inspection never edits originals. Missing/changed originals are explicit;
a surviving derived excerpt must not masquerade as an available original.

Expose stable case identity separately from display names: Rust package/binary kind/binary/test,
or pytest invocation/node ID, retaining native attempt and phase. JUnit call/teardown rows cannot
be deduplicated by name or counted as separate executions. Captured sections repeated across
pytest phase reports likewise do not prove repeated execution.

### 3.4 Commands and causal investigation

The following target commands are **not implemented**:

```text
just verify report RUN --json
just verify cases RUN --outcome failed --limit 50 --json
just verify evidence RUN --case CASE --phase call --limit-bytes N --json
just verify events RUN --step STEP --json
just verify compare RUN_A RUN_B --json
just verify rerun-plan RUN --case CASE --print --json
just verify export RUN --output PATH
```

Inspection returns schema/version, stable identities, evidence coverage and unavailable fields.
Inspection's `RUN` accepts a local run handle or an explicitly selected exported-bundle directory;
bundle loading creates no permanent import registry and never merges original runtime records.
Small overviews lead to complete paginated membership and selected original evidence. Excerpt
limits bound response size, not execution resources or producer inputs. Invalid/unsupported
requests fail clearly; a valid empty selection is distinguishable from incomplete coverage.

Group failures by transparent signatures or evidenced prerequisite identity, retaining every
member, exceptions and passing comparisons. Comparison reports observed differences and unknowns;
it does not emit a causal verdict. Preserve native trace export and rich replay as selectable
routes. CPU/resource profiling stays separately opt-in.

`rerun-plan` is read-only and launches nothing. Resolve exact selections through the current
verification catalog and actual prerequisite operations, preserving Cargo/runner profiles,
ignored/partition policy and fixture requirements. A serving consumer still needs its real
producer unless the existing owner independently admits retained content. Where dependencies are
currently implicit in step order, extend the existing step declarations with consumed/produced
requirements and resolve their actual closure; do not create another dependency registry or
equate minimality with running all preceding steps. Apply the selected case filter only to its
own execution, not prerequisite producers. Print argv arrays and
a safely quoted human command, selected obligations, context changes, readiness repairs and
remaining scope. Refuse unresolvable identity instead of broadening silently. Current-code
execution and recorded-output replay are distinct. Preserve existing `just verify --rerun RUN`
as a boundary rerun; a focused pass never certifies the original full scope.

## 4. Native capture and lifecycle

### 4.1 Nextest0.9.146

Qualify against pinned commit `8af696ddcce8fff2962d6a5168b6d138b8616a35`. Preserve the
[review's configuration analysis](../design_review/reviews/design_review_agent-test-evidence-and-causal-analysis_2026-10-11.md#4-library-fit-and-capture-configuration):
explicit managed recording, selected-profile JUnit including successful/failed output, skipped
reasons and flaky failure meaning. Recording `max-output-size="256MiB"` is the producer's
largest supported value, not a test memory reservation or a completeness guarantee. Native
head/tail truncation and JUnit UTF-8/XML normalization remain visible.

Build invocation-owned effective repository and user configuration from the discovered inputs,
changing only declared evidence settings/destinations. `--config-file` and `--user-config-file`
replace their respective discovery; preserve effective execution policy, profile inheritance,
explicit input paths and referenced settings. Tool-config lower precedence cannot guarantee an
override. Unsupported composition must report its limitation before launch rather than silently
change retries, scheduling, timeouts or threading. User recording overrides outrank base fields:
prepend a managed matching `[[overrides]]` record stanza (`platform="cfg(all())"`) while retaining
all unrelated override fields and order. Qualify field-specific first-match behavior against the
[pinned resolver](https://github.com/nextest-rs/nextest/blob/8af696ddcce8fff2962d6a5168b6d138b8616a35/nextest-runner/src/user_config/elements/record.rs#L231).
Keep the user's dirty repository file untouched.

Give each invocation a private `NEXTEST_STATE_DIR` base, independent of `[store].dir` and Cargo
target/build caches. This makes UUID binding unambiguous without isolated compilation artifacts.
Completed JUnit's native run ID and supported native store metadata must agree. For live/partial
runs, resolve the sole private-store candidate from `store list` then `store info` to its full
UUID, requiring exactly one invocation and later JUnit agreement when available. These installed
commands expose human output, so pin and qualify the small adapter; never assume a JSON interface
or private storage schema. Validate an exported recording through native artifact inspection.
When only partial channels survive, report exactly the established identities and coverage.

Native recording can prune during finalization, before an external export runs. Therefore private
state alone is insufficient. For the exactly-one-invocation private store, select the following
managed record fields, with equivalent values in the matching override:

```toml
[record]
enabled = true
max-records = 1
max-total-size = "18446744073709551615"
max-age = "18446744073709551615s"
max-output-size = "256MiB"
```

The size and age values are exact `u64::MAX` bytes/seconds, neutralizing native cache retirement
for this sole represented recording while leaving retirement to its existing owner. They allocate
no resources. These are **source-qualified, Proposed** settings: locked bytesize2.7.0 parses a
quoted decimal exactly; humantime2.3.0 accepts integer seconds and Nextest saturates the age
comparison. TE1 must verify parsing, effective overrides and finalization/export with small controls,
including a lowered-threshold demonstration of self-pruning and the selected protection.
[Pinned retention](https://github.com/nextest-rs/nextest/blob/8af696ddcce8fff2962d6a5168b6d138b8616a35/nextest-runner/src/record/retention.rs#L71),
[bytesize parser](https://docs.rs/crate/bytesize/2.7.0/source/src/parse.rs),
[humantime parser](https://docs.rs/crate/humantime/2.3.0/source/src/duration.rs).

Qualify the handoff for interrupted execution as well as completion.
Do not claim owner-backed retention until this handoff is demonstrated. Retain/export the native
recording under existing admission, validate readable identity and required replay capabilities,
and hold originals until publication and capture finalization resolve. Export failure or an
unresolved cleanup protects surviving inputs; test success does not attest export success.

Use uniquely owned JUnit as the completed selective projection and native recording/export for
rich replay. Libtest JSON-plus may supply an incremental partial channel where composition is
qualified, but does not replace missing setup/retry/native metadata or guarantee unfinished output.
Keep each channel's coverage independent. Native replay's inspection exit0 is not test success;
`--exit-code` can report106 for incomplete execution. Native rerun may report5 for outstanding
obligations even when selected work passes. Preserve those meanings and the parent UUID.

### 4.2 Pytest9.1.1 / Python3.14.7 and other steps

Add exact development dependency `pytest-reportlog==1.0.0` during implementation, through the
normal pinned-dependency route. Explicitly select the plugin and an uncompressed per-invocation
report-log destination. Preserve normal project plugin discovery, fd capture, warnings, useful
passing output and long traceback chains. Minimal hooks supplement test starts, collection
membership and deselections using native hooks/serialization; they do not implement another
general-purpose reporter. Qualify their composition and avoid duplicate plugin loading.

JSONL retains completed setup/call/teardown reports and collection errors; abrupt termination
can preserve a readable prefix while losing unfinished captured output. Do not promise fsync/
power-loss durability or xdist support from the existing probe. JUnit with complete logging
remains optional interchange, not a second execution authority.

For Cargo commands that actually support it, retain native `compiler-message`, `compiler-artifact`
and `build-finished` JSON separately from human stderr. Do not parse arbitrary console output as
machine diagnostics or add unsupported flags to other tools. Build/discovery/doctest/compile-fail/
lint failures remain first-class step observations even when no case report exists. Respect
explicit Rust backtrace overrides; record effective diagnostics. Native phase/error envelopes
retain primary/secondary failures, terminality and unknown remote effects through their owner.

### 4.3 Storage, exports and adoption

Use run ownership for new native captures, generated effective config, payload projections and
indexes; enroll independent physical components before production with a named consumer.
Use existing run-log lifetime for raw evidence/rebuildable data and run-receipt lifetime for
small manifests where appropriate. Do not nest independently disposable raw data so that a
longer-lived receipt descriptor silently retains it; qualify ancestor/descendant enrollment.
Add a category only if a genuinely distinct lifetime cannot use the current policy.

Reader admission protects open artifacts. Cross-session citations need explicit existing consumer
holds on the originals and necessary contextual material, not merely a returned path/hash.
Export enumerates selected raw artifacts, context coverage, indexes/manifests, digests and support
versions into an explicitly owned destination; qualifications include inspecting/rebuilding and
native replay after source-location removal. Exporting does not release/delete the originals or
invoke profile-only archive APIs with an invalid owner. Any needed portable replay/archive
extension belongs to the existing lifecycle owner and must establish real reader capabilities.

Existing runs remain inspectable at their actual legacy boundary/log coverage; missing fields
stay unavailable. No backfill asserts historical source identity. Managed new invocations use
the target capture path; bare tools remain first-class with their native guarantees. Replaced
managed fixed-path assumptions retire with consumer migration. No historical data or native
cache is deleted as part of adoption; no operator service/database is touched.

## 5. Execution packages and dependencies

All packages are **Open / not implemented**. One coordinator owns shared contracts, this plan,
integration and acceptance. Delegate bounded adapters/readers only after consumed contracts settle;
coordinate overlapping verification/storage files instead of inventing independent authorities.

| Package | Consumes | Delivered capability and completion boundary |
|---|---|---|
| TE0 — contracts and decision | Source review, current owners, §3/§6 | Executable invocation/capture/result/reference definitions and focused foundation controls. Use the ADR skill to record native-authority/derived-access alternatives with DESIGN §1.2; complement ADR-0142 without rewriting it. Settle producer support and retention qualification inputs before dependent implementation. |
| TE1 — owned capture foundations | TE0; run/storage admission and context mechanics | Invocation allocation, durable launch/step/post-step snapshots, lightweight context observations, supported config composition and native retention/export handoff. Preserve process/fixture authority; reveal lost-owner and interrupted-step behavior. |
| TE2 — producer adapters | TE1 identities/destinations; qualified versions | Nextest native/JUnit and pytest reportlog/hooks, supported Cargo diagnostics, native trace/replay routes and explicit unsupported/partial cases. Add the exact pytest dependency and repair route; verification never installs it. Each adapter can finish independently. |
| TE3 — preparation and inspection | TE2 supported artifacts; TE1 reader/storage contract | Streaming physical index/payload access, stable references/watermarks, report/cases/evidence/events/compare commands and transparent grouping. Rebuild/delete-index controls and selected-read access controls demonstrate the route. |
| TE4 — valid rerun planning | TE3 case identities; current command/dependency catalog | Printed case plans, semantically required production/readiness, source/context difference reporting, original outstanding obligations and unsupported-identity refusal. No automatic execution or environment repair. |
| TE5 — portable evidence and retention | TE1 lifecycle; TE2 replay; TE3 reference/index contracts | Export/import-for-inspection, exact artifact identity, owner/reader/citation holds, supported replay after relocation and explicit missing originals. Inspection import does not run tests or attach a database. |
| TE6 — integration and focused acceptance | Working TE1–TE5, migrated consumers | End-to-end tooling journey, adverse cases in §7, current command/help/Codex documentation, scoped non-functional checks and implementation review. Transfer enduring contracts to current owners; close findings only on their named evidence. |

TE1's identity/lifecycle slice unblocks adapters before all context-comparison presentation is
complete. TE3 can integrate each qualified adapter incrementally. TE4 and TE5 can proceed after
their shared inputs; their order is not a runtime barrier. Storage protection is required before
any producer writes, not a cleanup phase deferred to TE5. Durable observations and supported
capture must accompany consumer migration, not wait for TE6.

Useful pause points are owned capture with truthful coverage, then selective inspection, then
valid experiments/portable review. No intermediate milestone closes an enclosing finding until
its remaining producer, lifecycle and acceptance contributors finish.

## 6. Rule impacts and decision routes

The operator authorized this standalone plan on **2026-10-11**. The source review explicitly
describes RC01–RC04 as extensions/clarifications; this authorization is not represented as an
explicit acceptance of an unidentified rule replacement. Assessment below finds no required
repeal. Any newly discovered actual conflict follows create-plan's explicit confirmation route
before dependent target changes; ordinary additive tooling requires no blanket permission gate.

| Source impact | Disposition in this target | Route before dependent work |
|---|---|---|
| [RC01](../design_review/reviews/design_review_agent-test-evidence-and-causal-analysis_2026-10-11.md#RC01) | Extend evidence channels; full file logs remain authoritative for logs, native producers for their captured records, derived indexes for access only. | TE0 complementary ADR/owner prose; retain ADR-0142 supervision/display decision. |
| [RC02](../design_review/reviews/design_review_agent-test-evidence-and-causal-analysis_2026-10-11.md#RC02) | Add case-plan operation while preserving boundary `--rerun` and native argument grammar. | TE4/TE6 command, README and AGENTS guidance; no existing execution policy removed. |
| [RC03](../design_review/reviews/design_review_agent-test-evidence-and-causal-analysis_2026-10-11.md#RC03) | Managed capture supplies explicit settings and unique destinations; preserve operator execution policy and dirty source file. | TE1/TE2 effective generated configuration and policy-equivalence controls; later edit stale comments only within authorized consumer migration. |
| [RC04](../design_review/reviews/design_review_agent-test-evidence-and-causal-analysis_2026-10-11.md#RC04) | Apply existing admission/retention/consumer guarantees to new components. | TE1 enrollment/readers before writes, TE5 citations/exports; new category only for a qualified distinct lifetime. No cleanup automation activation. |

## 7. Verification, investigations and completion

### 7.1 Revealing acceptance scenarios

| Scenario | Required independent observation / packages |
|---|---|
| Setup, call and teardown failures; identical names across binaries; retries/flaky/skip/xfail/xpass | Adapter matches hand-authored native expectations without collapsing phase/attempt distinctions; TE2/TE3. |
| Collection/build/discovery/doc/compile-fail error with no completed cases | Step outcome and diagnostic identity survive independently of case count; TE1/TE2. |
| Interrupt during case, postprocessing, fixture restart or finalization; hard-stop verification owner | Durable completed-prefix/step evidence survives; outstanding work and cleanup remain unknown where appropriate; TE1/TE2. |
| Passing tests but recording/export/plugin failure; malformed or unsupported artifact | Native result and capture failure are separate, with exact requested/available coverage; TE1/TE2/TE3. |
| Concurrent runs/checkouts, same profile and case names | Unique destinations, native UUID agreement and independent cancellation; ordinary Cargo cache reuse preserved; TE1/TE2. |
| Native self-pruning, huge output, XML normalization and missing unfinished bytes | Retention handoff demonstrated; actual truncation/transformation/loss visible; no false completeness; TE1/TE2/TE5. |
| Same HEAD with changed tracked/untracked bytes, effective config drift, concurrent edits, reused binary | Recorded differences or explicit provenance uncertainty, never inferred exact build-source identity; TE1/TE3. |
| Many unrelated cases and large outputs, small selected request | Index once; access spy/seek controls show selected payload reads without repeated full decode/hash; full membership remains pageable; TE3. |
| Delete/rebuild projection; append native JSONL; replace/miss original; partial last line | Stable semantic citation and pinned prefix; new derived offsets do not change identity; losses exposed; TE3. |
| Exact Rust binary/test and pytest parametrized node, ignored/partition settings, serving consumer | Printed argv quotes safely and selects intended case plus actual prerequisites; no launch/repair, no silent scope broadening; TE4. |
| Citation outlives ordinary log expiry; active reader; unresolved export/cleanup; relocation | Existing holds prevent retirement; exported inspection/rebuild/replay remains valid or explicitly unsupported; TE1/TE5. |
| Optional diagnostic channel and existing bare/managed command paths | No injected concurrency/memory/healthy-duration cap, no plugin-discovery suppression, no automatic sync/service operation, unchanged boundary rerun; TE6. |

Use small purpose-built passing/failing/interruptible cases and independently specified malformed
fixtures. The failure fixtures are controls, not repository regressions. Do not merely round-trip
the production serializer or copy its expected classifications. Retain real native integration
for recording/config/export/phase behavior; unit substitutes cannot establish native capture.

During implementation, run focused affected controls through explicit tooling selections or bare
pytest as appropriate, with compile checks only for touched Rust. At TE6, run an assembled
**tooling** journey: launch small native cases, inspect while live and after interruption, retrieve
selected evidence, compare contexts, print valid reruns, export, then inspect after relocation.
Use managed fixture attachment only for the specific prerequisite-dependent control. Apply
affected Ruff/Pyrefly/dependency/docs leaves once at scope completion. No full product test suite
or `just qualify` follows from this tooling plan; preserve the existing receipt/trust contracts
through sidecars. If implementation changes a shared governing contract, reassess ADR-0126's
applicable assembled scope instead of quietly extending this acceptance claim.

### 7.2 Further investigations and explicit limits

| Item | Status / settling evidence / dependent work |
|---|---|
| Concrete locator/index route | Selected file-backed streaming route in §2; TE3 proves selective access, rebuildable identity and live-prefix handling before claiming F04 closure. |
| Producer-version and config composition | TE0/TE2 qualify exact support, effective policy and controlled unsupported cases; upgrades rerun affected adapter controls. |
| Native retention through finalization/export | Selected source-qualified private-store settings in §4.1; TE1 must verify parsing, effective overrides, interrupted retention and export before claiming durable native replay. |
| Source-to-binary provenance | TE1 records only established linkage; TE6 demonstrates known/unknown cases. Exact historical recomputation is outside scope without independently retained build inputs. |
| Starts, collection membership and deselection | TE2 selects minimal native hooks and composes them with reportlog; unfinished activity remains unknown until observed. |
| Additional unfinished-byte capture | **Deferred**. Trigger: a concrete investigation requires output absent from native completed reports; assess opt-in capture under existing supervision, with its own loss/overhead controls. |
| Native-reader replacement for duplicated text | **Deferred**. Trigger: observed duplication/lifecycle burden becomes material and a qualified supported reader reduces total machinery. |
| Perfetto SQL or another trace-analysis dependency | **Deferred**. Trigger: native exported trace and ordinary structured queries cannot answer a named temporal question. Qualify usefulness before adding tooling; no CPU attribution claim. |
| xdist, power-loss durability and measured agent effectiveness | Not claimed. Activate only for a real consumer/measurement question; current probe does not establish these properties. |

### 7.3 Evidence available now

**passed, 2026-10-11 (source review's dated evidence):** four synthetic pytest reporting scenarios
in run `20261011T052916.677Z-80b055`; the
[evidence README](../design_review/evidence/2026-10-11_agent-test-evidence/README.md) owns the
exact command and limits. Installed Nextest interfaces/pinned source were inspected. These are
baseline inputs, not execution of TE0–TE6 or qualification of new capture/configuration code.

**not_run:** implementation, native recording/export/interruption composition, new selective
commands, full product tests, whole-product acceptance and quantitative effectiveness measurement.
**passed, 2026-10-11 (plan authoring):** `just docs-check` published391 canonical pages with
zero link errors; `git diff --check` passed. Read-only independent design advice found no material
coverage/contract gaps in this draft. These checks establish document validity and scoped design
consistency, not implementation or native integration acceptance.

## 8. Sole current finding disposition and checkpoint

| Source finding | Disposition | Responsible capability / closure evidence |
|---|---|---|
| [F01](../design_review/reviews/design_review_agent-test-evidence-and-causal-analysis_2026-10-11.md#F01) | Open | TE0–TE3/TE6; independent case/phase/capture contracts, durable step/prefix evidence, native adverse/concurrency controls in §7. |
| [F02](../design_review/reviews/design_review_agent-test-evidence-and-causal-analysis_2026-10-11.md#F02) | Open | TE1/TE3/TE6; relevant content/config comparison, real binary attribution or explicit uncertainty, changed-context controls. |
| [F03](../design_review/reviews/design_review_agent-test-evidence-and-causal-analysis_2026-10-11.md#F03) | Open | TE4/TE6; exact-case argv, real dependency closure, preserved profiles/obligations, read-only planning and unsupported-case controls. |
| [F04](../design_review/reviews/design_review_agent-test-evidence-and-causal-analysis_2026-10-11.md#F04) | Open | TE1/TE3/TE5/TE6; selective access, original/citation identity, rebuild/prefix controls, protected readers and qualified relocated replay. |

Recommendations outside the four finding summaries are scheduled in §§3–7: capture defaults,
diagnostic channels, grouping/passing comparisons, timing/native replay, context limits, exports,
consumer documentation, alternatives and deferred investigations. Plan acceptance or an ADR
does not close a finding. No separate implementation ledger or sibling-plan dependency is added.

**Checkpoint, 2026-10-11:** plan authored; all implementation packages Open. Existing execution,
fixture and storage foundations are available, but the new capture/read contracts are Proposed.
Next: detailed execution planning for TE0 and the TE1 identity/lifecycle slice, including native
retention qualification. Preserve unrelated configuration and failure evidence; paused product
work retains its existing owners and verification boundary.
