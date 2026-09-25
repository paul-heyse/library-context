# Validation and evaluation

This owner states what the system checks and how its answers are judged. **§8** covers
publication validation: the rules every snapshot passes before it becomes visible, and the
independent oracles that challenge the analysis without feeding it. **§12** covers evaluation:
whether published answers are useful and correct on registered questions. Validation consumes
the Arrow contracts and derived tables of [facts and identity](facts-and-identity.md) and runs
inside publication ([§6.1](storage-and-publication.md#section-6-1)); evaluation consumes a
published generation through the serving tools ([§11.3](synthesis-and-serving.md#section-11-3)).
Neither writes facts. Declarations live in `cpg_schema::rules` (rule generation and edit guards),
`cpg-core` (`validate`, shared by tests and publication), `tests/scripts/test_flow_soundness.py`
(runtime oracle), `eval/behavior/` (question sets) and `scripts/structured_eval.py`,
`scripts/score_gold.py`, `scripts/gold_match.py` (packets and gold scoring). Stage exit rules and
open validation work are in the [forward plan](../../plans/behavioral-model-forward-plan_2026-09-24.md).

## §8 Validation

**Implemented and Tested** for the local, per-row and cross-table rules below: every hand-written
rule rejects an injected violation or is a declared edit guard, and every generated template
(`key`, `ref`, `fact`, `fact-payload`, `codebook`, `endpoint`, `evidence`, `one-per-evidence`,
`no-parallel`, `lineage`, `id`) has at least one case;
`every_rule_is_exercised_or_declared_an_edit_guard` fails when a rule is added without either.
Rules for the behavioral, condition and summary tables are generated or hand-written the same
way and land with their tables.

**Local** (Arrow/Rust), at every materialization boundary: exact physical types, nullability and
widths (`RecordBatch::try_new` against the declared schema); codebook membership; numeric bounds;
finite floats and unit-norm vectors.

**Per-row, at every Delta write.** Invariants that never change (span order, non-negative
offsets) are also Delta CHECK constraints generated from the `cpg-schema` declarations and
enforced by `DeltaTable::write`; the open-time verify keeps them identical to the declarations, so
they are enforcement at the storage boundary, not a second definition. Codebook membership is
**not** a CHECK: codebooks grow append-only, and a stored range would reject the next code.

**Cross-table** (DataFusion), **one query per rule**, generated in `cpg_schema::rules` from the
contracts and snapshot-tested. Rules read the session's tables cached once in memory, run
concurrently and report violations in rule order (`violations_come_back_in_rule_order`;
`every_table_is_read_by_some_rule` walks every rule's plan):
- `key`: uniqueness of every table's declared total key;
- `ref`: each declared reference, as a `LEFT ANTI JOIN` returning zero rows;
- `fact`: every raw row has its `facts` row and every `facts` row its raw row;
- `codebook`: every codebook column holds a code of its codebook;
- `coverage`: a row for every declared family × module of the run's release;
- **graph**, generated from the registry
  ([§3.8](facts-and-identity.md#section-3-8)), so references and the mapping have one
  authority: endpoint kinds; node-valued references; evidence and support exist; `nodes`/`edges`
  keys; lineage from raw rows (each source row yields its declared edges, or its derived row
  carries a provider's reason); the `pysa_calls` partition; typed targets (a null target carries
  a provider reason, never a catch-all); and each Rust id recipe equals its SQL form;
- `semantic` (hand-written, one query each): provider/derivation agreement, injectivity of the
  Stage C mapping, producer and model attribution, stored source text and role, finding-status
  policy, witness chains, negative premises inside complete coverage
  (`semantic:refuted-needs-complete-region`), and the brief rules (every assertion cites
  findings and evidence of the same snapshot; every public symbol in a brief exists in `exports`).

**Edit guards.** A lineage rule that re-reads its edge kind's own unfiltered source cannot fail on
today's SQL: it guards an edit of that derivation, not a data condition. Such rules are declared
in `cpg_schema::rules::EDIT_GUARDS` and counted apart from falsifiable rules. A reference whose
target is built from its own source column is never generated.

**What validation cannot prove.** A validator that reconstructs a producer's output with the same
semantic input agrees with that producer, including its defects: equivalent-spelling access
defects ([plan W4](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition))
and lossy serving projections (W2, W3) pass today's rules. Independent semantic controls (§8.1,
real-provider fixtures, served round trips) are kept separate from derived validators.

**Rules of use**
- **Read-only.** Validators only read and reject; they never repair.
- **Shared.** Validators are library code used both by tests and before publication; there are
  no test-only copies.
- **Not delegated.** DataFusion `Constraints` are informational and never relied on.

**Determinism.** Every relation that is published, compared or snapshot-tested has a total
`ORDER BY`; every `row_number()` ends in a unique tie-break; every cast in validation code uses
`safe: false`.

> Decision: ADR-0047, ADR-0046, ADR-0015

### §8.1 Independent oracles (the validation lane)

**Implemented and Tested in focused cases** for the runtime soundness oracle and the matched-source
Pysa control; CrossHair is **Tested** for single model specializations; the broadened comparison
is a **Proposed** Stage 3 order (forward plan §3.2, order 8).

Each instrument challenges one kind of claim and never writes facts or tunes the analysis
(providers observe, our rules conclude; oracles only disagree).

| Instrument | Checks | Where and how |
|---|---|---|
| Runtime soundness oracle | Within the stated model, every executed statement's region is admitted, every observed reaching definition is present and, as summaries grow, every observed value flow is admitted. A violation is a counterexample; a finite pass is evidence, not proof | Generated programs (a seed corpus of known translation shapes plus Hypothesis-generated variants) run on the pinned CPython 3.14.7 under `sys.monitoring` with a private tool id, bounded examples, temporary storage and timeouts, in an isolated worker. `tests/scripts/test_flow_soundness.py`, part of `just check` |
| CrossHair `diffbehavior` | A model of a pure callable agrees with the real function | Isolated worker with timeouts; only exhausted paths support equivalence; a timeout or unexplored path is inconclusive; a counterexample is replayed concretely before it becomes a fixture |
| Pysa (pyre-check 0.10.0, pinned Pyrefly binary) | Taint-in-taint-out models against `summary_flows` on identical source, with a real source→sink rule | Offline; Pysa silence or an `obscure` model is inconclusive; its call graph is Pyrefly's, so only its TITO result is independent. The matched-source control is `pysa_tito_control_uses_the_same_source_as_finite_summary_fixture` ([evidence](../../design_review/evidence/2026-09-25_pysa-tito-rule/README.md)) |

**Boundaries.** Only generated programs execute; the analyzed library and `fixtures/python/`
never do, and nothing runs with network access. Oracles stay out of compiler inputs; the gold
stays out of analysis. The current runtime oracle checks raw flow over a small generated corpus;
it does not yet challenge ignored predecessors, raise escapes or refutations, and kernel tests
are example-based (plan W11: truth-table kernel tests, Python-lowering controls and served
round trips).

---

## §12 Evaluation

**Accepted** (ADR-0021) for the structured behavioral evaluation; gold scoring is
**Implemented** as a record of brief retrieval.

**The structured behavioral evaluation** is the primary judgment of usefulness and correctness.
- **Question sets.** `eval/behavior/<library>-<release>.toml` holds representative questions a
  coding agent might ask. Each has atomic target items cited to the library's source lines and
  docs, including **negative** items an answer must not state.
- **Pre-registration.** A set is written from the library's own source and docs **before** any
  output of the stage it judges is read, and committed first. It is append-only: a changed item is
  a new item that `supersedes` the old one.
- **Assessment.** Each stage produces a packet (`just structured-eval <generation> <stage>`) with
  the targets, the tools' answers, the brief if any, and mechanical marks where a check is
  mechanical. The author assesses each item as present / partial / absent / incorrect /
  misleading, per item and never as a percentage, and the operator reviews it. No API agents
  evaluate content. Stage exit rules are in the forward plan (§5).
- **Mechanical known answers:** the `flow_shapes`, `behavior_shapes`, `analysis_shapes` and
  related fixtures, plus an injected violation for every new rule.
- **The held-out check** is `eval/heldout/` (manifest-verified), sealed until the end of
  increment 5, when targets are written before anything runs.

**Gold scoring** ([§1.4](../DESIGN.md#section-1-4)) records brief retrieval against the
`fastmcp` skill's reviewed families, which are evaluation-only and never tune parameters. The
extract `eval/gold/fastmcp-4.0.5.json` records each family's operations, task aliases and static
evidence spans. Matcher version 2 (`scripts/gold_match.py`, shared by `score_gold.py` and
`ranking_check.py`) resolves identity to the declaration node: a gold operation resolves by exact
path equality against served `public_paths`, with no fuzzy fallback; a class operation matches
only a brief seeded by that class; unresolved operations stay in the union as strings. Scores,
counted over all gold units: (a) node Jaccard per family; (b) hit@5 over every task alias, where a
hit is a returned brief whose seed is in the family's node set; (c) recall of gold evidence spans.
Every score records `matcher_version` and each alias's mode; a live run with a degraded alias is
`blocked`. `just ranking-check <generation> vllm` is the §1.5 retrieval check. The matcher, fusion
rule and brief-document template were registered before any rescore; none changes on the strength
of gold scores, only with a rationale independent of the gold. **Tested** over constructed rows (`tests/scripts/test_gold_match.py`).

**Ablation and the keep rule.** Technique defaults are decided by the §9.8 keep rule
([analytics](analytics.md#section-9-8)); the registered outcome is ADR-0020 (proposed).

**Generative-model trigger (§B11).** `unresolved` slot counts by brief section are reported; if the
end-of-increment-5 evaluation attributes failures to those slots, an ADR may add a local
compile-time generation model under mechanical grounding.

> Decision: ADR-0021, ADR-0046, ADR-0020
