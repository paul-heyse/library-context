# Status

_Updated 2026-10-06 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Evidence/retrieval and primary evaluation plans authored, 2026-10-06; implementation Proposed.**
The [combined coordinator](docs/plans/evidence-retrieval-and-evaluation-plan_2026-10-06.md) owns
shared ER/EV decisions, dependencies and the nominated [target review](docs/design_review/reviews/design_review_evidence-retrieval-and-programmatic-evaluation_2026-10-06.md)'s
F01–F03. The [programmatic evaluation plan](docs/plans/programmatic-evaluation-plan_2026-10-06.md)
owns the independent primary loop; grounded outer agentic feedback improves both system and evaluator.
The four graph-native supporting plans develop production ER1–ER4. The forward product sequence,
architecture, profile1.5 and ADR-0130/0131 incorporate approved RC01–RC05. Existing gold,
confirmation and heldout stay protected. Full4096 winners with shared normalized1024 search/E1
projection and all affected consumer migrations are targets, not implemented changes.

The [independent revised-plan review](docs/design_review/reviews/design_review_evidence-retrieval-and-evaluation-plan_2026-10-06.md)
accepts the target at **Proposed** architectural strength, with no new material finding or rule
impact. Its dependency-policy follow-up preserves that verdict; the new plans follow the operator's
2026-10-06 exact-version instructions without changing dependencies. This does not establish runtime acceptance. Source F01–F03 remain open at
[coordinator §6](docs/plans/evidence-retrieval-and-evaluation-plan_2026-10-06.md#6-finding-disposition-investigation-outcomes-and-completion);
decided design does not establish implemented closure. **passed**, 2026-10-06:
`UV_NO_SYNC=1 just adr-lint` (71 records) and `UV_NO_SYNC=1 just docs-check` (324 canonical pages,
offline links, ADR and agent instructions), after repairing one stale retired-ADR link.
Product tests, probes, inference, agent/evaluation campaigns and performance measurement are
**not_run** for this document-authoring scope. The supplied external proposal remains unchanged
and untracked. No extra worktree or owned runtime worker was created.

**Next:** plan execution for ER1 construction and EV1 independent finite kernels from the settled
semantic design; ER2 shared-value/consumer migration follows actual ER1 inputs. Native and live
lanes consume their actual repaired prerequisites; external comparison parity cannot block pure EV1.
No production work or paused native acceptance is activated by this documentation handoff.

**Graph-native remediation remains paused at the user's request; it is integrated, with final native acceptance pending.**
The authorized scope is [coordinator §9](docs/plans/graph-native-pivot-plan_2026-10-05.md#9-remediation-and-improvement-execution)
and its four supporting plans. They retain the hard design pivot and targeted functional verification.
The original graph-native target/capabilities reviews remain the architectural basis; the audit supplies defects.
Native querying is selected. Design efficiency follows first principles and library capabilities,
without cost accounting, proof machinery or quantitative performance claims.

All agent patches are integrated on main. The operator's pushed `ad68af70` is preserved.
Follow-ups are `1eac2cde` (upper Place endpoint admission), `02b18ab6` (formatted source)
and `e5df7449` (native logical-text schema correction). No merge remains unresolved.
All fourteen extra worktrees were reviewed and removed after confirming integration; only main remains.
Owned native fixtures and credentials are cleaned up. No owned build/test/worker remains running.
Unrelated processes, containers and shared Cargo caches are preserved.

[Coordinator §7](docs/plans/graph-native-pivot-plan_2026-10-05.md#7-sole-finding-disposition-and-optional-capabilities)
remains the sole graph-native remediation finding disposition owner. Its F01–F12 remain open pending complete integrated closure;
a landed slice or focused pass does not close the entire remediation.
[Coordinator §8.1](docs/plans/graph-native-pivot-plan_2026-10-05.md#81-remediation-checkpoint--in-progress-2026-10-06)
records source, exact outcomes, stopped builds and resume order. All four supporting plans carry the pause.

The integrated compiler separates necessary semantic admission from diagnostic producer replay,
retains complete neutral typed import, uses bulk/ordered normalization and scoped upper consumers,
and shares normalization-owned immutable receiver/event/binding and compact projection authority.
Canonical transport now retains the necessary source/member/service/frontier companions.
SourceCalls/Enriched retain complete admitted callee bodies while excluding unrelated rich syntax.
Fresh Normalized/upper lowering includes exact intrinsic Place endpoints; Facts does not publish
undeclared normalized records. Detached import remains strict and never repairs missing inputs.

Native changes include streamed search/witness construction and cold reconciliation, terminally
checked gRPC backup, atomic selection, exact batched cache winners, complete executable source
capture, scoped browse vocabulary/counts, parent-owned nested cursors and typed safe Rust/PyO3/MCP
failures. Current architecture, agent/assurance/task/runbook owners describe the native system.
Obsolete assurance plans are removed with current obligations transferred; independent product/Q1
work remains at its existing owner. No compatibility store/reader or legacy runtime is retained.

**Verification checkpoint, 2026-10-06:**

- **passed:** scoped model/compiler controls and their affected repairs, including all necessary transport/frontier companions, actual selected-service consumption, structural/body scope, positive SourceCalls/Enriched equality and detached omission refusal. Exact commands/logs and bounded earlier receipts are at coordinator §8.1.
- **passed:** paired canonical normalization of 60,000 occurrences under 2 MiB/one partition and 16 MiB/two partitions; `/tmp/lctx-r-final-normalization-paired.log`. This is canonical-output evidence, not RSS/throughput measurement.
- **passed:** model missing/foreign Place alias refusal; actual detached Behavioral Analysis/Catalog (190.28s/226.92s) and guarded Facts (18.86s); `/tmp/lctx-r-final-place-endpoint2.log`, `/tmp/lctx-r-final-compiler-fast-controls{10,11}.log`. Earlier aggregate attempts failed, with repaired affected reruns recorded separately.
- **passed:** scoped eight-crate all-target release Clippy, Python types, Ruff and dependency checks on formatted `02b18ab6`; `/tmp/lctx-r-final-source-{clippy,types,ruff,deps}.log`.
- **passed:** direct Cargo cdylib build and editable-package import of `NativeFailure`/`NativeSession` on `02b18ab6`; `/tmp/lctx-r-dev-native-{build,import}.log`. This extension predates the final native schema correction and must be rebuilt before resumed served acceptance. Wheel packaging is waived during development.
- **passed:** five actual native serving/identity controls before that final correction; `/tmp/lctx-r-final-native-serving.log`, run `fd6b863e-c7a6-411b-a8ba-2e460a8f490f`.
- **failed:** both Catalog/native MCP prerequisite journeys at publication because the closed schema omitted binary-backed logical text emitted by the codec; `/tmp/lctx-r-final-native-mcp-fast.log`. `e5df7449` aligns schema generation with textual metadata, preserving opaque bytes and strict shapes. Its affected compile check, actual persistent text/opaque-byte/unknown-field-refusal control and SDK all-target Clippy **passed**; `/tmp/lctx-r-pause-native-text-{check,runtime,clippy}.log`. The two journeys and Python MCP controls have not been rerun after repair.
- **not_run / stopped before runtime:** final store/CLI/cache/backup/search/reconciliation aggregate. The command-target-only O0 CLI build passed in 55.31s; the subsequent selected test build was stopped before runtime for the repair/pause. Earlier optimized CLI/MCP builds were stopped before runtime; target-only O0 replacements preserve cached release libraries, budgets and expectations. Initial system Python 3.12 launcher failures ran no controls; corrected commands use project Python 3.14.7 through `uv run --no-sync python`.

**Resume:** rebuild the Cargo development extension and create fresh owned persistent RocksDB
fixtures; finish selected store/CLI/backup/cache/search/reconciliation and affected serving controls,
then actual Catalog/native journeys and in-process/stdio safe-error/resource/cancellation/drain routes.
Finish remaining applicable leaves and F01–F12 dispositions before recording R-Q0 completion.
Do not revive an old realization or restart the stopped legacy suite to save setup.

Broad `just qualify`, legacy CLI snapshot/parity, sealed evaluation, real-library/live-Qwen/operator
activation and quantitative performance measurement remain **not_run** under this scope.
The original user-accepted compiler-stage stopped-test boundary remains intact. Q1 adoption is separate.
Root closeout documentation leaves and `UV_NO_SYNC=1 just turn-end` are recorded at coordinator §8.1.
