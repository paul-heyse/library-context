# Frame cleanup and caller continuation

**2026-09-27; Tested in ten generated CPython 3.14.7 programs.**

`uv run --no-sync python docs/design_review/evidence/2026-09-27_frame-exit/runtime_oracle.py`
**passed**. The [receipt](raw/runtime_receipt.json) records worker/harness hashes and all events.
Each worker is isolated with CPU, memory, wall-time and network bounds. No analyzer fixture or
analyzed library was executed; compiler results are not inputs.

An ordinary unused-argument function, `typing.cast` and `typing.assert_type` each produce a
`PY_RETURN` body event before a temporary argument's finalizer starts. That finalizer sleeps
30 seconds; the worker is killed at the two-second bound without caller continuation. The same
calls with a literal or an independently retained caller argument continue normally. Retained
controls exit before shutdown cleanup so shutdown does not masquerade as invocation cleanup.
A custom class namespace additionally returns an ephemeral value from `__getitem__`; its apparent
name read has no ordinary function-local retention root. The compiler therefore withholds
class-body frame certificates, including the superficially closed-argument case.

This challenges treating a body event as a complete callee-return certificate. The timeout is
an observation of delayed continuation, not proof of divergence. These programs demonstrate
the missing premise in the catalog rationale; they do not reproduce an admitted compiler false
positive. [ADR-0063](../../../adr/0063-frame-exit-completion.md) owns the correction; the active
plan owns implementation and closure. General object-lifetime analysis is outside this probe.

## Compiler qualification

**Focused Tested, 2026-09-27.** Compiler97/catalog7 carry typed direct-return bodies and
independent frame-release certificates. The release nextest selection recorded in STATUS passed
69 cases (151 skipped), including exact source publication, raw/nested/assignment native
whole-group omission, missing argument/step support, class-namespace withholding, defaults with
known invocation and unknown cleanup, and original proof bounds. All changed schema/codebook
snapshots were inspected and accepted. These are schema migrations, requiring fresh generations.

`CARGO_TARGET_DIR=/home/paul/library-context/target uv sync --frozen --reinstall-package lctx-semantics`
**passed**. After that build and fresh fixture generation,
`uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py -q` **passed**, 13 cases.
The first replay failed eight cases because the old fixture lacked the new required frame files;
its five independent cases passed. No fallback schema or relaxed admission was introduced.
`uv run --no-sync pyrefly check python/lctx_mcp/src/lctx_mcp/generation.py` **passed**, zero errors.
Native proves the bounded structural/catalog commitment; full raw-signature and read-provenance
closure and integrated Stage 3 acceptance remain open.

The follow-up source admission selection (`cargo nextest run --release -p cpg-core -p
lctx-analytics --lib --test compile --test bundle` with the finite/model/composed-argument/native
selection in STATUS) **passed**, 28 cases. Adding the independent retained-call occurrence guard
then yielded **27 passed, one failed**: native had also applied completed-call admission to the
root of an invocation-only frame proof. Shared `admit` already validates those final two root
anchors; native now checks nested completion on the preceding operand group. After rebuilding:
`CARGO_TARGET_DIR=/home/paul/library-context/target RUST_MIN_STACK=16777216
target/release/deps/bundle-cea1e98b65ba0b62 --exact
finite_depth_and_unsupported_refusals_reach_the_native_response` **passed**, one case in 13.02 s.
It rejects outer frame+normal+model-rule deletion while an inner transfer remains. The 13-case
Python replay passed again. Deleting an entire source occurrence including its anchors is part
of the still-open full S6 source-evidence closure obligation.

## Source-body foundation

**Focused Tested, 2026-09-27; compiler98.** The shared statement kernel produces body outcomes
independently of parameter flows, with a separate closed-local release commitment and mandatory
unresolved function-retainer obligation. No call continuation or native source-call behavior is
claimed. Normal fallthrough, explicit Return, exact TypeError and unknown remain distinct;
first closed initialization does not silently make a later normal read a closed value.

`CARGO_TARGET_DIR=/home/paul/library-context/target INSTA_UPDATE=no RUST_MIN_STACK=16777216
cargo nextest run --release -p cpg-schema -p lctx-analytics -p cpg-core --lib --test contracts
--test compile -E 'binary(contracts) | test(completion::) | test(source_body::) |
test(source_body_outcomes_do_not_invent_value_flows_or_caller_continuation) |
test(composed_argument_reads_keep_ordered_source_evidence)' --status-level fail --final-status-level fail`
**passed**, 26 cases, 187 skipped, 15.232 s (`/tmp/lctx-stage3-source-body-tests4.log`).
Tests cover source/Delta reconstruction, retained terminal and expression-root segments,
resealed foreign/reordered/merged/missing support, mandatory callable retention, docstring-only
empty runtime bodies, unsupported ownership domains and the original 64→65 proof boundary.

Earlier runs had expected schema migrations and a fixture brief without its required documented
Outcome; that fixture metadata was corrected without changing compiler admission. A 25-case
intermediate run passed; review then tightened proof segments, shared docstring selection and
indexed preparation. The final no-update run includes those corrections. Snapshots were read
and accepted as schema migrations.

`uv run --no-sync python docs/design_review/evidence/2026-09-27_frame-exit/source_body_oracle.py`
and its corresponding Ruff check **passed**. The [source-body receipt](raw/source_body_receipt.json)
records twelve independently generated CPython 3.14.7 controls. It includes body/caller event
ordering, exact TypeError, docstring-only fallthrough, and a temporary zero-argument function
whose attribute finalizer delays caller continuation. A retained function resumes normally.
No analyzer fixture is executed; finite observations and a worker timeout are not universal
proofs. Call-site discharge, full native evidence closure and integrated Stage 3 remain open.

## Fresh source-call consumption

**Focused Tested, 2026-09-27; compiler99.** A separate fresh-call certificate proves exact
zero-argument binding, successful immediate definition creation and callable retention before
consuming a base body with closed release/result provenance. Normality under entry does not assert
that the caller reached the call. Literal return, docstring-only fallthrough, return override in
`finally` and a nested pinned call reach the real source→Delta→bundle→native route. Raising bodies,
unknown local releases, formal defaults, captures, aliases and intervening statements withhold.
An unreachable caller produces no feasible derived behavior or value path; unknown is preserved.

`CARGO_TARGET_DIR=/home/paul/library-context/target INSTA_UPDATE=no RUST_MIN_STACK=16777216
cargo nextest run --release -p cpg-schema -p lctx-analytics -p cpg-core --lib --test contracts
--test codebooks --test compile --test bundle -E 'binary(contracts) | binary(codebooks) |
test(completion::) | test(source_body::) | test(source_call::) | test(evaluation::) |
test(actions::) | test(reach_fixed_point_tests::) |
test(source_body_outcomes_do_not_invent_value_flows_or_caller_continuation) |
test(fresh_source_calls_require_body_binding_and_release) |
test(serving_schema_digests_are_the_shared_known_answers)' --status-level fail --final-status-level fail`
**passed**, 54 cases, 173 skipped, 9.074 s (`/tmp/lctx-stage3-source-call-tests8.log`).
The integrated fixture invokes [20 native controls](native_source_call.py), including removal
of entire support tables and source-call occurrence groups, plus substitution of another caller's
valid certificate. Shared Rust admission additionally rejects resealed valid foreign bodies and
removed/replaced declaration or binding header evidence. The action control admits a 63-step
expanded invocation while refusing its 65-step Normal/Finally proof; no bound was increased.
Two source-call tables, codebook additions, rules and five served schemas are reviewed migrations.

Earlier focused runs exposed the expected migrations, exact-false derived projections, a
completion-result variable shadow and a syntax-node/lexical-reference identity mismatch. Source
tracing corrected the adapter to use the reference's explicit `name_node_id`; no admission
premise was relaxed. The 53-case intermediate run passed 52 cases before that positive-path fix.

`uv run --no-sync python docs/design_review/evidence/2026-09-27_frame-exit/source_call_oracle.py`
**passed**, ten independently generated CPython 3.14.7 controls, with a
[hashed receipt](raw/source_call_receipt.json). Programs never read analyzer fixture files or
compiled results. Supported continuations and conservative withholding cases are distinct.
The corresponding oracle/native-script Ruff checks and `generation.py` Ruff/Pyrefly checks
**passed**. Full raw-source serving closure and integrated Stage 3 acceptance remain open.

The final native rebuild,
`CARGO_TARGET_DIR=/home/paul/library-context/target uv sync --frozen --reinstall-package lctx-semantics`,
**passed**, 32.67 s. Then
`CARGO_TARGET_DIR=/home/paul/library-context/target RUST_MIN_STACK=16777216
target/release/deps/compile-258cb583674e070e --exact fresh_source_calls_require_body_binding_and_release`
**passed**, one case, 34 filtered out, 7.65 s (`/tmp/lctx-stage3-source-call-native-replay.log`).
This reruns all 20 native controls against the current shared guard.

The adjacent finite/default/action/native regression selection recorded in review §34 **passed
29 of 30 cases** initially; the remaining case expected the old generic control-flow boundary
for source calls outside the fresh nested domain. Source admission now preserves the more
specific `scope_boundary`, with zero positive paths unchanged. After updating the exact SQL/native
expectations, the affected nextest replay **passed**, one case, 226 skipped, 12.881 s
(`/tmp/lctx-stage3-source-call-regressions3.log`). The full 30-case selection was not repeated.
The selection also rebuilt the Python fixture with the new required support files;
`uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py -q` **passed**, 13 cases.


## Reached source invocation

**Implemented and focused Tested, 2026-09-27; compiler100.** The common invocation owner now
accepts explicitly typed source targets through independent fresh binding/header support, with
no model identifier or normal-body premise. Reached raising, captured and unknown-body calls
can invoke while their normal summaries remain withheld. Failed headers and unreachable callers
withhold. An independently indexed callee scope excludes generators, including unreachable
yields. Duplicate-aware indexes replace repeated global binding/normal/candidate scans.

The schema migration adds `source_call_bindings`, keys header steps by binding, reduces normals
to binding/body support, makes call execution targets explicit and appends `SourceInvocation`.
Four table contracts, codebooks, reference rules and three served projections were inspected and
accepted. Publication independently reconstructs binding, invocation and normal relations; native
exports only bindings referenced by served normals. Full native source-invocation serving and
raw-source reconstruction remain open. Existing expanded proof caps are unchanged.

```sh
CARGO_TARGET_DIR=/home/paul/library-context/target INSTA_UPDATE=no RUST_MIN_STACK=16777216 LCTX_PY_FIXTURE=/home/paul/library-context/build/py-fixture cargo nextest run --release -p cpg-schema -p lctx-analytics -p cpg-core --lib --test contracts --test codebooks --test compile --test bundle -E 'binary(contracts) | binary(codebooks) | test(completion::) | test(source_body::) | test(source_call::) | test(call_execution::) | test(evaluation::) | test(actions::) | test(summaries::finite::) | test(reach_fixed_point_tests::) | test(source_body_outcomes_do_not_invent_value_flows_or_caller_continuation) | test(fresh_source_calls_require_body_binding_and_release) | test(serving_schema_digests_are_the_shared_known_answers) | test(pinned_identity_models_require_and_publish_their_real_formals) | test(action_triggers_preserve_partial_io_and_withhold_unproved_outcomes) | test(composed_argument_reads_keep_ordered_source_evidence) | test(finite_depth_and_unsupported_refusals_reach_the_native_response) | test(finalizer_proof_round_trips_through_the_native_generation_reader) | test(writes_the_python_fixture_generation)' --status-level fail --final-status-level fail
```

**passed:** 87 tests, 141 skipped, 28.477 s (`/tmp/lctx-source100-tests3.log`). The real fixture
runs 25 [native controls](native_source_call.py), including whole-table removal and foreign-caller
substitution. Shared Rust tests reject mixed source/model variants, missing or foreign binding
support and duplicate normal/binding IDs, and distinguish expanded 64 versus 65 invocation steps.
The prior 63/65 invocation-versus-action test remains unchanged in meaning.
The first run **failed:** 45 passed, five failures from pending schema/serving migrations and the
old native installation (`/tmp/lctx-source100-tests1.log`). The 49-case second run used
`INSTA_FORCE_PASS=1` only to generate all pending migrations; it is not the qualification receipt.
Review also found the generator phase and quadratic preparation defects; both were corrected
before the final no-update run.

`CARGO_TARGET_DIR=/home/paul/library-context/target uv sync --frozen --reinstall-package lctx-semantics`
**passed**, 31.21 s (`/tmp/lctx-source100-native-sync.log`), before the final native checks.
`uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py -q` **passed**, 13 tests
against the newly generated fixture (`/tmp/lctx-source100-python.log`).

`uv run --no-sync python docs/design_review/evidence/2026-09-27_frame-exit/source_call_oracle.py`
**passed**, 14 independently generated CPython 3.14.7 controls. The current harness extends the
prior ten-case receipt above with explicit body-entry observations, failed header, unknown body,
and two generator controls. The [current hashed receipt](raw/source_invocation_receipt.json)
distinguishes generator creation from body entry and caller return. No fixture or compiler
output is executed or used as an oracle expectation. Ruff for the oracle/native/generation files
and Pyrefly for `generation.py` **passed** with zero errors. The fixture was parsed only.
Integrated Stage 3 gates remain `not_run` until the remaining S1–S7 work is assembled.
