# Captured input tree — bounded change review

Independent reviewer Carson, 2026-09-29. Change/conformance under core 3.0, code-intelligence 1.1 and
repository binding. Inspected `cpg-extract::capture`, its unit/integration controls, native conformance
integration, and model-owned ContentHasher/path validation. No reviewer edits or new findings.

**Outcome: Accept scoped.** The caller supplies the complete selected inventory and quiesces its
acquisition. This checked copy is not a filesystem-wide atomic snapshot or protection against hostile
concurrent path replacement. Total RSS, other producer/store allocations, production assembly and
full P0 qualification remain open in [the plan](../../plans/semantic-model-cutover-plan_2026-09-29.md#42-execution-status).

## Ownership, composition and fidelity

Filesystem effects and change detection stay in `cpg-extract`; content identity, relative-path rules,
canonical chunks and complete-content proofs remain in `lctx-model`. Existing tempfile owns the frozen
tree and its lifetime. Another provider consumes that root and verifies afterward; a new sink consumes
canonical chunks without inventing hashing or chunk rules. Native syntax coverage remains Partial;
undecodable source remains Unavailable with retained bytes.

Capture hashes the bytes it writes, checks open/path metadata and length, and rechecks earlier files
before returning. Reads stop at the original length plus one byte, so an append-only writer cannot be
chased indefinitely. The private tree rejects selected symlinks and exposes read-only files. Dropping
the owner attempts deletion of its own directory; ordinary destructor cleanup errors are unreported.

Reservations precede capture inventory/buffer allocations. Temporary stamps are dropped before
shrinking metadata charges, and retained artifacts keep their charge. Canonical emission owns read/
chunk reservations until buffers drop. A retaining callback must reserve its own copies. Sink failure
propagates; complete success requires the shared verifier's finish. Earlier callback output remains
provisional and must be abandoned with the surrounding staging attempt.

## Evidence and judgments

**Author-tested, 2026-09-29**, commands use `python3 scripts/build_environment.py --`:

- `cargo test --release -p cpg-extract --test capture --test typed_conformance`: passed, three capture
  controls and one real pinned Pyrefly/PG18 conformance test. Includes a 65 MiB artifact streamed under
  a 3 MiB capture reservation budget, empty/undecodable bytes, source independence after capture,
  modified frozen-content refusal, sink-error cleanup, metadata budget refusal and symlink refusal.
- `cargo test --release -p cpg-extract --lib capture::tests`: passed, changed-during-capture refusal and
  reservation cleanup; rerun after the final bounded-read edit.
- `cargo check -p cpg-extract`: passed before the final bounded-read edit; subsequent release tests
  compiled and exercised that edit. The reviewer did not rerun tests or broad gates.

A1, A2 and A3 are satisfied within this boundary: coherent effects owner, typed content/lifetime
contracts and reusable capture/chunk composition. G1–G8 and CI-G1 passed at bounded source-inspection
strength with the attributed test evidence. CI-G2 serving and CI-G3 evaluation integrity are not
exercised. Continuous append refusal is source-inspected; the mutation control changes a file after
copy. No process-RSS measurement or integrated product qualification is claimed.
