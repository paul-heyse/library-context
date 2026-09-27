# Status

_Updated 2026-09-27 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and checkpoint

- **Increment 4 / Stage 3 remains functionally incomplete.** The [forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) owns S0–S8; §6 owns finding disposition. Work stops after this operator-requested checkpoint; no further feature slice is in progress.
- **Implemented and Tested: compiler101 scheduler foundation.** The value consumer uses shared frontier, component queue and alternative-progress helpers. Nondominated depth/expanded-proof-cost witnesses can reopen work; equal-cost witness alternatives do not. Successful alternatives survive later limited witnesses while unrelated refusals remain. Original depth, proof and pair-work caps remain fixed. All-channel composition remains open.
- **Implemented and Tested: compiler102 pilot repair.** Library/corpus extraction can observe the same pinned signature twice. Schema-owned agreement selects deterministic cited representatives for model formals, constructors, call evaluation and SQL argument binding, preserving raw facts and refusing conflicting payloads or duplicate fact IDs. The fresh real pilot now binds `json.dumps.obj` successfully.
- **Prior source/model work remains bounded:** independent Source/Model invocation, fresh header/binding and separate body/frame completion; explicit action triggers, default availability and undischarged Normal postconditions; narrow nullcontext/suppress lifecycle and entry-value identity; origin-specific coverage, transfer alternatives and typed RCA/Pass C. General release, resource/callback fates, model-family activation and full serving remain open in the plan.
- **Checkpoint repairs:** formatting and strict lints; stale validator injections, recursive-type inputs, generation inventory and import-root symlink expectations; nine inspected output snapshots and current output digests. The RCA freeze now records the already accepted ADR-0058 policy, with disclosure in ADR-0021; evaluation targets and thresholds did not change.
- **Versions:** compiler output **102**, extractor **33**, synthesis template **20**, catalog **7**, FORMAT **9**. No new Arrow columns in compiler101/102. Current native rebuilt; fresh store/generation follows ADR-0048.

## Last verified (2026-09-27)

Cargo/native builds use `CARGO_TARGET_DIR=/home/paul/library-context/target`; the shell inherited a sibling-project target. Test runs use `RUST_MIN_STACK=16777216`. Exact receipts and qualification limits are in [plan §1.1](docs/plans/behavioral-model-forward-plan_2026-09-24.md#11-operator-requested-checkpoint-2026-09-27).

| Command | Outcome and scope |
|---|---|
| `just fmt`; `uv sync --frozen --reinstall-package lctx-semantics` | **passed:** formatting and native102 rebuild (31.05 s). |
| `just test-all` | **passed:** 429/429 release Rust tests, rebuilt Python fixture, 148/148 Python tests, Pyrefly, strict lints, seven rule suites, ADR/agent checks, 83 fixture parses and dependency/fork/shear/gold policy. Snapshot updates disabled. |
| `just pilot build/store-stage3-checkpoint-2026-09-27` | **passed for real extraction/compile/publication and serving smoke with fake vectors:** snapshot `8b4fb9ab6dcaa3698c27018593553129`, generation `cdcf4b4e519e8b79`, 20 briefs; total 204.0 s, peak 4,258 MiB. Live embedding **not_run**. |
| Read-only `lctx query` on the pilot | **passed:** 16,825 behaviors, including 6,353 unknown `call_transfer`; raw duplicate signature observations retained. No Stage 3 exit or cost-improvement claim. |
| `just docs-check` | **passed:** 135 canonical pages and offline link/fragment checks; corrected one stale dated STATUS link. |
| Structured Q01/Q03/Q05/Q09 exit assessment, clean wheel, live W9/W16 replay/conformance, engine/cost comparison and assembled review | **not_run:** remaining functional scope and corresponding acceptance are still open. |

## Remaining Stage 3 scope on a later resumption

- **S1:** typed non-value subjects/payloads, phase-specific complete empty channels and shared evidence/obligation admission; parameterless effects without fabricated value IDs.
- **S2/S3a:** broader source/default/binding/frame-release domains, context entry/exit models, concrete resource identity/pairing and distinct callback fates. Named-handler cleanup has a specific unknown boundary; general cleanup completion remains unsupported.
- **S3b:** remaining pure/helper, I/O, async/context, validation/settings and HTTP/server targets in plan §3.3, with exact bindings/phases and independent challenges. Normal postconditions do not establish their outcome.
- **S4:** call-specific stable multiple controls and residual conjunction; all-channel SCC composition, applicability/obligation/refusal keys; current-worklist/Ascent/datafrog parity and measured costs.
- **S5:** claim-specific source/model/callee/handler/exit completeness and discharge. Positive witnesses do not close unrelated origins or channels.
- **S6:** native actions/defaults/postconditions/invocations, full raw-source evidence closure, compatible effect/role/exact-input queries and matched/excluded/source-open/unexamined partitions.
- **S7:** original W5/W7/W12 default-cap producer→Delta→native traces and independent composed CPython/CrossHair/Pysa challenges.
- **S8:** after S1–S7, assembled semantic exit assessment, clean-wheel/live checks, controlled cost measurements, review and final qualification. Current checkpoint checks do not close S8. Stage 4 FCA/registry and Stage 5 deferred execution remain outside this scope.

**Stopped at the requested checkpoint. Resume only on a new instruction.**
