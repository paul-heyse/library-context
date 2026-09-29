# Mechanical codec allocation — bounded review

## 1. Scope, outcome and coverage

| Field | Assessment |
|---|---|
| Tier · purpose | Change · conformance; compressed template slots 1, 6, 7, 8 and 12 |
| Reviewer · date | Independent Codex reviewer, 2026-09-29 |
| Subject | Uncommitted generated streaming encoders, borrowed sum physical rows and borrowed record-key encoding, including inspected F01 reservation correction |
| Standard | [Core 3.0, template, code-intelligence profile 1.1 and repository binding](../design_principles/standard.toml); design-review and companion skill |
| Authority | [ADR-0085](../../adr/0085-typed-semantic-domain.md), [ADR-0086](../../adr/0086-immutable-postgresql-generations.md), [cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md) |
| Decision | **Accept scoped**; no identity, lifetime or codec-correctness defect identified by inspection; non-blocking F01 corrected |
| Evidence | **Implemented / Interface-checked, 2026-09-29:** independent source inspection. Test results below are author-reported. |
| Exclusions | Full resource admission/budgeting, measured allocation or RSS improvements, zero-copy guarantees, producer coverage and phase qualification |

Reviewed [`lctx-model-macros/src/lib.rs`](../../../crates/lctx-model-macros/src/lib.rs),
[`record.rs`](../../../crates/lctx-model/src/domain/record.rs),
[`identity.rs`](../../../crates/lctx-model/src/domain/identity.rs) and the added
[`domain` control](../../../crates/lctx-model/tests/domain.rs).
Inspected exact local dependency source under
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/serde_arrow-0.15.1/`:
`src/arrow_impl.rs`, `src/internal/array_builder.rs`, `src/internal/serializer.rs`,
`src/internal/serialization/{struct_builder,utf8_builder}.rs` and array-buffer helpers.
The workspace selects `=0.15.1` with `arrow-59`; no dependency change is introduced.

**Ownership and scenarios — Interface-checked, 2026-09-29.** The bounded derives still own
mechanical physical-row generation; `Key`/`KeySink` own semantic encoding, and serde_arrow owns
array construction. For a large key payload, `Record::id` now writes borrowed key fields instead
of cloning an owned key. Explicit `key()` APIs remain available. For a sum with optional or absent
payloads, generated physical references preserve active/inactive columns and optionality.

- Struct `write_key` encodes the same key-field list in the same order as the generated owned key.
  Sum `write_key` delegates to the existing enum `Key::encode`. Namespace, scalar/container framing,
  tag encoding and digest truncation remain unchanged (`identity.rs:22–35`).
- `ArrayBuilder::push` serializes synchronously into owned builder buffers. Borrowed row-local
  physical structs do not escape. Required payloads become `Some(&T)`, optional payloads use
  `Option<&T>`, and inactive columns remain `None`.
- All-unit sums omit the unused lifetime; they retain physical ID/tag columns. Existing schema,
  unknown-tag, inactive-payload and identity checks remain in the decode path.
- The former convenience API uses the same builder and finalization path. The new loops remove
  the intermediate physical-row vector and sum payload clones; they still build a complete Arrow
  batch, and `Batch` retains typed rows alongside Arrow. This is not end-to-end bounded streaming.

### Focused receipts

All receipts are **author-reported, 2026-09-29**, not reviewer-executed or independently log-verified.
Commands identify the reported suites; they do not imply a single combined original invocation.

| Suite command | Outcome and scope |
|---|---|
| `cargo test --release -p lctx-model --test domain --test domain_assertions --test domain_documents` | **passed** before the added control: domain 17, assertions 3, documents 3 |
| `cargo test --release -p lctx-model --test domain` | **passed — author-reported**, 18 tests both before and after F01's reservation correction, including borrowed-ID versus owned-key equality, all-unit/optional sums, empty encoding and a 1 MiB string. |
| `cargo test --release -p lctx-postgres --test generations` | **passed**: two real PG18 generation/chunk tests, including the author-reported 65 MiB case; no separate post-reserve rerun receipt supplied |
| `cargo test --release -p lctx-model --doc` | **passed**: seven positive and seven compile-fail examples; one old example ignored; no separate post-reserve rerun receipt supplied |
| Reviewer tests, formatting, linting and integrated gates | **not_run** |

The author reports the post-reserve focused domain rerun passed on 2026-09-29. Other earlier
passes are not relabelled as post-reserve execution. No additional checks were run for this artifact.

## 6. Correctness and fidelity gates

| Gates | Verdict | Source-inspection evidence / limit |
|---|---|---|
| G1 Authority; G2 Semantic fidelity | pass scoped | The same declared key fields and enum encoder determine identity; physical shape and semantic distinctions remain unchanged. |
| G3 Validity; G6 Transformation and reuse | pass scoped | Borrowed values serialize synchronously; optional/inactive payload and decode checks persist. No new cache/reuse mechanism. |
| G4 Hidden behavior | pass | Encoding introduces no ambient input or external effect. |
| G5 Consistency and recovery | n.a. to lifecycle | No publication protocol changes; full resource admission is excluded. |
| G7 Truthful capability claims | pass scoped | Source-level clone/vector removal is distinguished from measured performance and full memory budgeting. |
| G8 Library leverage | pass after F01 correction | Existing pinned ArrayBuilder API supplies construction, reservation and finalization. |
| CI-G1 Fidelity | pass scoped | Mechanical encoding does not strengthen or reinterpret provider assertions. |
| CI-G2 Evidence closure; CI-G3 Evaluation integrity | n.a. | No serving or evaluation-reference path changed. |

## 7. Findings and applicability

<a id="F01"></a>
### F01 — Preserve known row-count reservation

**Original priority: Low; non-blocking.** Stable source:
`design_review_semantic-codec-allocation_2026-09-29.md#F01`.

**Original finding — Interface-checked, 2026-09-29.** Both generated encoders replaced sequence
serialization with per-row `push` but omitted the prior capacity reservation. Pinned
`src/internal/serializer.rs:83–87` calls `reserve(len)` for a known sequence length; the public
builder provides `reserve` at `src/internal/array_builder.rs:69–71`. Removing the intermediate
vector need not discard that known-size preallocation. Runtime/RSS impact was not measured.
Relevant rules: DP-14/16; correction owner: `lctx-model-macros`.

**Correction evidence — Implemented / Interface-checked, 2026-09-29.** Independently reinspected
both generated loops: each now calls `builder.reserve(rows.len())` immediately after successful
construction and before pushing rows (`lib.rs:253` and `lib.rs:479`). This restores the missing
library reservation. Source reinspection accepts the correction; the author reports the
post-reserve 18-test domain rerun passed on 2026-09-29. This does not establish byte-level reservation or RSS bounds.

Current scheduled disposition belongs to
[cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition).
This review preserves the source finding and dated closure evidence, not a competing status register.
No other actionable finding was identified. FP-01–06 and relevant DP-01/04/08/14/22/23 obligations
hold for the inspected mechanical transformation, subject to the stated evidence limits.

## 8. Library fit and total complexity

**Interface-checked, 2026-09-29.** Direct use of pinned serde_arrow's `from_arrow`, `reserve`,
`push` and `into_record_batch` replaces intermediate owned physical rows without a new framework
or serializer. The existing owned decode representation remains appropriate for returned records.
No bespoke array builder, extra dependency or new semantic identity recipe is introduced.

## 12. Architectural judgment and decision

| Judgment | Verdict | Bounded evidence |
|---|---|---|
| A1 Localize change | satisfied | Mechanical encoding stays in the derives; semantic encoding stays with existing key contracts. |
| A2 Encode meaning structurally | satisfied | Field order, enum tags, optionality, nominal identity and decode validity remain explicit and equivalent by inspection. |
| A3 Extend through composition | satisfied | Borrowed physical rows compose with the pinned builder, including all-unit sums; existing explicit owned-key APIs remain usable. |

**Bounded decision: Accept scoped.** F01 is corrected by source reinspection. Known focused
passes, including the post-reserve 18-test domain rerun, remain explicitly author-reported.

**Enclosing architecture: not assessed as complete.** No full-budget, RSS, zero-copy, admission
or phase-completion claim follows. Revisit allocation admission with the shared resource work,
and requalify this boundary if key encoding, supported field shapes or serde_arrow changes.
