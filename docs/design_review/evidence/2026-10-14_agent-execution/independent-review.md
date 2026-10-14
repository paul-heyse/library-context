## Independent integrated implementation review

**Decision: Accept scoped, at Implemented evidence strength.** I found no outstanding material correctness or architecture defect in the inspected agent-effectiveness slice. This is an independent static judgment, not functional acceptance or qualification of the remaining Catalog work.

I examined the dirty working tree over `2fc7f7d5f68e28bfd871d74460cda411cc4d8fc4` on 2026-10-14, concluding at approximately 22:42 UTC. Coverage includes C00, T01–T04, I03, P01 and the narrow O02 diagnostic additions. It includes the final delegated-owner escalation block and the NativeCalls test-fixture closure helper. I performed no edits, builds, services or tests. Changes made afterward require review of their affected boundaries.

The assessment uses core design principles3.3 and the efficiency heuristics, with the repository’s code-intelligence1.5 profile and binding. No graph/fact interpretation, serving semantics or evaluation contract changes are assessed here; CI-G1–CI-G3 therefore remain outside this slice.

### Ownership and cleanup

Prospective registration addresses the original F09 gap at the correct boundary. [`register`](../../../../scripts/run_delegation.py) holds admission across launch publication, while [`close_admission`](../../../../scripts/run_delegation.py) closes that same boundary. Each registration owns its receipt and lock; aggregate observations retain uncertainty instead of rewriting the child’s original result. `SpawnGuard` settles the registration after direct cleanup.

[`request_stop`](../../../../scripts/run_delegation.py) now also handles an ordinary owner that ignores TERM. My initial inspection identified that waiting for active delegated registrations could postpone direct cancellation indefinitely. The revised implementation groups requests by exact owner identity, retains the common initial request time, and checks identity plus held ownership immediately before escalation. Recovery remains separate from original receipts. Independent profiler observers retain their existing finalization policy. The corresponding multiwriter TERM-ignore control is present; I did not execute it.

Aggregate cleanup reaches the consequential consumers: run/storage release, profile completeness, development outcomes, campaign quiescence and worktree removal. Direct process cleanup and native finality remain separate. This preserves the original review’s distinction rather than merely changing status wording.

### Discovery and capture

[`run_listing.page`](../../../../scripts/run_listing.py) limits default detailed hydration and reuses a process snapshot. Cursor validation binds selection to directory membership and explicitly describes live observations rather than promising a frozen historical snapshot. Filtering may still examine substantial history; the implementation does not claim otherwise.

The worktree consumer explicitly requests `--all --json` and rejects incomplete or unstable coverage at [`worktree.py`](../../../../scripts/worktree.py). Corrupt records remain protected.

Capture uses a short manifest-registration lock and invocation-owned UUID files at [`CaptureOwner.register`](../../../../scripts/verify_evidence/capture.py). Actual launch seams retain cwd, original/effective argv and fixture context. Process-bound preparation markers prevent duplicate capture in one launcher without suppressing a child’s independent nested invocation. Unsupported adapters retain the command. Ad hoc rerun planning explicitly refuses to invent prerequisites. Verification summaries and progress are session-owned, with a separately rebuilt parent projection.

### Native finality and diagnostics

P01 preserves the important distinction between an operation error and unfinished ownership. [`OperationResult`](../../../../crates/lctx-compiler-store/src/dispatcher_completion.rs) retains owned errors without reconstructing them from display text. Its finality traversal preserves nested Outstanding/Unknown, storage, committed identities and actual finalizer failures.

NativeCalls and NativeBridge expose retryable finality separately from retained operation diagnostics. Workspace, independent and borrowed admission, CLI and restore consumers append diagnostics after settling the actual finality signal. The additional [`close_fixture`](../../../../crates/lctx-compiler-store/src/native_calls.rs) helper closes the real native fixture independently of injected dispatcher results; it does not relabel those results as successful cleanup.

Phase observations retain dispatcher/span attribution and process/invocation identity. Pending waits produce observations without fabricating completion on Drop. Resource diagnostics reuse the constructed typed refusal and preserve the library’s formatted cause; reservation quantities are not presented as RSS.

### Configuration, judgments and limits

Project and role model pins are removed consistently. Functional role contracts and effort remain explicit. ADR0171/0172 and DESIGN distinguish operator-selected inheritance, accepted ownership changes, current implementation and broader unmeasured effectiveness.

| Judgment | Scoped result |
|---|---|
| A1 — Localize change | Satisfied: ownership, capture, discovery and dispatcher semantics have identifiable owners. |
| A2 — Encode meaning | Satisfied: direct exit, delegated cleanup, operation failure, finality and capture health remain distinct. |
| A3 — Compose capabilities | Satisfied: existing owners and adapters supply the operations; no universal scheduler is introduced. |
| A4 — Execution fit | Satisfied qualitatively: bounded discovery avoids unrelated detailed hydration, and prospective ownership preserves necessary independent lifetimes. |

G1–G8 have no identified outstanding violation in the inspected source and supported scenarios. No additional rule impact is proposed beyond the already approved dispositions.

Focused compilation, actual native/fixture acceptance, model-inheritance receipts and non-functional checks remain the coordinator’s evidence obligations. This review does not establish their outcomes, measured performance, P02–P06 closure, full Catalog/Summary/cold/mutation/FastMCP acceptance, or enclosing product qualification. Current disposition remains solely in Catalog coordinator §6.

Addendum, 2026-10-14 at 22:44 UTC: **Accept scoped remains unchanged; no new material findings.**

The final [`_signal_owner`](../../../../scripts/run_delegation.py) pins the target with `pidfd_open`, rechecks the recorded identity and held owner lock, signals through the descriptor and closes it. TERM and KILL use this boundary. Unsupported facilities or errors retain Unknown rather than falling back to a numeric PID signal. This strengthens the reviewed cancellation contract.

The CPG control cleanup changes settle the actual native fixture after their dispatcher/provider assertions. The deliberate provider-panic case still records failed execution finality; subsequent native abandonment does not rewrite that immutable signal. Interrupted-drain and batch-retention assertions remain intact.

This follow-up was source inspection only. The coordinator-reported successful native, Workspace and host controls are separate execution evidence; historical Unknown receipts remain preserved.
