# Pinned call default availability

**2026-09-27; Tested against the installed CPython 3.14.7 implementation.**

`qualify.py` independently compares each target's source default declarations, runtime
signature and an explicit expected formal list. It qualifies `json.dump`, `json.dumps`,
`json.loads` and `gzip.compress` individually. The receipt records function-source and harness
hashes. These observations support already-created default availability under the exact pinned
implementation; they do not supply default values, mutable-value stability or normal return.
The model's pinned assumptions exclude arbitrary mutation of function metadata.

```sh
uv run --no-sync python docs/design_review/evidence/2026-09-27_pinned-defaults/qualify.py
uv run --no-sync ruff check docs/design_review/evidence/2026-09-27_pinned-defaults/qualify.py
```

Both **passed**. The [receipt](raw/receipt.json) also retains seven generated programs, run
in separate isolated workers with CPU/address-space/time bounds and network disabled.
`sys.monitoring.PY_START` observes actual callee-body entry: missing required arguments can
produce a CALL event without entering the function. No analyzer fixture or analyzed library
is executed, and no compiler output is an oracle input.

Dumping with omitted options enters and returns; a later unencodable value enters, writes
partial output and raises. Dumps and loads enter before their respective encoding/decoding
failures. Compression enters and returns. Missing required arguments and raising explicit
arguments prevent body entry. These are finite observations, not exhaustive behavior claims.

The separate static source/Delta action test compares the corresponding entry claims for
all four targets while withholding normal completion. Pure contract controls distinguish
availability from normal return, require all-signature agreement and known requiredness,
retain signature-specific omitted-formal identities, and preserve obligations under work
refusal. Shared admission checks exact root-group ownership, order and independent binding
commitments; removing all retained default evidence cannot erase the omitted-formal domain.
Publication still reconstructs raw source binding and model application independently.

Nested default-proof groups in a flattened invocation require their own binding commitments
before admission; broader default values, local callable domains and native action serving
remain open under the [active plan](../../../plans/behavioral-model-forward-plan_2026-09-24.md).
Context7's CPython documentation supplied the definition-time-default lead; current main
documentation is not treated as pinned implementation evidence.

The final focused release selection **passed 33 cases, 176 skipped** (6.797 s),
`/tmp/lctx-stage3-pinned-defaults-tests4.log`, with `INSTA_UPDATE=no` and this checkout's target
directory. Earlier attempts encountered expected schema migrations and a corrected print control:
that callable's currently unsupported binding domain does not isolate default availability.
The four-target source challenge instead removes only the availability premise and checks
DefaultUnavailable plus publication rejection. The shared huge-count control refuses before
allocating a marker range. Full native action/Stage 3 acceptance remains `not_run`.
