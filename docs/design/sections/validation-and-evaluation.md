# Validation and evaluation

**Implemented graph-native validation, 2026-10-06 (ADR-0128); scoped evidence remains explicit.**
The [semantic owner](semantic-model.md) supplies typed declarations and shared validators. The
[graph-native coordinator](../../plans/graph-native-pivot-plan_2026-10-05.md) owns current commands,
package acceptance and native publication/serving dependencies. Earlier PostgreSQL receipts
qualify only their original tree and boundary.

This owner states what the system checks and how answers are judged. **§8** covers admission and
independent oracles; **§12** covers usefulness and correctness on registered questions. Compiler
checks consume completed typed Arrow streams and typed graph records. Native publication and
serving enforce the same declarations against selected admitted content. Neither validation
nor evaluation writes facts. Shared semantic validators live in `lctx-model::domain`; `cpg-core`
invokes the applicable necessary support/qualification/ownership, structural, reference, outcome
and derivation checks. Diagnostic producer replay is explicitly separate from production admission. Independent generated
runtime controls live in `tests/scripts/test_flow_soundness.py` and
`tests/scripts/test_semantic_soundness.py`; the latter observation bundle needs a native served
comparison before it can qualify replacement serving. Product evaluation remains separate work.

## §8 Validation

> Decision: ADR-0130

**Accepted target.** Model declarations own structural and reference checks. Targeted independent
known-answer and invalid-row controls challenge semantic operations. Admission retains exact
producer outcomes and declared derivation topology; it does not rerun every compiler algorithm.
Native serving requires its own acceptance evidence; prior publication tests do not establish fidelity.

**Accepted wire/catalog verification target, implementation Proposed (ADR-0073).** Rust
`jsonschema` validators independently challenge Schemars schemas against typed request decoding
and emitted packet JSON, with an explicit draft/format policy and offline-only approved references.
Cover unknown fields/variants, missing/null, defaults, Unicode lengths, IDs, integer/boolean/coercion
and input/output union differences. Exercise real FastMCP listing and calls; function-only checks
cannot certify transport parity. Shared runtime/publication validators still enforce generation
membership, relational integrity, coverage and evidence closure; schema validity cannot prove them.
Use existing `insta`/`proptest` and meaningful cross-module ID/state compile-fail cases (`trybuild`
when a dedicated harness is justified). Pure catalog known answers should not need acquisition,
embedding or a database. Coarse and optional fine-grained reuse compare complete clean/reused
outputs, including insert/delete, missing evidence, source coordinates and policy changes.
[Forward-plan §3.0/§6.2](../../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns timing and CLF closure. These controls do not replace PR6's independent product comparison.

> Decision: ADR-0073

**Local** (Arrow/Rust), at every materialization boundary: exact physical types, nullability and
widths (`RecordBatch::try_new` against the declared schema); codebook membership; numeric bounds;
finite floats and unit-norm vectors.

**Compiler artifact admission** checks typed graph records, canonical identity/content,
reference namespace/kind/subtype closure, source bounds, required scoped outcomes, declared
acyclic derivations and exact consumed vector values. Captured originals remain byte-exact.
Arrow/portable bytes are transport rather than semantic identity. Exact completed native contributions/views
and retained state have their own identities. Pending or cancelled output cannot become admitted. Checked completed-input views are private
attempt-scoped authorities over immutable descriptors/profile/policy and exact selected vocabulary.
A detached artifact or restored dump receives fresh pure semantic admission from neutral typed
graph records and complete retained native state; its authored hashes or publication marker cannot establish validity.
Cold reconciliation compares full canonical/body/key/content agreement for graph and fixed non-graph
backing, and verifies original bytes independently. Ordinary compilation carries exact unchanged validity.

**Native publication** (Implemented, 2026-10-06) derives schema and indexes from
the model and enforces the artifact's contract before readiness or selection. No PostgreSQL path,
old-format reader or independent rule registry remains.

**Shared semantic controls** replay selected model-owned operations where a test or corruption
audit needs agreement. Pure stateless replay consumes explicit completed input streams and profile
premises. Missing coverage or undeclared input refuses; an explicitly unrequested profile domain
cannot establish absence. Independent hand-known expectations are separate from producer replay.

**Assurance policy (ADR-0126/0128).** During this stage, run focused affected model, provider,
compiler and analytics controls, then applicable leaves at functional completion. Assembled
`just qualify` remains the assembled assurance route. For this execution, the user selected
targeted actual native functional controls and stopped further compiler-stage tests; coordinator
§9/R-Q0 records the current remediation boundary; §8 preserves historical scoped receipts. The old PostgreSQL/MCP receipt is not replacement
acceptance. Quantitative performance claims require measurements.
Efficient design uses qualitative judgment, bounded streams, shared preparation and library
capabilities; runtime accounting or proof machinery is not an admission prerequisite.

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

> Decision: ADR-0143, ADR-0117, ADR-0015

### §8.1 Independent oracles (the validation lane)

**Current controls, Tested within the bounded receipts, 2026-10-05.** Instruments challenge one specific claim;
none writes facts or tunes analysis. Provider CLI agreement is parity, not independent semantics.

| Instrument | Claim and expectation source | Live control |
|---|---|---|
| Raw-flow runtime challenge | Independent bounded CPython observations challenge regions, reaching definitions and admitted flow over generated programs | `tests/scripts/test_flow_soundness.py` |
| Served-claim runtime challenge | Independent guards/returns challenge original served refutations and exact identity transfers | Observation bundle: `tests/scripts/test_semantic_soundness.py`; native served comparison pending S2/S3 |
| Finite analytics challenges | Authored incidence matrices, independent double derivation/concept enumeration and dev-only odis implication consequences | `crates/lctx-analytics/tests/implication_oracle.rs`; [analytics owner](analytics.md#section-9) |

The former Pysa TITO control is absent from the current tree. Historical Pysa output and CrossHair
`typing.cast`/`assert_type` specializations establish no current compiled/served coverage. They are
retired rather than restored for an instrument count. A future TITO claim requires matched-source,
independent bounded expectations; silence, obscure results, timeouts and unexplored paths remain
inconclusive. Observation-only bundle success is not served correctness.

**Boundaries.** Only generated programs execute; the analyzed library and `fixtures/python/`
never do, and nothing runs with network access. Oracles stay out of compiler inputs; the gold
stays out of analysis. The current runtime oracle checks raw flow over a small generated corpus;
its broader exclusions remain separate. The retained generated CPython observation bundle
challenges path-local refutations and exact identity transfers using independent guards and
returns. Its former PostgreSQL serving driver is retired. A native driver must compare actual
producer and served answers before claiming replacement fidelity. May-compatible answers never
establish identity; historical receipts retain their own scope.

**Implemented and focused Tested (2026-09-27):** the raw-flow runtime oracle builds and drives
this checkout's release `lctx` binary, overriding an inherited target directory. `uv run --no-sync
pytest tests/scripts/test_flow_soundness.py -q` passed 18 cases. This receipt covers raw flow and
regions; composed summary/coverage/serving challenges remain separately scoped Stage 3 work.

---

## §12 Evaluation

**Accepted** (ADR-0071) for the structured behavioral evaluation; gold scoring is
**Implemented** as a record of brief retrieval.

**Implemented private programmatic loop; selected runtime integration pending, 2026-10-06
(ADR-0130).** The offline `lctx-eval` crate, outside the production CLI/PyO3 dependency closure,
owns compatible All/Any/Exists witnesses, satisfiable-first finite worlds, explicit applicability
and operation statuses, and versioned experiments. The bounded long-lived JSONL worker and Python
development runner generate and shrink private cases against independently grounded expectations.
Empty or unsupported domains cannot report successful sufficiency.

The fixed observers independently decode exact final public MCP objects for selected operation,
original-evidence and search responses. Readable predicates, actual release/context/signature
containers and source attribution establish witnesses; producer delivery maps cannot authorize
truth. Separate conformance checks reject maps that disagree with actual delivered fields, source
ranges or bindings. Bounded navigation follows visible public references and records attempted
calls; private expected fields never enter those calls. Renderer controls establish local transport
and interpretation behavior only; actual native journeys require the rebuilt development extension
and an owned persistent fixture.

The stored-vector reference compares exhaustive full4096 and deliberate1024 projection with an
independent scalar F64 control and canonical ties. Supplied candidate-union diagnostics and controlled
stage injections identify observed losses without claiming production task success. A private
native replay adapter pins the exact published view through the private native owner, exhausts one declared eligible cohort
up to256 stored rows, and compares actual indexed HNSW nominations with those retained full bytes
without inference. Oversized or incomplete populations refuse rather than becoming exhaustive
references. Its selected runtime acceptance remains pending. Development optimization remains under frozen meanings; no production classification
acts as the expected-answer engine.

Outer agentic evaluation supplies independently grounded system improvements **and** evaluator
model/coverage/observation/judgment revisions. Agent success/failure alone is not truth. Freeze each
comparison's meanings/population; changed meanings establish an explicit revision/new baseline or
stratum, with same-packet rejudgment where applicable. [The dedicated plan](../../plans/programmatic-evaluation-plan_2026-10-06.md)
develops these contracts; [the combined coordinator](../../plans/evidence-retrieval-and-evaluation-plan_2026-10-06.md)
owns its findings. On 2026-10-06 the source-bound private worker's focused Python selection passed;
six selected actual renderer/MCP controls remain blocked by the stale development extension. The
dedicated plan records exact commands and scope. Pure finite/serializer work need not wait for live
inference or native acceptance.

Protected confirmation/gold/heldout never tune parameters and remain isolated; private truth never
enters production compilation, embeddings, retrieval or query prompts. Authorized development
optimization is distinct. Product usability/differentiation studies in [§14.12](api-and-evidence-product.md#section-14-12)
remain separately admitted external claims, not the primary engineering loop or a prerequisite for it.

> Decision: ADR-0130

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
