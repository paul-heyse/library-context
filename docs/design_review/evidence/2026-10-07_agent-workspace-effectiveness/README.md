# Agent workspace effectiveness (2026-10-07)

**Question.** Which changes to the environment, command surface, tooling and agent configuration would make highly capable agents more effective in this repository? Effective here means:
- less up-front thinking to form a command;
- fewer complications caused by the environment;
- parameterised related actions;
- legible feedback.

Claude Code and Codex are weighted equally.

**Consumer.** The [agent workspace effectiveness plan](../../../plans/agent-workspace-effectiveness-plan_2026-10-07.md), its findings AE-nn and their dispositions.

**Baseline.** The 2026-10-07 checkout at HEAD `1be0dd96`. Another agent's uncommitted cpg-core edits were left untouched.

| Tool | Version |
|---|---|
| Claude Code | 2.1.293 |
| codex-cli | 0.161.0 |
| just | 1.58.0 |
| cargo-nextest | 0.9.146 |
| uv | 0.12.22 |
| rustc | nightly 2026-09-28 (toolchain nightly-2026-09-29) |
| Docker | 29.8.2 |
| systemd | 255 |

Machine: one workstation with 32 CPUs, 188 GiB RAM and 8 GiB swap. Swap was nearly full during the assessment.

## Method

| Workstream | What it examined | Where its raw output lives |
|---|---|---|
| W1 transcript mining | Claude and Codex sessions in this checkout, from 2026-09-22 to the 2026-10-07T22:42Z cutoff. Stratified by runtime × main/subagent × role × configuration period. | `build/agent-effectiveness/w1/` (gitignored; contains command text) |
| W2 command surface | `just --dump`, `scripts/verify.py` families and boundaries, `cargo metadata` test targets, `pytest --collect-only` | `build/agent-effectiveness/w2/` |
| W3 instruction and configuration load | Measured first-request context per runtime, instruction bytes, duplication and staleness | `build/agent-effectiveness/w3/` |
| W4 capability research | uv, maturin, nextest, Docker, Codex and Claude Code at the installed versions (Context7, tagged source, `--help`) | `build/agent-effectiveness/w4/` |
| W5 probes (2026-10-07) | Four checks: `uv sync --project … --frozen --dry-run` and `--check --inexact`, each with `LCTX_NATIVE_SEMANTICS_INPUTS` unset and then set (read-only); `/usr/bin/python3` (3.12.3) `ast.parse` over `scripts/*.py`; first-request context of a fresh `claude -p --max-turns 1` (full, then with `--setting-sources project,local --strict-mcp-config`) and a fresh `codex exec`; Docker `-p 127.0.0.1::8000` port before and after `docker restart` on a throwaway labelled container (33192 → 33193, then removed) | `build/agent-effectiveness/w3/`, `w4/` |
| W6 cold-start drills | Six read-only tasks, each run once by `claude -p --permission-mode plan` and `codex exec -s read-only`, scored against targets written beforehand | `build/agent-effectiveness/w6/` |
| W7 isolation | Kernel/oomd journal, cgroup placement, swap | `build/agent-effectiveness/w7/` |

**Configuration periods (UTC)**, taken from the git history of the harness files:

| Period | Ends | Boundary |
|---|---|---|
| P0 | 2026-09-29T02:49Z | ADR-0079, shared nightly builds |
| P1 | 2026-09-30T21:15Z | end-of-turn hooks |
| P2 | 2026-10-01T16:18Z | ADR-0110 |
| P3 | 2026-10-05T05:08Z | `verify.py`, ADR-0126; hooks removed at 08:09Z |
| P4 | 2026-10-06T02:41Z | Docker fixtures, `native_controls.py` |
| P5 | — | the current surface |

**Privacy.** No transcript text, command arguments or human messages are committed. `metrics.json` holds aggregate counts, durations and token figures only.

**Rebuild.** `miner/` is the parameterised transcript miner. It was ported from the pse-arrow 2026-10-07 assessment, uses the standard library only and runs under Python 3.14. Its header explains how to run it. Raw outputs regenerate under `build/agent-effectiveness/`.

## Results

The findings, their evidence labels and the proposed packets are in the consuming plan. This folder keeps only what is needed to rerun the measurement for a before/after comparison.
