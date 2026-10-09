# Recent Codex command evidence

This is bounded supporting evidence for the [principal review](../../reviews/design_review_agent-effectiveness-followup_2026-10-09.md), not a second assessment or disposition ledger. Extracted and context-read on 2026-10-09. Proposed improvements are not implementation or effectiveness receipts.

## Window, attribution and limits

The fixed window is **2026-10-06 00:00 EDT through 2026-10-09 14:45:09 EDT**, inclusive (`2026-10-06T04:00:00Z`–`2026-10-09T18:45:09Z`). Eligibility uses the recorded completion-event timestamp, not the session's creation date or its directory name. The October 5 root session below contains eligible October 6–7 work. October 9 is partial.

[`mine_codex.py`](mine_codex.py) reads `~/.codex/state_5.sqlite` with `mode=ro`, selects recently updated library-context/main/worktree/scratch threads and their descendants, and reads their original append-only JSONL files. The current review root and descendants are excluded. Commands in an attributed thread remain attributed even when their cwd is a shared skill or registry directory; this is repository-work attribution, not proof every command acted on repository code. Native `CommandExecution` completions are deduplicated by thread/item ID. Code-mode invocation counts and outer failure candidates remain separate; an outer invocation is not assumed to have executed or succeeded.

The helper reuses the [October 7 shell parser/classifier](../2026-10-07_agent-workspace-effectiveness/miner/extract.py), SHA-256 `e81b87f405d4328a75878f383f2ba5df3ad2dc7ba6b334c06639bfc753443e8f`. Classification is a search aid. Manual review found false positives: a pytest `AttributeError` and Ruff E501 were classified as environment failures because their printed source contained environment-related words; five apparent masked failures were successful readers printing failed logs. Neither class totals nor shell success establish an operation's acceptance.

The extraction encountered nine invalid JSON lines across candidate streams. Their events were not recovered or attributed to the window; missing/malformed events and unrecorded work prevent a completeness claim. No token, elapsed-time or productivity rate is inferred. No Claude corpus was assembled: Codex is the primary agent and this is the selected retrospective.

| Recorded observation | Count |
|---|---:|
| Populated attributed threads / inspected candidate files | 115 / 115 |
| Native command completions | 22,211 |
| Exit zero / nonzero | 20,484 / 1,727 |
| October 6 / 7 / 8 / partial 9 completions | 7,947 / 5,276 / 4,647 / 4,341 |
| Heuristic search/probe misses among all completions | 1,146 |
| Outer code-mode error candidates, not added to shell failures | 39 |
| Recorded Context7 resolve / query completions | 147 / 179 |

The heuristic categories include 372 code failures, 130 other failures, 54 interruptions, 22 environment candidates and three usage candidates. These are **not adjudicated failure counts**. Invocation-construction mistakes also appear outside the usage category, as U2 demonstrates. Counts establish useful places to investigate, not agent effectiveness or a justification for resource caps. Root and executor work account for 10,319 and 8,071 completions; delegated roles are retained in [metrics.json](metrics.json). Role counts do not rank agents.

Recorded CLI versions were 0.160.0, 0.160.1 and 0.161.0; the installed shell CLI now reports 0.162.0. Thread metadata reports `codex-tui`, which alone does not establish parity with every desktop/IDE runtime or its applied settings. Configured, installed, exposed and exercised capabilities are distinguished in the [Codex/Rust research](capabilities-codex-rust.md).

Raw commands, output samples, call pairs and the source index stay in ignored `build/agent-effectiveness-followup/2026-10-09/`. Original transcripts remain authoritative. The extractor stores nonzero output head/tail samples, not every full output, and leaves successful command output empty. Selected cases below were checked against the original context. Published excerpts omit private configuration and credentials; raw transcripts are not committed.

**passed — extraction:** `uv run --no-sync python docs/design_review/evidence/2026-10-09_agent-effectiveness-followup/mine_codex.py`. It reads logs/metadata and writes ignored output; it executes none of the mined commands. The frozen aggregate is copied into this folder. The helper is a review reproduction aid, not a new standing command or gate.

## Trace pointers

Line numbers below are physical JSONL lines, not the event's separate `ordinal` field. Item/call IDs and UTC timestamps make the observations recoverable if line layout changes.

- **A:** `/home/paul/.codex/sessions/2026/10/07/rollout-2026-10-07T19-05-14-01a1189d-2716-77d0-9b07-7806d51ad675.jsonl`.
- **B:** `/home/paul/.codex/sessions/2026/10/05/rollout-2026-10-05T14-26-51-01a10d51-8e55-7e23-91f8-31ca287b7fab.jsonl`.
- **C:** `/home/paul/.codex/sessions/2026/10/08/rollout-2026-10-08T23-16-24-01a11ea9-765b-75f0-8b67-9b8d0a16f84c.jsonl`.

### U1 — option borrowed from a neighboring subcommand

At A:10772, `2026-10-08T20:47:25.336Z`, item `exec-f5fe78ea-2b95-48cf-a9b0-928ab6009dc9`:

```sh
just runs retain 20261008T202337.170Z-114602 --json
```

**failed, exit 2:** `unrecognized arguments: --json`. The following invocation at A:10780, `20:47:29.645Z`, item `exec-8827291b-89f2-4175-b2c4-0ea59b2bc946`, removed `--json`:

```sh
just runs retain 20261008T202337.170Z-114602
```

**passed, exit 0, retention command only.** No underlying run pass is inferred. This supports discoverable per-operation option/output contracts; it does not establish that every operation needs JSON or that a new CLI is necessary.

### U2 — package selection and a global Cargo flag

At A:20231, `2026-10-09T05:26:21.327Z`, item `exec-bcbf0fff-67b5-4eae-9ceb-907a686f01b7`:

```sh
cargo check --release --locked -p lctx --tests -p lctx-serving --tests -p lctx-publisher --tests
```

**failed, exit 1:** `the argument '--tests' cannot be used multiple times`. At A:20245, `05:26:46.977Z`, item `exec-1496c4d6-771e-411a-a791-3b80023f7087`, a composite edit/check command ended with:

```sh
cargo check --release --locked -p lctx -p lctx-serving -p lctx-publisher --tests
```

**passed, exit 0 for that composite.** The grammar was corrected, but code also changed between the two calls. This is not a same-code before/after acceptance comparison. A native command example and target discovery are sufficient candidates; a Cargo wrapper needs an additional consumer justification.

### U3 — background ownership belongs to the launcher

At A:21353, `2026-10-09T06:25:32.033Z`, item `exec-50f037d1-653c-498b-a1b0-6104ee20d4a2`:

```sh
env XDG_RUNTIME_DIR=/run/user/1000 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus just verify --select leaf:deps --background
```

**failed, exit 2:** `verify.py: error: unrecognized arguments: --background`. At A:21361, `06:25:39.020Z`, item `exec-8f9e3c10-fe1c-4523-953c-74dff425a4c4`:

```sh
env XDG_RUNTIME_DIR=/run/user/1000 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus just run --background --label populated-final-deps -- just verify --select leaf:deps
```

**passed, exit 0 for launch.** This says the background launch was accepted; it does not establish the leaf's eventual result. The existing composition preserves one lifecycle owner. Better help/recovery can teach it without copying background management into verification. The explicit workstation D-Bus values show historical invocation context, not a proposal to require those values globally.

### E1 — wrong Python followed by an independent query error

At B:47365, `2026-10-07T09:02:50.879Z`, item `exec-bd7b621d-0e66-4412-936f-7ba43b8e6f34`, an inline `python3 - <<'PY'` script imported `scripts/surrealdb_fixture.py`.

**failed, exit 1:** `SyntaxError: multiple exception types must be parenthesized`. This was the system Python 3.12 parsing the project's Python 3.14 syntax. At B:47374, `09:03:06.041Z`, item `exec-3e6af8d2-d61d-4c67-827d-5308ba09a631`, the fixture-query script used `uv run --no-sync python - <<'PY'`.

**failed, exit 1, different boundary:** import/parsing succeeded, but the SQL HTTP request returned `HTTP Error 400: Bad Request`. The interpreter correction is visible; the operation still failed. B:47388's later source read and HTTP-body diagnostic exited zero, which establishes diagnostic execution rather than query acceptance. Exact inline bodies are retained at the pointers; no private fixture values are published.

The current pinned Python and documented `uv run --no-sync` route address the first failure. Rewriting all scripts for system Python would add a second compatibility obligation without a shown consumer. The query error motivates preserving structured per-statement/error context, but this trace alone does not prove a specific SDK, SQL grammar or fixture defect.

### T1 — formatted output consumed as authoritative JSON

At A:6171–6174, `2026-10-08T11:58:20.918Z`, call `call_58d0a5ca3ff946fba14b30680725c6ad`, code-mode ran Python to print five complete documents and dirty-file hashes as JSON, requested `max_output_tokens: 40000`, then called `JSON.parse(r.output)`.

The shell item `exec-89c428e8-6829-45e2-84df-608e20527159` **passed, exit 0**. The outer script **failed:** `SyntaxError: Unexpected token 'W', "Warning: t"... is not valid JSON`. Its tool-visible output had a truncation warning; successful producer output was not an intact structured channel.

At A:6178–6185, call `call_d1768afa558841e780c4710b0d8ceb2e`, the corrected Python wrote the complete structure to `/tmp/library-context-compile-plan-baseline_2026-10-08.json` and printed a compact receipt. Item `exec-1e19d651-0fbe-498a-adab-a67d3b827948` **passed, exit 0**, reporting five document baselines and 106 dirty-file hashes; the outer script completed. This establishes capture execution, not later use or preservation of every concurrent file.

The lesson is to consume actual structured results/files and use displayed output for feedback. It supports neither a global lower output limit nor a ban on batch reads/code-mode orchestration.

### T2 — outer JavaScript syntax rejected before native execution

At C:137–139, `2026-10-09T03:17:12.196Z`, call `call_a3f64b0ecf1b427e8a03731caae2e230`, the code-mode input began `const r = await tools.exec_command({cmd: ...}` without the closing `)`.

**failed at the outer invocation:** `SyntaxError: missing ) after argument list`. No shell execution is established for that malformed input. C:143–146 supplied `...}); text(r.output)` and the script completed with the requested source output. This is a concrete construction failure, separate from native exit codes. Additional same-worker syntax mistakes are leads, not an adjudicated rate. A short valid composition pattern can help; an extra command service is not justified by this trace alone.

### T3 — unavailable live process handle

At B:50248–50250, `2026-10-07T11:18:03.505Z`, call `call_TwMSOHgdOp2edgK3y2qXzayk`:

```javascript
text(await tools.write_stdin({session_id:92725,chars:"",yield_time_ms:1000,max_output_tokens:1000}));
```

**failed:** `write_stdin failed: Unknown process id 92725`. Earlier B:50136 returned that handle; B:50229 still returned it while reporting three completed compiler-view tests and two native tests starting. The surrounding excerpts do not establish why the handle became unavailable, nor a replacement successful continuation. This is an **unresolved lifecycle observation**, not proof that workload cancellation, wrong handle type or a repository runner bug caused it. Session process IDs, code-mode cell IDs and durable repository run IDs have distinct consumers; cross-session resumption should use the existing durable route when needed.

## Historical changes and counterexamples

At B:15426 on October 6, worktree removal failed with `PermissionError` on a root-owned SurrealDB `.sst`; B:15453's Docker cleanup attempt failed because the image contained no `/bin/sh`, and removal still failed. That container fixture mechanism is retired in the current native fixture design. A missing `surreal` executable on October 7 also predates current installation. These are real historical failures, but neither is evidence a current native fixture still has that fault. Existing October 7–8 implementation receipts must retain their original scope; source fixes are not a fresh live acceptance pass.

Current test assertion failures, Rust type errors, SQL/schema refusals, 300s/900s workload timeouts, authentication failures and operator/runtime cancellations remain separate from commands that could not start or parse. A failing test can be useful feedback. The native-execution plan and coordinator own their product diagnosis/qualification; this review does not relabel them as agent friction or claim model/configuration changes would cure them.

The inspected current runner still has source-derived exception/backpressure questions. None of the selected traces establishes those scenarios occurred locally. If the principal reviewer retains such a finding, its evidence must say source inspection or a separately reported focused probe, rather than attributing the corpus example's failures to this repository.
