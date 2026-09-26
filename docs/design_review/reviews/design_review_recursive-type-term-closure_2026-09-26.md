# Design review: recursive type-term set closure

**2026-09-26 · change/conformance · scoped review.** Core standard 3.0,
code-intelligence profile 1.1 and library-context binding. Subject: the two
DataFusion recursive type-term relations in `cpg-schema::concepts` and
`cpg-schema::communities`, resolving W13/F07 in the
[forward plan](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition).

## 1–5. Ownership, contracts and scenario

The source `type_terms` and `type_term_args` own structural type identity.
The FCA attribute relation walks from an unknown `Any` child to its
ancestors; its recursive row identity is **term id**. It consumes the result
only as anti-join membership: repeated routes carry no distinct evidence.
The community type layer walks from each function's declared parameter term
to child terms; its recursive identity is **(function id, term id)**. It
consumes only distinct function/class pairs. Neither relation exposes route
count, route order, modality or path provenance. Other queries with those
requirements cannot inherit this choice without a new review.

A malformed or future cyclic type-term edge previously made `UNION ALL`
repeat the same rows until an external recursion bound. DataFusion 55.1's
recursive `UNION` uses distinct rows across iterations, so these two
set-valued walks reach a fixed point. The targeted cycle adds a back-edge to
an otherwise acyclic parent/child pair. Both production SQL relations
terminate and produce exactly the same result as without the back-edge:
the type layer retains one function/class membership, and FCA still withholds
the unknown type attribute while retaining unrelated attributes. Real
analytics variants continue to pass.

The alternatives were a custom visited set, a depth cap, or DataFusion's
distinct recursive union. A cap still leaves set closure incomplete. A
custom traversal would duplicate the pinned library's fixed-point mechanism
and require moving schema-owned SQL semantics into another owner. `UNION`
is appropriate because the actual recursive tuples are the consumed set keys;
it also removes duplicate work from shared subterms. The changed query text
changes the compiler/all-techniques digest and needs its integrated repin.

## 6–8. Judgments, gates and disposition

A1–A3 and FP-01–FP-06 are **satisfied for this slice**: both query owners
retain type semantics, their set contracts are explicit, and adding a cyclic
edge stays local to DataFusion's recursive relation. Applicable
DP-01/02/03/08/11/13/15/16/21/23/24 and CI-01/02/04/06/08/11 pass at the
tested boundary. G1–G8 and CI-G1/CI-G2 **pass for these set consumers**:
typed membership and unknown ancestry are unchanged by the cycle; no
absence is promoted to a negative claim. CI-G3 is unchanged; gold is not an
input. Work/memory cost on the fresh pilot, the integrated digest and Stage 3
recursive summary composition are **unresolved**. No new ADR is needed:
this uses the accepted schema-owned relational contract and the pinned
DataFusion feature, without changing a §B decision.

**Tested 2026-09-26:** `cargo test -p cpg-schema --test
recursive_type_terms --quiet` passed both cyclic/acyclic production SQL
comparisons. `INSTA_UPDATE=no cargo test -p cpg-core --test analysis
variants_add_relational_attributes_and_layers --quiet` passed the real
type-layer variant. `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p
cpg-core --test analysis concepts_come_from_each_seeds_structural_scope
--quiet` passed the real FCA consumer. The latter requires the configured
test stack; a run without that setting overflowed before a result. `just
fmt`, `just test-all`, fresh `just pilot` and the all-techniques digest
repin are `not_run`.

**Decision:** accept this scoped change; W13's pilot and integrated digest
remain in the forward plan.
