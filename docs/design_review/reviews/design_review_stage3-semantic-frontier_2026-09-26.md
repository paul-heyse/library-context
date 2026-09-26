# Stage 3 semantic frontier and default obligations

## 1. Scope and outcome

**Interface-checked, 2026-09-26; change/conformance; Accept scoped after correction.**
Core 3.0, code-intelligence 1.1 and the repository binding govern this review under ADR-0057.
The read-only reviewer examined value-only semantic scheduling and default-binding admission.
No reviewer tests ran. Multi-channel composition and assembled Stage 3 remain outside this judgment.

## 2. Responsibilities and fidelity

Schema owns explicit/default binding obligations. The pure analytics worklist owns semantic
progress, bounded representatives, retained witnesses and refusals. Core continues to acquire,
publish and reconstruct the same relations; the native path consumer receives retained proofs.

| Fact | Meaning | Fidelity boundary |
|---|---|---|
| Semantic value state | Same origin, source, return, condition, paths and declared result | Witness ID and depth do not change semantics |
| Shorter representative | Less depth consumed by a proof of that state | May enable a bounded caller; does not close coverage |
| Default requirement | Omitted fixed formal needs definition-time availability | Syntactic optionality is not a call-time guarantee |

## 3. Change scenario

A recursive cycle produces a longer proof of an already known value fact. It retains a local
witness without restarting semantic propagation. A later shorter proof can reopen a caller
previously refused at the depth limit. Another semantic alternative's refusal must survive.

## 4. Analysis contract

The existing typed finite-summary inputs and SCC schedule feed an origin-preserving semantic
key. Ordered pending pairs consume current representatives only. Results retain ordered proof
IDs, typed refusals and independent witness-omission disclosure. Omission follows actual
`CalleeSummary` dependencies. Source default admission remains conservative until certificates exist.

## 5. Consumer journeys

A finite recursive base stops at semantic stability instead of repeated proof unrolling. A
base-free cycle remains unknown. Explicit optional literals still bind, including keyword-only
parameters. Omitted source defaults withhold a positive even when the callee never reads them.
The focused native fixture covers both explicit and omitted arguments through a Delta snapshot.

## 6. Gates

G1–G8 and CI-G1/CI-G3: no new in-scope violation identified after correction. CI-G2 remains bounded
by existing FORMAT 8 support closure; full FORMAT 9 proof reconstruction is not certified here.
No new engine, dependency pin, framework or alternate source of semantic authority is introduced.

## 7. Findings

<a id="F01"></a>
**F01 — Superseded representatives left stale refusals.** Initial per-seed depth/condition flags
could survive after a shorter proof enabled the same semantic alternative. Correction: refusals
are keyed by `SemanticFlow`, replaced only when that alternative is reprocessed. Distinct blocked
alternatives remain. The [forward plan W12](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns current disposition. Focused producer tests cover recovery, a distinct blocked alternative
of the same caller, separate deep origins and deterministic ID variations.

## 8. Architecture judgment

A1–A3 are satisfied for this slice: schema owns binding meaning; semantic and witness progress
have separate roles; shorter representatives compose without erasing unrelated unknowns.
Default requirements remain independent of explicit argument evaluation order.

## 9. Alternatives and library fit

Retaining the accepted bounded SCC scheduler requires ordinary maps and ordered sets. Replacing
it with a Datalog engine here would not supply default stability or completion semantics. The
planned Ascent/datafrog comparison remains required on the actual future multi-channel contract.

## 10. Author verification

**Tested, 2026-09-26:** 29 focused cases passed with `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo
nextest run --release -p cpg-schema -p lctx-analytics --lib -p cpg-core --test bundle -E
"test(binding_tests) | test(evaluation::tests) | test(summaries::finite::tests) |
test(finite_depth_and_unsupported_refusals_reach_the_native_response)" --status-level fail
--final-status-level fail`. The expanded same-caller recovery control then passed with the same
settings and `-p lctx-analytics --lib -E 'test(shorter_representative_reopens)'`.
Eight independent [CPython controls](../evidence/2026-09-26_default-availability/README.md) passed.
Fixture embeddings are synthetic. Earlier compilation failures were corrected before these receipts.

## 11. Remaining integration

Source-default certificates, context exits, call-specific stable values, residual conditions,
multi-channel states and callee/model coverage remain incomplete. A retained finite witness is
not exhaustive coverage. Full tests, formatting, fresh pilot and serving qualification are `not_run`
until the complete functional scope is ready.

## 12. Revisit

Revisit when another summary channel changes semantic identity, when proof selection affects a
consumer beyond bounded depth, or when source-default admission is added. Compare engines only
on equal semantic, refusal and witness contracts.
