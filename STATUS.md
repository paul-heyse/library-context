# Status

_Updated 2026-09-27 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- **Increment 4 / Stage 3 remains functionally incomplete.** The operator-approved [forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) owns S0–S8; §6 owns finding disposition. ADR-0057/0058 accept the target, not completion.
- **Current S5 slice:** typed current FCA/RCA attributes and Pass C handoffs. Semantic keys retain category, target, modality and phase; occurrence incidences retain object and source evidence. Consumer formals remain incidence-local. Labels are presentation, produced by one renderer; label-prefix consumers are removed.
- Implications retain their premise-extent supporters. Publication and serving require support for each cited supporter and every retained handoff pair, complete pair members, both endpoint facts/edges and the formal. Handoff pairs are not delegation-chain witnesses. Source reconstruction validates retained incidences and source labels; it does not certify full lattice completeness.
- **Migration:** compiler output **88**, extractor output 32, synthesis template 20. FORMAT **9** includes typed finding support and callable/source-identity support; remaining S6 semantics are unimplemented. Fresh stores/generations follow ADR-0048.
- **Earlier implemented subsets:** typed value/site coverage with complete empty escaping-exception domains under site entry; fresh immediate nested definition/default certificates; shared callee proof admission; transfer alternatives and occurrence-specific lexical parameter-identity certificates. Raw provider approximation remains recorded; entry/exit evidence is independently required.
- **Next accepted target (ADR-0059):** pinned class protocols supply synchronous lifecycle assertions separately from observed-call phase matching. The provider probe is interface evidence only; constructor phases, manager/entry-result identity and cleanup obligations stay distinct. No context implementation is complete.
- **Model contracts:** catalog format 3 authors phase and static/runtime/unresolved validation schema attribution separately from effects. No production validation effect is active. Call-origin and operation-wide coverage remain open; no witness closes unrelated paths or channels.
- The [bounded review](docs/design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md) distinguishes corrected implementation subsets from assembled Stage 3 acceptance. No remote push requested.

## Last verified (2026-09-27)

Cargo commands use `CARGO_TARGET_DIR=/home/paul/library-context/target`: the inherited shell points at `/home/paul/pse-arrow/target`, so checks explicitly reuse this checkout's cache.

| Command | Outcome and scope |
|---|---|
| `LCTX_PY_FIXTURE=/home/paul/library-context/build/py-fixture INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p lctx-analytics -p cpg-schema -p cpg-core --lib --test contracts --test codebooks --test analysis --test bundle --test syntax -E 'test(concepts::) \| binary(contracts) \| binary(codebooks) \| test(concepts_come_from_each_seeds_structural_scope) \| test(fca_scopes_are_nodes_and_state_only_their_own_apis) \| test(variants_add_relational_attributes_and_layers) \| test(typed_attributes_keep_source_evidence_through_serving) \| test(typed_handoff_pair_support_survives_serving_without_becoming_a_call_chain) \| test(handoffs_and_usage_patterns_come_from_official_code) \| test(pass_c_and_usage_patterns_are_identical_across_location_and_module_order) \| test(serving_schema_digests_are_the_shared_known_answers) \| test(writes_the_python_fixture_generation)' --status-level fail --final-status-level fail` | `passed`: 29 cases, 181 skipped. Typed semantics, parallel edges, relabeling invariance, source/Delta/serving support and missing/foreign/member/edge/fact/pair refusals. `/tmp/lctx-stage3-typed-attributes-final.log`. |
| Earlier focused typed-attribute selections with `INSTA_UPDATE=new` | `failed`: initial duplicate SQL expressions, then unregistered `mod()` in member-shape rule; corrected. Expected schema/rendering snapshot differences were read and accepted as compiler-88 migration; sixth run was 26 passed and 2 snapshot failures before the final passing run. |
| `uv run --no-sync pytest python/lctx_mcp/tests/test_server.py python/lctx_mcp/tests/test_digest.py` | `passed`: 15 cases against the regenerated compiler-88 fixture, including Python/Rust known-answer schema agreement. `/tmp/lctx-stage3-typed-attributes-python.log`. |
| `uv run --no-sync pyrefly check python/lctx_mcp/src/lctx_mcp/generation.py python/lctx_mcp/src/lctx_mcp/server.py` | `passed`: 0 errors, 1 warning. `/tmp/lctx-stage3-typed-attributes-pyrefly.log`. |
| `just adr index`; `just adr lint` | `passed`: additive ADR-0059 and governed target references; implementation remains Proposed. |
| `uv run --no-sync python docs/design_review/evidence/2026-09-27_sync-contexts/source_probe.py` | `passed`: extractor interface observation; missing exit and inherited stub entry preserved, no generated Python input executed. |
| `just fmt`; `just test-all`; fresh `just pilot`; structured Q01/Q03/Q05/Q09; clean wheel; live W9; all-techniques/cost; `just docs-check` | `not_run`: integrated qualification awaits assembled S1–S7 functionality. |

## Next work and boundaries

- Continue S2b/S3a synchronous context entry/exit, ordered partial-entry cleanup and suppression/outcome replacement; resources/callbacks and named-handler cleanup remain open. Pinned model families, dynamic-schema activation and independent pure-model challenges remain open.
- S4/S5 retain stable multi-control/residual conjunction, all-channel SCC semantics with separate witness progress, actual callee/model coverage and claim-specific discharge. Typed current RCA is implemented; new behavioral FCA/registry stays Stage 4.
- S6/S7 retain FORMAT 9 semantic queries/full evidence, actual default-cap producer→Delta→native traces and independent compiler/oracle challenges. Stage 5 deferred execution remains outside scope.
- Finish S1–S7 before integrated S8 gates. Focused receipts certify only their named cases; no current full-suite or assembled Stage 3 receipt exists.
