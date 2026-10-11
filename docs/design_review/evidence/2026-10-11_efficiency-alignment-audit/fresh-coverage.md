# Supporting inquiry: independent additional coverage

The supporting reviewer formed FC01–FC04 **before reading the original review, its handoff
or its inquiry conclusions**. It then compared those candidates on the coordinator's explicit
instruction. The independent pass boundary was recorded in the agent messages before exposure.
This report records both passes. Root integration: 2026-10-11.

Baseline `2b05c6c239fb6569472751faf126f5e39f822853`; relevant files have no source diff
from original `88f33bc0`. Frozen overlays do not alter the findings. Diagnoses are
**Implemented/static, 2026-10-11**; remedies are **Proposed**. Builds/tests/fixtures/services:
**not_run**. No protected evaluation populations were opened. The reviewer changed no files.

Consumed by the [principal audit](../../reviews/design_review_efficiency-alignment-audit_2026-10-11.md).

## Additional findings

The principal retains FC01–FC04 as F14–F17, respectively.

| Candidate | Evidence and comparison | Priority and corrective contract |
|---|---|---|
| FC01 — Summary discovery repeatedly searches unrelated state | `crates/lctx-model/src/domain/execution/summary_production.rs:1185–1234,1310–1334,1942–1968` nests support/Entry searches, scans all calls per proof and scans seeds/branches per SCC. Many independent singleton SCCs still pay unrelated work. Original O-F31 owns condition lowering and O-F32 Structural delegation; neither covers Summary discovery. | **Medium**, behavioral profile. Prepare seeds/branches by owner, calls by caller/callee and Entry/support adjacency once per immutable invocation. Follow matching relations and genuine support alternatives. Preserve deterministic SCC order, semantic frontier, proof alternatives and explicit residuals. Closure: sparse independent-callable and alias cases retain canonical results while unrelated components cease multiplying discovery work. |
| FC02 — Automatic synthesis repeats joins across public candidates | `domain/synthesis/automatic.rs:199–267` scans member frames/ranks/community memberships per candidate; `result()` also scans analytic inventories. Production caller `crates/cpg-core/src/synthesis.rs:934–945`. Distinct from O-F32/O-F33 and remains after compact literal-summary preparation. | **Medium**, catalog synthesis growth. Prepare member/frame, analytic-result, rank and community indexes once per admitted-input lifetime. Preserve the entire decision population, duplicate-slot ambiguity rejection, configured selections, deterministic ranking and community/budget outcomes. Do not stop after filling brief budget: decision records have consumers (`automatic.rs:340–355`). Closure: complete decision/output equivalence including absent analytics and ties; source shows keyed lookup rather than candidate×relation scans. |
| FC03 — Ruff binding correspondence scans canonical events per binding | `crates/cpg-extract/src/ruff_lexical.rs:201,228–249,281` repeatedly searches events/scopes and allocates hit vectors. Site/name are available selective keys. O-F26 covers parser/checker traversals; O-F28 artifact-use lookup. Downstream normalization already uses maps (`domain/normalized/native_lexical.rs:100–145`). | **Low–Medium**, large generated/configuration modules. Build source-event buckets by site with exact-name disambiguation and canonical scope/observation sets. Preserve unique-site fallback, ambiguous-name and unlocated outcomes. Closure: identical correspondence for multi-name sites and unsupported locations; remove full-event scans per binding. |
| FC04 — Profile status repeatedly decodes unchanged completed samples | `scripts/compile_profile.py:1222–1253` calls `sampled_report(recent, readonly=True)` per unit. On miss, 1171–1202 hashes the chunk/tool, runs `perf report`, then returns before writing the cached receipt. Original O-F48 concerns executable identity, not this repeated decoder work. | **Low–Medium**, opt-in diagnostics. Ordinary status should use bounded metadata; request hotspot decoding explicitly or reuse results under the existing report owner. Preserve completed-versus-active capture, exact reader/input identity and unknown labels. Closure: repeated status over unchanged chunks does not repeat decoder execution; changed capture/reader invalidates. No latency claim. |

Keep separate findings: sharing a “prepare once” remedy does not give Summary, automatic
synthesis, Ruff correspondence and diagnostic status the same owner or closure contract.
The original handoff explicitly identified these areas as outside its kernel/tooling breadth.

## Smaller opportunities and non-findings

- **Freshness source copying is additional, but an opportunity.**
  `scripts/freshness.py:102–164,173–195` captures all Rust body bytes, copies them to a
  temporary tree and recaptures for stability during Hakari metadata checks. A metadata-shaped
  snapshot might suffice; first qualify Cargo/Hakari automatic/explicit target discovery,
  manifests/configuration, relevant file existence and concurrent-change detection.
- **Cold-backup final hashing is an additional narrow opportunity.**
  `scripts/surrealdb_service.py:1255–1318` keeps the service stopped through completed archive
  hashing. Hashing while writing could remove that pass without weakening cold coherence.
  No measured downtime or operational target supports higher priority.
- **Per-call HTTP client already covered.** `python/lctx_mcp/src/lctx_mcp/embedder.py:173–185`
  constructs/closes an HTTPX2 client per request; original §7.10 explicitly classifies this as
  small. Retain that position. A lifespan-owned pool with close is supported by inspected
  installed HTTPX2 2.13.1 and upstream guidance; no inference that connection setup dominates
  local inference. Embedding/native admission and drain remain O-F39, not a new finding.

## Independently inspected strengths

- `execution/summary_schedule.rs:35–62` delegates SCC discovery to petgraph and establishes
  deterministic callee-first ordering. `summary_worklist.rs:73–145` separates semantic progress
  from proof identity/cost and retains Pareto representatives.
- `cpg-core/src/synthesis.rs:803–839,947–955` streams documentary grains, retains compact
  identities and hydrates selected members later. This is useful operation-shaped preparation.
- `scripts/programmatic_eval.py:38–111` retains one subprocess across a campaign and shrinks,
  batching 32 judgments. Public capture uses a shared journey deadline and exact SDK bytes
  (`211–243,295–357`). This does not certify CI-G3 or expose heldout data.
- `lctx_mcp/native.py:49–64,106–114` retains admission until worker completion and drains
  before session close; a cancellation rewrite must preserve ownership of running native threads.
- `third_party/surrealdb/src/engine/remote/grpc.rs:1818–1839` stops export pulls when the receiver
  disappears; compression is scoped to bulk uploads (`510–518`).
- Installed custom vLLM source matches `services/vllm/pyproject.toml`; scheduler token/input
  budgets and pooling admission (`v1/core/sched/scheduler.py:580–588,1113–1123,1354–1358`)
  and encode cancellation (`v1/engine/async_llm.py:1018–1048`) provide a plausible route for
  the declared request/batch settings. No deployed performance, fairness or cancellation test.
- `scripts/storage_archive.py:107–179,272–308` checks streamed membership/content before
  exclusive publication, preserves source and requires requested reader capabilities. Replay
  is an independent recovery guarantee, not inherently redundant verification.
- `scripts/surrealdb_server.py:437–442` reuses an exact completed server descriptor before
  acquisition/build. Cold coherent backup is justified.
- `scripts/worktree.py:403–555,569–583` consults real holders, dependencies, runs and consumers;
  it does not infer disposal authority from age or size.

## Remaining breadth

This pass extends coverage, not product-wide gate certification. Complete transfer/consequence
kernels, `lctx-eval` internals, lexical statistics/search lowering, full Python middleware/resource
flows, server-side gRPC scheduling, complete service upgrade/recovery bodies and every development
tool interaction were not exhaustively reviewed. Installed vLLM was sampled at the declared exact
custom version; runtime throughput/memory/fairness remained unmeasured. Real-library volumes,
timings, peak live state and algorithmic thresholds were not measured.
