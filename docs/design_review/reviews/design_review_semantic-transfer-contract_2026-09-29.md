# Transfer and structural path contracts — bounded review

Independent reviewer Dalton, 2026-09-29; core 3.0, code-intelligence 1.1, repository binding.
Design/target review of the model-owned transfer/control/selection and path-composition contracts,
shared support expansion, root membership and focused controls. Current finding disposition belongs
to [cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition).

**Outcome: Accept, bounded, after F01 correction.** This does not qualify whole-call composition,
caller/callee journeys, P4 consumers, total resource admission or assembled P0 completion.

## Contracts and architectural judgments

TransferKey is stable aggregation vocabulary, not unconditional asserted flow. TransferAlternative
retains its qualification; merge ORs conditions without changing original alternative IDs. Different
contexts, scopes, modality, approximation, transfer kind and provenance remain separate keys.
ControlInfluence is distinct from value transfer. Selection connects a supported influence and its
atom to a supported guarded alternative, retaining both qualified premise IDs. It is a structural
dependency edge, not a proof of joint satisfiability or unconditional influence.

Structural composition extends the caller input only across caller identity, and the callee output
only across callee identity. Derived steps preserve the original paths. Unknown comparisons yield an
obligation; bounded paths preserve their unknown suffix. Nominal lookup values are checked against
keys, including tails unused after an early difference. Numeric aliases, NaN and wildcard controls
avoid false structural disjointness claims. This is C05(a), not arbitrary Python alias/overload proof.

A1 localizes transfer/path policy in the model and scope enforcement in the shared assertion checker.
A2 encodes stable vocabulary, qualified alternatives and distinct relationship meanings. A3 composes
existing condition/path kernels and nominal subjects. All three hold within the corrected scope.
G5 total resource qualification, C06 rebasing, C05 whole-call/root mapping and enclosing acceptance
remain outside this receipt. No remaining in-scope fidelity/correctness failure was found.

## Finding

<a id="F01"></a>
**F01 — Populated transfer call site bypassed scope/input validation (FP-05, DP-03/08, CI-01/06).**
Shared transfer-subject resolution initially followed only endpoint Places. An existing occurrence
from an unrelated input could therefore be attributed as the transfer call site. Resolution now adds
a present call-site occurrence to the same scope/acquisition checks. Positive local and authorized
corpus cases pass; unrelated-input and outside-artifact-scope cases refuse. The reviewer independently
reinspected the correction and reran the tests. A2/G3 and CI-G1 are corrected at this scope.

## Evidence

**Tested, 2026-09-29**, commands prefixed by `python3 scripts/build_environment.py --`:

- Reviewer independently ran `cargo test --release -p lctx-model --test domain_transfer
  --test domain_assertions --test domain_paths`: passed, five transfer, three assertion, two path.
- Author ran `cargo test --release -p lctx-postgres --test domain_transfer`: passed, real PG18
  readback of keys, alternatives, transfer/control supports and selection; cross-scope call-site
  validation fails before publication. These are contract fixtures, not a behavioral producer.
- Formatting/lints/integrated gates: not_run; functional Phase 0–2 scope remains incomplete.

The later derivation metadata/view implementation has a separate review boundary.

## Author follow-up: condition operand ownership

**Tested, 2026-09-29, author receipt; outside the independent review above.** Condition support
validation now follows each atom's optional operand Place as well as its evaluation occurrence.
A local evaluation cannot import a foreign/out-of-scope operand. This uses the shared support checker
for ordinary syntax assertions too; common condition dependencies now include Place/PlaceRoot.
`cargo test --release -p lctx-model --test domain_transfer --test domain_assertions` passed six transfer
and three assertion controls. Local/authorized-corpus operands pass; unrelated and outside-artifact
operands refuse. The extended `cargo test --release -p lctx-postgres --test domain_transfer` passed
real PG18 validation/publication refusal for the crossed operand. Commands use the build-environment
prefix above. This does not close full resource or assembled phase qualification.
