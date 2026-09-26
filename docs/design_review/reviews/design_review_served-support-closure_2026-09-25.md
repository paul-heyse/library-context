# Design review: served support closure

**2026-09-25 · change/conformance · scoped review.** Standard: core 3.0, code-intelligence
profile 1.1 and the library-context binding (`standard.toml`). Subject: ADR-0049 and the FORMAT 8
bundle/loader/renderer slice through `d9d8a40`. This review is evidence, not the disposition
owner; [plan W3](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns remaining work and integrated acceptance.

## 1. Scope, outcome and coverage

The change addresses the finding-only `coordinates` claim demonstrated in the follow-up review's
F03. It adds `support_findings`, `support_witnesses` and `support_members` to the immutable
generation, with exact schemas owned by `cpg-schema`; the bundle selects cited findings from one
published snapshot; the loader refuses missing edges; FastMCP structured and Markdown forms use
one hydrated support object. The supported source-span claim is **witness-backed findings whose
call, syntax or declaration site resolves**. A member with only a fact identity is explicitly
`fact_only` or `unavailable`. This local pass does not certify every Stage 3 claim or the pilot.

## 2. Responsibilities and fact fidelity

| Owner | Input and semantic identity | Output and consumer |
|---|---|---|
| Canonical `findings`/`analysis_invocations`/`witnesses`/`finding_members` | Snapshot-qualified analytic ids, model and cited facts; `witnesses` retain path and step | Authority for finding and derivation meaning |
| `cpg-schema::bundle::files` | Named, typed FORMAT 8 fields and sort keys | Rust writer and Python schema check |
| `cpg-core::bundle` | Published session at recorded versions; only brief-cited findings | Sorted, byte-identical IPC files; row and byte refusal |
| `lctx_mcp.generation` | Manifest-bound IPC and support keys | Refuses missing cited finding/evidence/member fact/witness span |
| `lctx_mcp.server` | One loaded generation | Structured `Support.finding` and assertion-local Markdown links |

CI fact fidelity: `finding_id` and `invocation_id` are semantic ids, and `source_fact_id` remains
the source row's fact id. The projection does not turn a candidate witness into a definite call;
it retains `modality`, `arc_kind` and `phase`. The source span is located by the witness site id,
not by guessing from a callee name. A fact-only member retains table/model identity and reports
that no generic source span was resolved.

## 4–5. Composition and change scenario

For a new finding kind that cites a call, syntax or declaration site, the canonical relation and
its existing witness path feed the same projection and renderer without a new Python classifier.
For a new witness-site kind, `cpg-core::bundle` must add a justified source resolver; the loader
will otherwise refuse the generation instead of serving a source-free witness. For a new renderer,
the `FindingSupport` object is the contract; it does not have to rejoin the canonical tables.
The three queries select the same cited universe but repeat their short cited-ID subquery. That
is a local SQL cost, not a second semantic classifier; a measured pilot cost can justify a shared
DataFusion view later. The file cap is 100,000 rows and 64 MiB per support file; over-cap output
fails, never truncates a support path.

## 6. Correctness and fidelity gates

| Gate | Scoped verdict and evidence |
|---|---|
| G1/G2, CI-G1 | Pass for the examined path: Delta remains authoritative; finding kind, status, model and witness modality are preserved. |
| G3, CI-G2 | Pass for the witness-backed coordinates claim: missing closure row is refused and one source span resolves. The broader fact-only source-span guarantee is explicitly outside this scoped claim. |
| G4/G5 | Pass at the examined boundary: bundle reads a published session; startup only reads its pinned generation; deterministic rebuild passed. |
| G6/G8 | Pass within this choice: Arrow IPC, DataFusion SQL, PyArrow and Pydantic remain the existing mechanisms; no new evidence engine or server-side Delta reader. Pilot cost is not measured. |
| G7 | Pass for the stated witness-backed claim; `fact_only`/`unavailable` labels prevent a false source-resolution promise. |
| CI-G3 | Not affected: evaluation references and gold inputs were not touched. |

## 7–9. Findings and alternatives

No new blocking finding was established for the witness-backed claim. The **known limit** is a
finding member whose `cited_fact_id` does not have an exported generic span. Its identity, source
table and model remain in the support object, with `fact_only`; a consumer requiring a source
span for such a finding must add an owned fact-location projection and a positive/withholding
fixture before claiming full source closure. [Plan W3](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns that trigger. ADR-0049 compares the simpler ID-only rendering and the broader full-catalog
or live-Delta routes. The selected cited closure has less serving coupling and tests without an
analysis session; the new schemas and FORMAT bump are real integration costs.

## 10. Verification and uncertainty

**Tested 2026-09-25:** `RUST_MIN_STACK=16777216 cargo test -p cpg-core --test bundle
finalizer_proof_round_trips_through_the_native_generation_reader --quiet` passed, including the
finding-only coordinates model/source trace, Markdown parity and missing-row refusal;
`a_generation_rebuilds_to_the_same_bytes` passed; targeted
`uv run --no-sync pytest` for `test_server.py`, `test_digest.py`, `test_native_semantics.py` and
`test_operations.py` passed. `just adr lint` passed. `just fmt`, `just test-all`, fresh `just
pilot` and structured Stage 3 evaluation are **not_run** under the active plan's end-of-scope
boundary. The 64 MiB cap and the full pilot support distribution have not been measured.

## 11–12. Authority and architectural judgment

ADR-0049 updates §6.4, §10 and §11.3; the executable serving schemas and validators remain
the field-level authority. FP-01–FP-06 and applicable DP-01/02/03/08/11/13/16/19/21/24,
CI-01/02/03/06/11/13 are **satisfied for the bounded projection** by the owners and traced
round trip above. A1 (local change), A2 (one coherent support contract) and A3 (testable without
server or Delta at query time) are satisfied for this slice. **Accept scoped** at Tested strength
for witness-backed support closure. The enclosing Stage 3 architecture remains unqualified until
the plan's assembled gate and review; fact-only source expansion has the stated consumer trigger.
