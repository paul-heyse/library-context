# Design review: normal evaluation of modeled source operands

**2026-09-26 · change/conformance · scoped author review.** Core 3.0,
code-intelligence profile 1.1 and the library-context binding apply. This
reviews the one-call model argument fact, its pure summary proof and the
adjacent preceding-call and nested-chain consumers. [Plan order 1](../../plans/behavioral-model-forward-plan_2026-09-24.md#32-functional-completion)
owns the wider evaluation contract; [ADR-0055](../../adr/0055-modeled-source-read.md)
owns this alternative choice.

## Contract and change scenario

The selected model argument previously had one `source_operand` row citing a
raw value-flow fact. That was a source observation, not a proof that reading
the expression completes. A second nullable evidence field now carries its
normal-read witness. The existing `simple_argument_evidence_sql` classifier
supplies an exact ty reaching fact for a direct parameter where available.
The pinned ty flow line marks a `try` body reaching row approximate; a direct
lexically resolved parameter can instead cite its resolution fact if the
current function contains no explicit deletion or exception-handler frame.
The append-only `lexical_parameter_normal` code distinguishes that fallback
from exact ty reaching and from raw flow. A direct modeled source with neither
witness becomes unknown.

| Fact or stage | Fidelity and identity | Coverage and consumer |
|---|---|---|
| Raw `flow_values` contribution | Provider observation, origin and use identity | The model source; no completion claim |
| `flow_reaching` / `reference_resolutions` | Exact ty read, or resolved direct parameter binding with structural exclusions | Shared simple-argument classifier; unresolved/approximate nonparametric names stay unknown |
| `modeled_argument_evaluations` | Derived status, raw evidence and separate read evidence, checked by Arrow schema invariant | Pure finite producer and shared publication validator |
| Ordered summary steps | Raw identity followed by argument read at the source position | Delta and FORMAT 8 native proof reader |

The expected change is a modeled return within a pass-only `try/finally`.
Requiring only exact ty reaching loses that valid path; accepting only raw flow
would hide an unproved source expression. The lexical route uses existing
Ruff name syntax, lexical resolution and scope tables, not a new parser or
provider. DataFusion's joins and `NOT EXISTS` exclusions keep construction
and validation in the same relation. One pure producer orders the two evidence
steps. A possible `del value` in the same function withholds the lexical route;
the real `framed_maybe_deleted_identity` control has a raw model candidate but
no normal-read witness or positive summary.
An exception-handler exclusion is implemented but has no separate real-source
challenge in this slice. The broad function-wide exclusions trade precision
for a locally reviewable safety boundary and can be narrowed when an in-scope
consumer needs it.

| Judgment | Scoped result |
|---|---|
| A1–A3, FP-01–06 | Satisfied: provider observation, derived evaluation status, pure admission and serving retain distinct owners; adding a new safe direct-parameter context changes one shared classifier and its proof consumer. |
| G1–G3, G6–G7, CI-G1–CI-G2 | Satisfied in tested cases: the schema requires two source witnesses, the shared validator reconstructs them, the framed positive cites lexical resolution, and possible deletion withholds. Ty approximation is never relabelled as exact. |
| G4–G5 | Satisfied within the slice: pure admission remains effect-free; current-store schema migration follows ADR-0048, and only the existing snapshot append publishes. |
| G8 | Satisfied: ty and Ruff provide the observations; the narrow Python binding rule is repository-owned §B5 semantics, not a replacement analyzer. |
| CI-G3 | Unaffected: gold references remain outside analysis inputs. |

Applicable DP-01/02/03/05/08/11/13/15/16/18/19/21/23/24 and
CI-01/02/04/06/08/11 are satisfied for the tested scope. General expression
normal completion, precise deletion reachability, exception-handler effects
and path-specific predecessor order remain in the plan; this slice does not
certify those contracts.

**Tested 2026-09-26:** `cargo test -p lctx-analytics --lib summaries::finite
--quiet` passed 16 pure controls, including a missing source-read witness.
`INSTA_UPDATE=no cargo test -p cpg-schema --test codebooks --test contracts
--quiet` passed after review and acceptance of the append-only codebook,
schema and generated-rule snapshots. `RUST_MIN_STACK=16777216 INSTA_UPDATE=no
cargo test -p cpg-core --test compile
pinned_identity_models_require_and_publish_their_real_formals --quiet` passed
the framed positive with its cited resolution fact, a raw candidate withheld
for possible deletion, and shared publication validation. `RUST_MIN_STACK=16777216
INSTA_UPDATE=no cargo test -p cpg-core --test bundle
finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`
passed the current schema through FORMAT 8 and native serving. `just fmt`, `just test-all` and `just pilot` are
**not_run** until functional completion.

**Accept scoped at Tested strength.** Stage 3's general evaluation and
multi-channel composition remain incomplete.
