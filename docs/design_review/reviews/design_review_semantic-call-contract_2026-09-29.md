# Native call and signature contracts — bounded review

Independent reviewer Aquinas, 2026-09-29; core 3.0, code-intelligence 1.1, repository binding.
Design/target of `lctx-model::domain::calls`, its model membership and focused controls. This review
covers the raw provider-qualified contract and pure binder/policies; it does not qualify P3 consensus,
SQL projection, P2 producers, full resource accounting or assembled P0 acceptance. Current disposition
belongs to [cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition).

**Outcome: Accept scoped after corrections.** Seven focused controls were independently rerun.

## Owners, evidence meanings and expected changes

`calls` owns native symbol namespaces, complete signature membership, receiver classification,
call-alternative membership and the whole-variant binder. Shared assertion infrastructure owns common
qualification/support validation; the root model owns concrete membership. A provider disagreement
remains a separate attributed alternative. It cannot be discarded before summary uniqueness is
computed. Cross-provider equivalence requires a later explicit relationship; spelling is insufficient.

Signature alternatives remain distinct. Defaults designate definition-time slots without fabricating
runtime values or certifying normal completion. Starred/double-starred operands refuse exact binding
with UnsupportedUnpacking. Bound receivers, empty aggregates, and positional/keyword/implicit values
retain their meaning. A new policy changes the calls owner; an aggregate consumer reads explicit
binding kind and element/key projection instead of reinterpreting the original arguments.

The plain Rust binder is appropriate for this bounded domain policy. Another framework or crate would
not resolve the inspected defects. The existing Domain/Assertion mechanisms provide nominal keys,
physical lowerings and concrete supports; PostgreSQL effects remain outside the model.

## Findings and corrections

<a id="F01"></a>
**F01 — Supporting provider could differ from the native symbol owner (G3, CI-G1).** The original
signature and generic support invariants accepted provider B supporting a signature using provider A's
native key. The model now registers native_symbol_support_ownership, following signature and resolved
call supports through ProviderRun to ProviderSymbol.provider. Cross-provider support refuses without
an explicit semantic relationship. Paired signature/call tests cover both providers; real PostgreSQL
validation also rejects the crossed call support before publication.

<a id="F02"></a>
**F02 — A caller-built shape map could substitute optional for required (G3/G6).** Membership digests
covered referenced IDs, but lookup values were not checked against their keys. Parameter resolution now
verifies each retrieved shape's ID before interpreting required/default semantics. A forged map refuses;
legitimate defaults and complete variants remain accepted.

<a id="F03"></a>
**F03 — Binding output lost aggregate argument meaning (G2/G6).** Original output collapsed receiver,
positional, keyword and implicit bindings and dropped kwargs keys. Output now retains BindingKind and
BindingProjection: Whole, positional element index, or keyword name. A varargs receiver occupies index
zero; subsequent explicit/implicit values advance independently of vector interpretation. Defaults and
empty aggregates remain explicit. Controls distinguish left/right keys and receiver/varargs/implicit
positions [0,1,2].

## Evidence, judgments and boundary

**Tested, 2026-09-29**, through `python3 scripts/build_environment.py --`:

- Reviewer independently reproduced F01–F03 using temporary compiled controls; files were removed.
- Reviewer independently reran `cargo test --release -p lctx-model --test domain_calls`: passed, seven
  controls, after correction reinspection.
- Author ran `cargo test --release -p lctx-postgres --test domain_calls`: passed, real PG18 signature/
  alternative/support readback plus wrong-provider publication refusal. These are contract fixtures;
  the test does not qualify a native call producer.

A1 localizes policy changes; A2 enforces native ownership, membership and explicit binding meaning;
A3 lets aggregate consumers compose without recovering discarded semantics. A1–A3 are satisfied within
the corrected boundary. G1/G4/G7/G8 passed at inspection; the initially failed G2/G3/G6 and CI-G1 are
resolved by the corrections and controls. G5 total coordinated resources remains outside the accepted
claim. CI-G2 serving and CI-G3 evaluation integrity are outside this slice. Formatting, linting and
integrated gates were not_run. C04 remains open through P2/P3 and assembled phase qualification.
