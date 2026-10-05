# Validation and evaluation

**Implemented for canonical publication; Phase 5 serving qualification pending, 2026-10-02.**
[§15](semantic-model.md) owns the typed model, PostgreSQL generation store and declared stage
validation. The [cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md) records
completed layer receipts and the remaining serving boundary.

This owner states what the system checks and how its answers are judged. **§8** covers
publication validation: the rules every snapshot passes before it becomes visible, and the
independent oracles that challenge the analysis without feeding it. **§12** covers evaluation:
whether published answers are useful and correct on registered questions. Validation consumes
the Arrow contracts and derived tables of [facts and identity](facts-and-identity.md) and runs
inside publication ([§6.1](storage-and-publication.md#section-6-1)); evaluation consumes a
published generation through the serving tools ([§11.3](synthesis-and-serving.md#section-11-3)).
Neither writes facts. Declarations and shared validators live in `lctx-model::domain`;
`lctx-postgres` enforces their physical lowerings and publication receipts. `cpg-core` invokes
model-owned validation and replay before publication. Independent generated runtime controls live
in `tests/scripts/test_flow_soundness.py` and `tests/scripts/test_semantic_soundness.py`, `eval/behavior/` (question sets) and `scripts/gold_match.py` (retained preregistered matcher and source-span arithmetic). Stage exit rules and
open validation work are in the [forward plan](../../plans/behavioral-model-forward-plan_2026-09-24.md).

## §8 Validation

**Implemented / Tested within the recorded canonical-layer receipts.** Model declarations own
structural checks; pure validators and strict stage replay own semantic agreement. Independent
known-answer and invalid-row controls challenge each supported layer. A new serving consumer
requires its own acceptance evidence; prior publication tests do not establish served fidelity.

**Accepted wire/catalog verification target, implementation Proposed (ADR-0073).** Rust
`jsonschema` validators independently challenge Schemars schemas against typed request decoding
and emitted packet JSON, with an explicit draft/format policy and offline-only approved references.
Cover unknown fields/variants, missing/null, defaults, Unicode lengths, IDs, integer/boolean/coercion
and input/output union differences. Exercise real FastMCP listing and calls; function-only checks
cannot certify transport parity. Shared runtime/publication validators still enforce generation
membership, relational integrity, coverage and evidence closure; schema validity cannot prove them.
Use existing `insta`/`proptest` and meaningful cross-module ID/state compile-fail cases (`trybuild`
when a dedicated harness is justified). Pure catalog known answers should not need acquisition,
embedding or PostgreSQL. Coarse and optional fine-grained reuse compare complete clean/reused
outputs, including insert/delete, missing evidence, source coordinates and policy changes.
[Forward-plan §3.0/§6.2](../../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns timing and CLF closure. These controls do not replace PR6's independent product comparison.

> Decision: ADR-0073

**Local** (Arrow/Rust), at every materialization boundary: exact physical types, nullability and
widths (`RecordBatch::try_new` against the declared schema); codebook membership; numeric bounds;
finite floats and unit-norm vectors.

**Physical publication** (PostgreSQL) enforces generated nominal keys, references, nullability,
codebooks and declared checks. The lowering and physical admission inspect the same model;
there is no Delta write path or independent rule registry.

**Cross-relation validation and replay** use shared pure model validators, with DataFusion where
relational construction is appropriate. Completed runs retain the declared source relations,
epochs, content digests and qualification boundary. Publication verifies the exact closure;
strict consumers reconstruct meaning from those records. Missing coverage, mismatched receipt
content or an undeclared source refuses admission rather than becoming an empty result. Tests
challenge injected invalid rows and independent hand-known semantics separately from generated
structural checks.

**Accepted assurance pivot, implementation in progress (2026-10-04).** One model-owned definition
with stable ID/revision supplies each obligation's complete ordered premises. Relations and stage
uses reference it explicitly. Existing store receipts own complete acknowledged binding conclusions;
read-check receipts are grant evidence. A stable validation session shares compatible streams and
charged preparation, with bounded separate execution when order/state requires it. Binding identity
includes installation/generation, model/layout, definition, exact immutable source/prefix and relevant
semantic configuration. Missing, failed, partial or unconfirmed conclusions never authorize a hit.
Fresh consumer admission remains required. Normal reads trust owned immutability; explicit audit
recomputes physical integrity, nominal references and pure/publication semantics. The [coordinator](../../plans/testing-architecture-pivot-plan_2026-10-04.md)
owns qualification and F01–F04; structural work counts alone establish no measured speed benefit.

> Decision: ADR-0126

**Edit guards.** A lineage rule that re-reads its edge kind's own unfiltered source cannot fail on
the current derivation: it guards an edit, not a data condition. Such rules are declared
apart from falsifiable data checks. Current typed derivations and their retained source rows
are validated through their declared model owner.

**What validation cannot prove.** A validator that reconstructs a producer's output with the same
semantic input agrees with that producer, including its defects: equivalent-spelling access
defects ([plan W4](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition))
and historical lossy serving projections (W2, W3) illustrate that limitation. Independent semantic controls (§8.1,
real-provider fixtures, served round trips) are kept separate from derived validators.

**Rules of use**
- **Read-only.** Validators only read and reject; they never repair.
- **Shared.** Validators are library code used both by tests and before publication; there are
  no test-only copies.
- **Not delegated.** DataFusion `Constraints` are informational and never relied on.

**Determinism.** Every relation that is published, compared or snapshot-tested has a total
`ORDER BY`; every `row_number()` ends in a unique tie-break; every cast in validation code uses
`safe: false`.

> Decision: ADR-0086, ADR-0117, ADR-0015

### §8.1 Independent oracles (the validation lane)

**Current controls, Interface-checked 2026-10-04.** Instruments challenge one specific claim;
none writes facts or tunes analysis. Provider CLI agreement is parity, not independent semantics.

| Instrument | Claim and expectation source | Live control |
|---|---|---|
| Raw-flow runtime challenge | Independent bounded CPython observations challenge regions, reaching definitions and admitted flow over generated programs | `tests/scripts/test_flow_soundness.py` |
| Served-claim runtime challenge | Independent guards/returns challenge original served refutations and exact identity transfers | Observation bundle: `tests/scripts/test_semantic_soundness.py`; actual producer/store/served comparison: `crates/lctx/tests/serving_soundness.rs` |
| Finite analytics challenges | Authored incidence matrices, independent double derivation/concept enumeration and dev-only odis implication consequences | `crates/lctx-analytics/tests/implication_oracle.rs`; [analytics owner](analytics.md#section-9) |

The former Pysa TITO control is absent from the current tree. Historical Pysa output and CrossHair
`typing.cast`/`assert_type` specializations establish no current compiled/served coverage. They are
retired rather than restored for an instrument count. A future TITO claim requires matched-source,
independent bounded expectations; silence, obscure results, timeouts and unexplored paths remain
inconclusive. Observation-only bundle success is not served correctness.

**Boundaries.** Only generated programs execute; the analyzed library and `fixtures/python/`
never do, and nothing runs with network access. Oracles stay out of compiler inputs; the gold
stays out of analysis. The current runtime oracle checks raw flow over a small generated corpus;
its broader exclusions remain separate. The Phase 5 generated CPython oracle challenges
original served path-local refutations and exact identity transfers using independently observed
guards and returns. Its driver is `crates/lctx/tests/serving_soundness.rs`; actual qualification
is pending. May-compatible answers are never interpreted as established identity.

**Implemented and focused Tested (2026-09-27):** the raw-flow runtime oracle builds and drives
this checkout's release `lctx` binary, overriding an inherited target directory. `uv run --no-sync
pytest tests/scripts/test_flow_soundness.py -q` passed 18 cases. This receipt covers raw flow and
regions; composed summary/coverage/serving challenges remain separately scoped Stage 3 work.

---

## §12 Evaluation

**Accepted** (ADR-0071) for the structured behavioral evaluation; gold scoring is
**Implemented** as a record of brief retrieval.

**The product comparison** in [§14.12](api-and-evidence-product.md#section-14-12) is the primary
first-product usefulness/differentiation criterion (Proposed execution). It uses coding agents as
task performers, independent checks/review as judges, and matched Context7 plus internal evidence
baselines. Pilot usability alone is not comparative acceptance.

**The structured behavioral evaluation** remains a separate judgment of semantic correctness and
research progress, with its existing unanswered targets and truth criteria.
- **Question sets.** `eval/behavior/<library>-<release>.toml` holds representative questions a
  coding agent might ask. Each has atomic target items cited to the library's source lines and
  docs, including **negative** items an answer must not state.
- **Pre-registration.** A set is written from the library's own source and docs **before** any
  output of the stage it judges is read, and committed first. It is append-only: a changed item is
  a new item that `supersedes` the old one.
- **Assessment.** When the retained research evaluation is activated, it produces a packet with
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
evidence spans. Matcher version 2 (`scripts/gold_match.py`) resolves identity to the declaration node: a gold operation resolves by exact
path equality against served `public_paths`, with no fuzzy fallback; a class operation matches
only a brief seeded by that class; unresolved operations stay in the union as strings. Scores,
counted over all gold units: (a) node Jaccard per family; (b) hit@5 over every task alias, where a
hit is a returned brief whose seed is in the family's node set; (c) recall of gold evidence spans.
Every score records `matcher_version` and each alias's mode; a live run with a degraded alias is
`blocked`. The retired bundle-based packet/scoring/workload runners have no current consumer.
Current-model evaluation runners are deferred until the product/research evaluation is activated;
Phase 5 qualifies serving behavior and does not run those comparisons. The matcher, fusion
rule and brief-document template were registered before any rescore; none changes on the strength
of gold scores, only with a rationale independent of the gold. **Tested** over constructed rows (`tests/scripts/test_gold_match.py`).

**Ablation and the keep rule.** Technique defaults are decided by the §9.8 keep rule
([analytics](analytics.md#section-9-8)); the registered outcome is ADR-0020 (proposed).

**Retained research trigger (§B11), outside the product critical path.** `unresolved` slot counts
by brief section remain diagnostic. If the retained end-of-increment-5 evaluation attributes
failures to those slots, an ADR may add a local compile-time generation model under mechanical
grounding. That research evaluation is no first-product prerequisite; no generative pipeline or
query-path feature is selected by ADR-0071.

> Decision: ADR-0071, ADR-0117, ADR-0020
