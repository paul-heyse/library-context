# Typed assertions and conditions — bounded change review

Independent read-only reviewer Kuhn, 2026-09-29. Core 3.0, code-intelligence 1.1 and repository
binding; change/conformance against ADR-0085/0086 and cutover P0-B. Reviewed the dirty typed value,
condition, assertion and derive changes over `04a99a6`, followed by correction reinspection.
Current finding disposition belongs to [cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition).

**Outcome: Accept scoped after corrections.** The enclosing P0 architecture remains incomplete.
Excluded: artifact chunks, stage execution, lifecycle changes, coordinated resources, remaining
representative families, producer integration and phase-exit qualification.

## Ownership and expected changes

Ordinary domain records own proposition identity; the bounded Assertion derive emits a concrete
support relationship and contract accessors. Shared model validators enforce attribution without
storage-specific domain branches. Adding a provider can add support without changing qualification;
adding an assertion family must carry keyed qualification and require its support companion.

Biodivine 0.6.3 owns Boolean operations. The model supplies occurrence-keyed atom identity,
canonical typed persisted nodes, work/node bounds and unknown-on-refusal. The port preserves the
existing bounded substitution algorithm without an old-ID adapter. Display paths remain a bounded
projection of the canonical BDD. New engine adoption was unnecessary for this boundary.

## Findings and inspected corrections

<a id="F01"></a>
**F01 — Condition evaluation escaped invocation input (G3 / CI-G1).** A source-A assertion could
use a condition atom evaluated in unrelated input B with the same context. Qualification and support
validators originally accepted it. Support validation now follows condition closure to evaluation
occurrences and checks acquired input and declared scope. Paired controls accept same-input and
explicit corpus membership, and reject unrelated input, missing membership and out-of-scope sources.

<a id="F02"></a>
**F02 — Qualification could be omitted from the key (G2/G3).** A derived assertion with an unkeyed
qualification collapsed Candidate and Definite identity and then conflicted. Assertion derive now
refuses this declaration. A compile-fail control pairs with a correctly keyed declaration; distinct
qualification IDs have distinct proposition IDs.

<a id="F03"></a>
**F03 — Omitting support membership removed validation (G3; A2/A3).** A validated model could retain
SyntaxObservation while omitting SyntaxSupport and its invariant. Generated companion dependencies
now make this fail model admission. Complete membership remains accepted.

## Evidence and architectural judgment

**Tested, 2026-09-29.** Initial reviewer runs passed the six then-existing condition/assertion tests
and the tagged-sum malformed-payload test. Independent stdin-compiled controls reproduced all three
defects; temporary binaries were removed. Correction reinspection found no remaining in-scope defect.
The reviewer independently observed paired assertion doctests passing. Author-reported corrected
evidence: three assertion tests, two real PostgreSQL generation tests, and six positive/six negative
declaration controls passed. Commands use `python3 scripts/build_environment.py --`:

- `cargo test --release -p lctx-model --test domain_conditions --test domain_assertions`
- `cargo test --release -p lctx-model --test domain tagged_sums_preserve_active_optional_null_and_reject_inactive_payloads`
- `cargo test --release -p lctx-model --doc`
- `cargo test --release -p lctx-postgres --test generations` (author run)

A1 localizes changes in model owners. A2 encodes qualification and typed attribution. A3 makes the
assertion/support dependency part of admission instead of extension-time hidden knowledge. All three
are satisfied within the corrected boundary, with applicable fidelity gates satisfied by the stated
controls and inspection. Full formatting, linting and integrated gates were **not_run**. Revisit when
remaining assertion families and producers assemble at P0-E; this review cannot certify that assembly.
