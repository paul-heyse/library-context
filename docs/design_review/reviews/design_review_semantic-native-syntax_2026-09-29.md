# Native syntax into typed relations — bounded change review

Independent read-only reviewer Hegel, 2026-09-29. Core 3.0, code-intelligence 1.1 and repository
binding; change/conformance against DESIGN §15 and cutover P0-E. Inspected `typed_syntax.rs`, its
conformance harness, fixture and dependency wiring. No files were changed by the reviewer.

**Outcome: Accept scoped.** No blocking in-scope findings. This is an identifier-observation subset,
not complete syntax coverage or assembled P0 acceptance. Production capture/acquisition, total
resource accounting, remaining representative families and facts-frontier admission remain open in
[the current cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md#42-execution-status).

## Responsibilities, fidelity and expected changes

`cpg-extract` owns interpretation of the pinned native AST. `lctx-model` owns source/span/kind/role/
structural-path identity and qualified assertion/support contracts. `lctx-postgres` owns COPY,
sealed validation, publication and leased readback. A native node-kind addition changes one exhaustive
provider translation; PostgreSQL consumers do not receive native AST types. A new sink composes
through the fallible callback and retains publication validation.

The harness reads Pyrefly's retained Ruff parse and its module text from the same transaction;
no legacy rows or IDs enter the emitter. The emitter verifies the source hash and byte length before
calling its sink. AST/text correspondence remains an explicit caller precondition. Names preserve
source spelling and SourceObservation/NativeTraversal/NativeStructural support. Coverage distinguishes
Partial/OutsideProviderModel, Partial/SyntaxError and Unavailable/UndecodableSource. Undecodable bytes
remain artifact chunks. A published conformance subset cannot be selected as facts.

## Library fit and composition

The pinned Ruff 0.0.11 SourceOrderVisitor provides structural traversal over Pyrefly's existing parse;
no second syntax parser or visitor framework was introduced. Inspection confirmed balanced enter/leave
callbacks after Skip. Parent-relative ordinals distinguish structural occurrences, including separate
with-items. The bounded fixture harness uses existing tempfile ownership and the permanent typed/Arrow/
COPY/readback contracts; PostgreSQL additions to the extractor are dev-dependencies only.

The emission/depth limits stop further output after refusal. Ruff can still dispatch later siblings,
so these limits do not establish a total traversal-work bound. The API documentation states this;
coordinated producer/resource work remains open. The harness deliberately collects a fixture smaller
than 1 MiB and is not a production assembler or memory qualification.

## Evidence and judgment

**Tested by the author, 2026-09-29**, with `python3 scripts/build_environment.py --`:

- `cargo check -p cpg-extract`: passed.
- `cargo test --release -p cpg-extract --test typed_conformance`: passed, real pinned Pyrefly and
  disposable PostgreSQL 18. Controls cover relocation, Unicode byte spans, separate with-items,
  changed-source refusal before emission, sink/refusal propagation, conservative coverage, full
  binary chunk readback, typed support/occurrence readback and facts-selection refusal.

The reviewer inspected the implementation, exact pinned traversal source and controls; independent
reruns and broad gates were not_run. A1 localizes native mapping changes; A2 encodes occurrence and
attribution meaning; A3 composes a fallible emitter with permanent model/store boundaries. A1–A3 are
satisfied within this scope. G1–G8 and CI-G1 satisfy the bounded contract at inspection strength plus
the stated test receipts. CI-G2 serving closure is outside scope; CI-G3 shows no evaluation-reference
leakage. Revisit at assembled P0-E and when production resource/capture contracts replace the harness.
