# Independent action-trigger challenges

**2026-09-27; Tested in nine generated CPython 3.14.7 programs.**

`runtime_oracle.py` creates programs in a temporary directory and runs each in a separate
isolated worker with CPU/address-space/time bounds and network disabled. It never imports
`fixtures/python/` or executes the analyzed library. `sys.monitoring` observes the generated
caller's actual `json.dump` invocation; a recording stream independently observes writes.
The producer under test is not an oracle input. The receipt retains generated-source, harness
and installed pinned `json` source hashes.

```sh
uv run --no-sync python docs/design_review/evidence/2026-09-27_action-triggers/runtime_oracle.py
uv run --no-sync ruff check docs/design_review/evidence/2026-09-27_action-triggers/runtime_oracle.py
```

Both **passed** after two harness line-length errors were corrected. The runtime receipt is
[retained here](raw/runtime_receipt.json). Normal serialization writes; a caller raise occurs
only after those writes; an unencodable later element produces partial writes before TypeError;
an unencodable root reaches the call without writing. A prior raise, raising argument, opaque
prior failure and skipped branch never invoke dump. Omitted defaults execute successfully in
this runtime example even though the current source proof deliberately withholds availability.

The catalog timing was qualified against the installed CPython **3.14.7** `json.dump`,
`gzip.compress` and `logging.Logger.warning` implementations with `inspect.getsource`.
Context7's official CPython JSON documentation supplied the streaming API lead; current main
is not treated as pinned implementation evidence. Serialization/compression are completed-result
models requiring Normal. `io_write` and `log` are potential partial actions under Invocation.
These finite observations do not establish exhaustive coverage or actual writes for arbitrary
arguments, custom encoders or streams. Reached invocation never changes Potential to Definite.

The source/Delta test `action_triggers_preserve_partial_io_and_withhold_unproved_outcomes`
compiles separate static input, checks positive I/O and explicit refusal rows, compares these
independent observations, and removes action/invocation evidence to challenge publication.
The normal/Finally positive admission controls use synthetic contract inputs because current
production action models have no total-normal-return assertion. They are not production model
qualification. Full native action support, resource identity, broader target/binding domains
and integrated Stage 3 remain open; the forward plan and bounded review own current scope.

The final focused release selection **passed 30 cases, 177 skipped** (5.694 s),
`/tmp/lctx-stage3-actions-tests6.log`, with `INSTA_UPDATE=no` and this checkout's target directory.
It includes the original 64-to-65-step composition boundary and malformed-modality/schema
controls. The [bounded review](../../reviews/design_review_stage3-channel-contracts_2026-09-26.md)
records the exact selection, earlier corrected test-control failures and remaining acceptance.
