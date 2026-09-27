# Normal-exit model implications

**2026-09-27; Tested in six independent generated CPython 3.14.7 programs.**

The [ADR-0062](../../../adr/0062-normal-action-postconditions.md) relation means that an exact
reached invocation's normal completion entails its authored Normal rule. It does not prove
feasibility or occurrence of normal completion. Activated actions retain ADR-0060's separate
trigger requirement.

```sh
uv run --no-sync python docs/design_review/evidence/2026-09-27_normal-postconditions/runtime_oracle.py
uv run --no-sync ruff check docs/design_review/evidence/2026-09-27_normal-postconditions/runtime_oracle.py
```

Both **passed**. The [receipt](raw/runtime_receipt.json) records harness/program hashes and exact
runtime outcomes. Each generated program runs in a separate isolated worker with resource/time
bounds and network disabled. Analyzer fixtures and the analyzed library are never executed;
compiler outputs are not oracle inputs.

- Successful `atexit.register` returns the same callback without invoking it. The oracle then
  explicitly runs the worker's exit queue and observes invocation; this later action is never
  attributed to the registration call. Registering a non-callable raises and adds no callback.
- Successful `json.dump` writes a complete encoding. An unencodable later item can produce
  partial writes before TypeError. Those writes do not prove the completed serialization rule.
- Successful `open` returns an open file, which the oracle closes. A missing file raises and
  returns no resource. A symbolic model result is not evidence that such an object exists.

These finite observations challenge the distinction between entry, a normal-exit implication
and an actual outcome. They neither qualify arbitrary inputs nor establish exhaustive channel
coverage. Source/Delta tests use separate static inputs and publication mutation controls;
native selection/rendering, concrete resource identities and integrated Stage 3 remain open.

The final focused release selection **passed 36 cases, 176 skipped** (8.194 s),
`/tmp/lctx-stage3-postconditions-tests2.log`, with `INSTA_UPDATE=no` and this checkout's target
directory. The bounded review records its exact command. Seven real source cases retain
serialization/compression/registration implications while their activated outcomes stay refused.
Binding, missing/replaced obligation, foreign occurrence and original 64→65 proof controls pass;
publication rejects deleted postconditions and removal of Normal qualification.

The first selection passed 33 cases and failed on two expected migrations plus an incorrect
expectation that explicit `open` would fit its overload evidence under the original proof cap.
The corrected test requires its actual `summary_proof_limit` refusal. Ordinary open stays
unsupported, and symbolic acquisition's positive checks use contract inputs only. Those checks
are not a source-backed acquired-resource claim. No native or integrated gate ran for this slice.
