# Retained execution: what the timeout establishes

**Inspected 2026-10-17; historical runs, not fresh executions.** Consumer:
[publication performance target review](../../reviews/design_review_catalog-publication-performance-target_2026-10-17.md).
Source baseline for this inquiry is `7f353fa1`; the captures retain their original source scope.

## Recursive and ordinary integration

Managed run `20261017T204229.825Z-e713d0` invoked:

```text
just fixture -- cargo nextest run --release -p cpg-core --lib --test compiler_artifacts --no-fail-fast -E 'test(selected_assurance_retains_exact_owner_closure_and_refuses_missing_input_binding) | test(scoped_summary::actual_summary_matches_complete_finite_whole_operation) | test(scoped_summary::actual_recursive_summary_preserves_scc_delta_answers_and_exhaustion)'
```

Raw source: `build/runs/20261017T204229.825Z-e713d0/verify-evidence/87a497f550e24c70ac1c0b4cee8360e7/native.log`.
The selected controls ran with normal concurrency. Assurance passed1.436s; ordinary Summary
passed874.607s; recursive timed out900.231s before its oracle. The whole run failed100 and
confirmed host cleanup. This is neither a failed public-manifest seal nor proof of a stalled SCC.

Recursive PID566281 / attempt `d385e9d9ba754221907b49ec297a954bb9b879cd61b327f9fd7b209aa9d08018`:

| Observation | UTC / duration | Interpretation |
|---|---|---|
| Initial native setup begins | 20:43:46.761837 | Near start of the timed control, not command compilation/build time |
| Summary producer phase begins | 20:53:42.380071 | 595.618s after initial setup; acquisition, predecessor stages, completion and retained-product work already consumed substantial time |
| Summary phase fails on termination | 20:58:46.820680 /304.441s | Encloses preparation, kernel, discharge and emission; not a kernel-only measurement |
| PairOutcome writes acknowledged | 19 full4096-row batches /77,824 rows | Kernel result exists and emission is progressing; full required output/disposition mix and later oracle remain unknown |
| Final cancellation/ingress errors | After timeout | Consequences of cancellation, not elapsed-work attribution |

Phase extraction filters `process_id=566281`, takes terminal `phase`, `status` and `elapsed_ms`
fields, and groups by phase name. Selected totals below **overlap** and are not an additive
wall-time breakdown:

| Phase | Terminal events | Sum seconds |
|---|---:|---:|
| native_batch_write | 463 | 498.554 |
| native_product_retention | 13 | 118.866 |
| native_compound_selection_assurance | 13 | 58.618 |
| native_compound_inventory_receiving | 13 | 28.985 |
| native_compound_child_installation | 13 | 23.514 |
| evaluate_base | 1 | 96.679 |
| provider_execution | 6 | 88.731 |
| provider_completion | 6 | 76.574 |
| analysis_native_inventory | 1 | 52.646 |
| enrich_execution | 1 | 49.622 |
| catalog_evidence | 1 | 47.842 |
| apply_models | 1 | 47.811 |

There are37,489 `native_call_acknowledgement` terminals;35,397 have no producer span. The count
does not establish37,489 database round trips. During20:46–20:53, per-minute median acknowledgement
durations are0.007–0.010ms; cheap completed/prepared calls coexist with slow genuine operations.
The examined native table stream uses one retained driver, not one NativeCalls task per row.
Do not infer a row-at-a-time transport defect from this event count alone.

The Summary test compiles the complete Analysis frontier, then explicitly admits semantics and
replays the ordinary finite producer over independently acquired/prepared inputs before comparing
all Summary output families. Production and replay share the underlying semantic algorithm.
Separate source-grounded recursive assertions exercise identity/relay/nested/guard/no-base/opaque
behavior, SCCs and accounting. Those assertions were not reached by the recursive failed run.
No public `lctx-publisher::seal_completed`, snapshot selection or operator adoption is called.

## Bounded native physical-write control

Managed run `20261017T211603.926Z-68c73f` invoked:

```text
just fixture -- cargo nextest run --release -p lctx-compiler-store --lib --no-fail-fast -E 'test(native_summary_pair_outcome_batch_has_committed_window_receipts)'
```

Its single selected test passed15.714s (nextest suite15.724s). The4096 rows have synthetic nominal
endpoints, so this tests physical persistence and ownership, not Summary answers. All73 window
effects committed, indexed contribution membership was4096, the worker's submitting subscriber
observed its batch span, ingress/session drainage succeeded, and reservations returned to zero.
Attachment `493c88fc-5dee-4dd0-be3e-f81d2f92cf46` released with native finality confirmed.

Passing-test phase output is retained in
`build/runs/20261017T211603.926Z-68c73f/verify-evidence/9fc2ffcec9e54e8fb72a513c608cd4b1/junit.xml`,
under the selected testcase's `system-err`; console absence is not absence of phase evidence.

| Observation | Count / elapsed |
|---|---|
| Logical rows / payload nodes / aliases / memberships | 4096 /4096 /0 /4096 |
| Ingress | 15.152s |
| Logical lowering | 0.109s |
| Immutable-window lowering | 76 events /0.223s |
| Immutable effects | 73 windows /14.818s |
| Physical expectation reads | 73 /0.175s |
| Atomic apply | 84 /12.697s |
| Recovery settlement | 84 /1.496s |
| Settlement apply (nested) | 84 /0.618s |
| Settlement issuance (nested) | 84 /0.080s |
| Nodes / memberships / aliases | 8.760s /6.283s /0.000006s |

The84 apply/settlement phases include surrounding lifecycle operations, not just73 payload windows.
Nested timings overlap. Atomic application is the dominant observed bucket in this one run;
server parse, planning, function evaluation, record/index work and commit are not separated.
Neither general throughput nor the size of a possible improvement follows from these timings.

## Preservation and unknowns

The recursive session/attempt remains unresolved in attachment
`d866a170-a139-4c1a-98f4-21799b42a4e1`; earlier tracing-failed attachment
`f752c29f-8618-42d0-989e-aafc62ad1e98` and six older attachments remain protected.
Host cleanup and the ordinary/assurance native receipts do not settle recursive remote finality.
The review performs no release, recovery or cutover. Existing capture storage remains retained;
this evidence does not convert old failures into passes.
