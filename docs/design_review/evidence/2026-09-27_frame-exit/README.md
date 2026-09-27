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
