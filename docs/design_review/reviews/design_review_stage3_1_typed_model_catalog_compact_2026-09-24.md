# Stage 3.1 typed model catalog — compact change review

## Scope and decision

**Subject:** committed TOML catalog, typed rule/path parser, pinned context target binding,
`model_targets` and `model_transfers`, and shared publication validation. **Standard:** core
2.0, code-intelligence 1.0 and the library-context binding. **Tier/purpose:** compact
change/conformance. **Reviewer/date:** Codex, 2026-09-24. **Decision:** accept these rows as
authored model data with pinned target citations, not as source-observed flows or served
behavior verdicts. The Stage 3 integrated gate remains `not_run` at operator direction.

## Authority and transformation

Serde/TOML tagged variants reject unknown fields and invalid path shapes. Input and output
paths have one Rust renderer; authored TOML does not parse a second string grammar. Model
identity includes committed file bytes, exact target pin and revision; the catalog digest
joins the compiler digest. An analyzed attempt parses and binds before any Delta write.
Applicable stdlib targets require a matching Python patch and a bundled-typeshed module;
dependency targets require a matching distribution/version and site-package module. A pinned
module without its qualified callable fails. A model for a different pin remains dormant.
Release-local targets and active non-transfer rules fail closed because their binders and
consumers are not built.

`model_targets` cites the module and definition facts with `synthetic_model` provenance;
`model_transfers` derives its canonical paths and stable rule id from the typed committed
model, per bound callable/overload. The shared validator reconstructs both tables from the
catalog and pinned context views, checking full row equality as well as SQL foreign-key and
provenance rules. No summary or negative claim reads these rows yet.

## Gate verdicts

| Gate | Verdict | Evidence and limit |
|---|---|---|
| G1 authority / G2 fidelity | Pass in scope | Pinned context facts identify targets; authored transfer claims remain distinct from source-observed flow. |
| G3 validity / G5 publication | Pass for stored rows | Malformed or unresolved applicable models abort before writes; the shared validator catches forged rows. Positive pilot model rows await the Stage 3 end run. |
| G4 effects | Pass in scope | The compiler parses source facts and never executes the analyzed library. CrossHair runs only an isolated authored probe. |
| G6 transformation | Pass in scope | Stable model/rule ids and source fact citations preserve the derivation; missing target/path support cannot become a verdict. |
| G7 claims | Pass for the narrow data claim | Focused tests, schema snapshots and a bounded oracle support the stated behavior; no full pipeline claim follows. |
| G8 library leverage | Pass | Serde/TOML provides strict variant parsing; the existing Arrow table macro, DataFusion validator and Delta publication path avoid parallel stores or parsers. |
| CI-G1 / CI-G2 / CI-G3 | Pass for attributed model rows; unresolved for serving | Every target has cited facts, but formal signature binding, summaries and a generation-pinned native consumer are absent. |

DP-01/02/03/04/05/07/08/11/13/14/15/18/19/21/22/23/24 and
CI-01/02/04/06/07/10/11/12/13 pass **for the typed data boundary**. No claim is made about
closed-world call behavior, full model semantics, or a served compatibility answer.

## Findings and disposition

| ID | Finding and consequence | Disposition |
|---|---|---|
| Deferred M01 | `Parameter[val]` is authored but not yet matched to the pinned callable's formal signature; a stale spelling would silently weaken or misroute a later summary. | Bind each used input/output path to a resolved signature before a summary consumes it; reject unresolved formals. |
| Deferred M02 | The target binding cites a Pyrefly bundled typeshed definition for the stdlib callable, not a proof of its runtime implementation. The isolated CrossHair probe covers only `int`. | Keep `synthetic_model` provenance and verify each pure model's semantics under a stated pin/domain; an unexplored CrossHair path is unknown. |
| Deferred M03 | Only transfer rules have a compiled table and consumer contract; callback, resource, exception and effect rules remain parsed but inactive. | Build each typed rule family with its own citation and publication validator before authoring an active model. |
| Deferred M04 | No positive model row has yet been published on the real library under the final catalog, and native serving has no model reader. | Confirm binding counts and exact model revisions in the final fresh-store pilot, then require the one-generation native load check. |

## Verification

**Passed, 2026-09-24:** focused release-profile Nextest selections cover strict parsing,
pin mismatch, missing callable, compiled paths, schema/codebook/rule snapshots, tampered
target/transfer rows, the synthesis ledger and no-analysis publication. Focused Clippy on
`cpg-schema` and `cpg-core` passed with `-D warnings`; `cargo check --workspace` passed.
An isolated CrossHair 0.0.110 control found `value=0` for a wrong model; the `int`
specialization of `typing.cast` versus identity exhausted paths with no difference
([receipt](../evidence/2026-09-24_typing_cast_model_oracle/README.md)).

`just test-all`, fresh-store `just pilot`, Stage 3 Q01/Q03/Q05/Q09, clean wheel/native query
and increment-end review are `not_run`: they belong to the assembled Stage 3 end gate.
