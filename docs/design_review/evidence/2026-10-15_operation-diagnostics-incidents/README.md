# Operation-diagnostics incident retrospective

**Question.** For five retained 2026-10-15 Catalog-completion incidents, which diagnostic questions
(Q1–Q18 of the operation-diagnostics review's workload premise) could retained evidence answer, at
what agent effort, and which missing signal would have answered them? This is input to the
[operation-diagnostics target review](../../reviews/design_review_operation-diagnostics-target_2026-10-15.md)
(assignment C, read-only `code-mapper`). It is evidence, not a decision. It does not diagnose product
root causes beyond answerability.

**Method.** Read-only inspection of retained managed-run directories, attachment records, native-drain
receipts, verification projection files and `journalctl --user` output for the owned campaign units.
[`phase_pairs.py`](phase_pairs.py) pairs `lctx_phase` begin and terminal lines by `phase_id` and
reports gaps and open phases from a raw log. Run it as
`uv run --no-sync python -I docs/design_review/evidence/2026-10-15_operation-diagnostics-incidents/phase_pairs.py LOG [START [END|0 [GAP_SECONDS]]]`.
It reads retained logs only.

Read-only. `just verify report/cases/events/evidence` were **not_run**: they take storage admission
locks under `~/.local/state/library-context-storage` (`scripts/storage_lifecycle.py:289-330`) and can
republish the projection checkpoint, so the mapper read the projection files and raw logs directly.
Owned campaign DBs are disposed, so every DB-side answer comes from schema and code, not data.
A scratch phase-pairing parser, [`phase_pairs.py`](phase_pairs.py), was used.

**Bottom line.** Nothing answers the live questions (Q1–Q5): no live signal reaches disk.
Post-run, nearly every question needs hand-parsing raw text, and often manual timestamp alignment
across three or more sources with mixed time zones: UTC "Z", "+00:00", junit and journald at -04:00,
and file mtimes in local time.

Effort codes: **L** = one structured reader or file; **M** = several files or raw grep; **H** = a
custom parser and/or manual alignment across ≥3 sources.

Incidents: I1 = `12d184` composition, I2 = `e3ce73`/`bd6eb5` finite pass/replay, I3 = `8a3347`
captured recursive, I4a = `65d5fb` cold timeout, I4b = `7a553d` cold pass, I5 = `65a69e` runner.

| Q | I1 | I2 | I3 | I4a | I4b | I5 |
|---|---|---|---|---|---|---|
| Q1 progressing (live) | no: nextest holds test output until 900 s (G1); drain receipts only "started" | no | no (G1); control tables could show effect counts by polling (code only) | no; retirement counters `examined`/`retired` pollable (code only) | no | partial: `progress.json` names the current command |
| Q2 active stage, ordinal/total | live no; after: yes, H; ordinal/total no | live no; after: partial (middle lost) | live no; after: yes, H; no ordinal | live no; after: partial (last 20 s outside any phase) | after: yes, M (junit only) | partial: step name, no position |
| Q3 waiting on | **no**: `evaluate_base` silent 492 s (G3) | unknown (truncated) | partial: final waits are 25–31 s native-call acks, statement unknown | partial: retry-conflict lines only | n/a | no |
| Q4 charge vs limit | no (G6) | no | no | no | no | no |
| Q5 server statements | no (G6) | no | no | no | no | no |
| Q6 open at deadline | partial, H: 0 open; `evaluate_base`/`retrieval` failed at ≈900 s | false positive: truncation leaves `normalized_admission` open | partial, H: `analyze_summaries` failed at 18:28:58.82 ≈ start+900 s | **no**: 0 open, final work unattributed | n/a | no |
| Q7 last progress | yes, H: 15:56:27.66 finite, 15:57:50 recursive | no for 18:01:20–18:08:15 | yes, H: still progressing at 18:28:58.81, so not a stall | yes, H: retries committing at 16:20:38.69 | n/a | no |
| Q8 exclusive vs inclusive | no (G4) | no | no (H approx.) | no | no | no |
| Q9 stacks/thread states | no: nothing captured anywhere | no | no | no | n/a | no |
| Q10 cancel took effect, when | partial, H (G7) | n/a | **partial/no** (G7) | partial, H | n/a | partial: timeout shows as `failed`, reason "" |
| Q11 exclusive time per stage | no; inclusive sums via H | partial | no; inclusive via H | no; inclusive via H | inclusive, M | no |
| Q12 rows/statements, server vs client | partial: per-campaign server CPU from journald | partial | partial: row counts only in dropped EOF lines | partial | partial | no |
| Q13 repeated vs reused | no | no | partial, H: binding observations (dropped) | no | no | no |
| Q14 memory high-water vs reservation | server peak per campaign lifetime only | same | same | same | same | no |
| Q15 attribution | partial: no `invocation_id`, so the projection would drop everything | partial (unattributed lines too) | partial: owner/attempt only in span text | partial | partial | yes at step level |
| Q16 coded refusals | partial: drain receipts have no completion | n/a | partial: coded causes in drain receipt; test failure is a 10.8 MB debug dump | partial | n/a | partial |
| Q17 retries | yes, M (raw only): 804 conflict / 506 committed | yes, M (70/46 retained) | partial: 138/82; cancellation event missing (G5) | yes, M: 1139/713 | yes, M: 145/84 | no |
| Q18 completion/finality vs assertions | H: 4 locations | M | H | H | M | **yes**: per-step `native_finality` + attachments, L |

## Material gaps

**G1. Live output never reaches disk.**
- `.config/nextest.toml` sets `status-level = "fail"`, which hides SLOW lines.
- I1 `output.log:5-7` has TERMINATING lines before any test output.
- I3: `nextest-events.jsonl` has only suite/test started until the end, and `native.log` gets output
  only at the end.
- The projection can follow a growing file (`projection.py` marks it `live: true`), but the file is
  empty while the test runs.
- Passing tests keep no output in uncaptured runs. I1's passing compact Catalog test (311.716 s) is
  self-closing in junit; its output exists only in the nextest record store, capped at 10 MB.

**G2. Lost or distorted by the evidence path.**
- **Truncation.** The user config `~/.config/nextest/config.toml` sets `max-output-size = "10MB"`,
  and uncaptured `e3ce73` was recorded under it. `bd6eb5 output.log:9437` drops 26,805,625 bytes,
  leaving a false open `normalized_admission` (line 6908), unmatched terminals and a 414 s fake gap.
  Captured runs use 256 MiB.
- **Dropped by the projection.** `runtime_adapter.py:46-62` keeps only events with a matching
  `invocation_id`, an integer `process_id`, and `phase`/`phase_id`. I3 loses 20,989 `lctx_phase`
  lines, including retries, `base_selected_input_eof` (15,203 lines, 67,372 rows) and
  `completed_input_binding` (4,055). I1, I2 and I4a have no `invocation_id` at all.
- **Double-counted.** I3's 101,614 native-phase rows are 50,808 from junit plus 50,806 from
  `native.log`, so about 50.8k unique events.
- **Readers.** Native-phase rows are always `outcome: "observed"`, so report failure groups and
  `cases --outcome failed` never show a failed phase. `events` pages 50 rows with no
  phase/status/time filter: about 2,051 pages for I3.

**G3. No wait or CPU signal inside long phases.**
- I1 finite: `evaluate_base` begins at `output.log:16151`, has one 480 ms child, then nothing until it
  fails at 16156 (16:04:39.99). The recursive test shows the same pattern for 410 s.
- `phase::wait` fires only on instrumented awaits. Nothing records an in-flight DB statement, lock,
  join or CPU sample, so CPU-bound work and waiting look identical.
- `base_selected_grain` appears 0 times in I1 versus 296 lines in I3.

**G4. Phases have no nesting.** Phase events have no parent id (`phase.rs:9-30`); nesting exists only
as the text span prefix. `evaluate_base` is 492.8 s inclusive against 0.48 s of instrumented children.

**G5. Present but unattributed, or missing.**
- Some I2 lines carry `provider{…}` without the `compilation{attempt}` span, or a bare
  `INFO lctx_phase:`.
- I4a's last 20 s are retry lines outside any open phase.
- I3's failure says "native transaction retry cancelled". Both sources of that string, `retry.rs:174/214`,
  log a terminal event first, yet no `cancelled_before_issue`/`cancelled_after_conflict` line appears.
  Either it was emitted and lost, or it went to a different dispatcher: the test attaches its subscriber
  per future (`scoped_summary.rs:48-57`, `logging.rs:8-12`). Unresolved.

**G6. Resource and server signals.**
- `resource_refusal` is emitted only on refusal (`phase.rs:96-109`). There are zero in any run, and no
  reservation gauge exists.
- The server logs at `warn` (`surrealdb_service.py:653`). From journald the mapper read two
  "transaction dropped without being committed" errors at the I1 and I4a kills, and whole-campaign CPU
  and peak totals: composition 38 min 31 s / 6.7 G; C2 25 min 39 s / 5.0 G; cold 8 min 16 s / 754 M.
  These cover each campaign's whole life, not a stage.
- Control tables (`crates/lctx-native/src/control/schema.surql`) have no timestamps and no
  stage/producer columns. The only progress counters are on retirement jobs.

**G7. Cancellation evidence is unpaired.**
- I3:
  - the cancel request is recorded (`cancel.json`, the run record, `delegation.json` closed 18:26:36);
  - the phase stream continues unchanged (1,457 events at 18:27);
  - the test ends on nextest's own 900 s SIGTERM;
  - all four delegated `recovery.json` files say `requested` 18:29:10, which is normal end-of-run
    settlement (`run_delegation.py:389-397` keeps the first request);
  - so no stop request was written at 18:26:36;
  - the attachment record has no `interrupted` field;
  - drain receipts record `signal: SIGTERM` with no time (`signal.rs:248-251`);
  - the run record keeps only its final state (`runs.py:636-650`).

  Why the stop didn't propagate cannot be answered from the evidence.
- I1: SIGTERM reached the tests. The drain continued until 16:04:47.59 and was killed at 910 s; drain
  receipts stayed "started". I4a is similar.

**G8. Q18 effort.** The pieces live in four places:
- test outcome in junit/`native.log`;
- termination in `record.json`;
- host cleanup in `delegations/*/recovery.json`;
- native completion in `~/.local/state/<campaign>/attachments/<id>/`.

`invocation.json` names the attachment and installation ids but not the campaign directory. Only the
runner's `summary.json` brings these together.

## Discrepancies from earlier facts
- I3 got past Base (`evaluate_base` passed after 322.9 s) and was still progressing at the deadline.
  I1's recursive test was still in Base at 900 s.
- I3's `recovery.json` 18:29:10 time is settlement, not a late stop.

## Coverage limits
- Nextest record stores were not decoded.
- I3's 70 MB junit and 202 MB projection metadata were counted only, not read.
- Other child runs of `65a69e` were not opened.
- Only I3 and I4b record a source context.
