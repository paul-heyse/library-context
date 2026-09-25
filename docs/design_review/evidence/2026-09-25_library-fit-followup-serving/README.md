# Follow-up review: serving projections

Date: 2026-09-25. Source inspected at `92eca2e8244a82f574be48e3124855d14f6b8241`.
This is a focused, read-only probe for the follow-up to the library-fit/reasoning review.

## Command and scope

```sh
uv run --no-sync python -B docs/design_review/evidence/2026-09-25_library-fit-followup-serving/probe.py > docs/design_review/evidence/2026-09-25_library-fit-followup-serving/raw/probe.stdout.txt
```

Outcome: `passed` (exit 0; the two diagnostic cases were present). This labels execution of the
probe, not conformance of the observed behavior. No generation was compiled or modified. No
integrated gate or pilot was run.

The probe reads the pre-existing `build/py-fixture/CURRENT` generation:

- generation: `3fc3f0a672790ffe`
- snapshot: `07070707070707070707070707070707` (the fixture snapshot)
- bundle format: 7
- recorded compiler digest: `f11a55821207c57b46fb0999a65ea070ce8b3f9a3379f61c0794438d1b12f7c3`

These are stored fixture inputs, not a fresh compilation of the reviewed tree. The exercised
Python serving code is the current checkout. [probe.stdout.txt](raw/probe.stdout.txt) records the
exact observations; [probe.py](probe.py) records the inputs and calls.

## Observations

**Tested, 2026-09-25.** For `pkg.configure`, the materialized facet
`delegates_to = pkg.controls.Registry.add` has verdict `unknown`. `get_operation` places the
value in its plain list of facet strings; `find_operations` for the same facet value returns no
matches and 17 unknown operations. The per-value verdict is available at load but is omitted by
the lookup result (`operations.py:282-284`); a facet's incompleteness is a separate property and
does not identify which of its values are unknown.

**Tested, 2026-09-25.** A coordinates claim of `pkg.configure` cites only finding
`c60a2729bc4b5c4b839676c6c97a7d03`, with no direct evidence id. The bundle has no findings table.
The capability's Markdown representation omits that finding id. Its 29 evidence records belong
to other direct supports; they do not recover the missing claim-to-finding link. This probe does
not claim the finding is missing from the canonical Delta snapshot: it demonstrates that the
served projection cannot resolve it and its resource rendering removes its citation entirely.

The RCA candidate-call synthesis finding is source-inspected only; this probe does not exercise
the optional `+fca,+rca` compilation path.
