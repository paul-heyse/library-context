# Validation and evaluation

<!-- owner-intro -->

## §8 Validation

**Implemented** for the cross-table rules below and **Tested** (slice 2, 2026-09-22; C6 review
F1, 2026-09-23): every hand-written rule rejects an injected violation or is a declared edit
guard (below), and every generated template (`key`, `ref`, `fact`, `fact-payload`, `codebook`,
`endpoint`, `evidence`, `one-per-evidence`, `no-parallel`, `lineage`) has at least one case;
`every_rule_is_exercised_or_declared_an_edit_guard` fails when a rule is added without either.
The brief rules are **Proposed**. Source: IP L1581–L1601.

**Local** (Arrow/Rust), at every materialization boundary:
- exact physical types, nullability and widths. `RecordBatch::try_new` with default options
  enforces these against the declared schema (§4.3);
- codebook membership;
- numeric bounds;
- finite floats and unit-norm vectors.

**Per-row, at every Delta write.** Invariants that never change (span order, non-negative
offsets) are also Delta CHECK constraints, generated from the `cpg-schema` declarations and
enforced by `DeltaTable::write` (§4.3). The open-time verify keeps them identical to the
declarations, so they are enforcement at the storage boundary, not a second definition.
**Codebook membership is not a CHECK:** codebooks grow append-only, and a stored range would
reject the next code. The local validators above check it against the current codebook.

**Cross-table** (DataFusion), **one query per rule**, generated in `cpg_schema::rules` from the
contracts and snapshot-tested; the rules read the session's tables cached once in memory and run
concurrently, their violations reported in rule order (§4.3; `violations_come_back_in_rule_order`;
`every_table_is_read_by_some_rule` walks every rule's plan):
- `key`: uniqueness of every table's declared total key via `GROUP BY … HAVING count(*) > 1`;
- `ref`: each declared reference via `LEFT ANTI JOIN` returning zero rows;
- `fact`: every raw row has its `facts` row and every `facts` row its raw row;
- `codebook`: every codebook column holds a code of its codebook (a query, since not a CHECK);
- `coverage`: a row for every declared family × module of the run's release, and every declared
  family is in the codebook;
- `semantic` (hand-written, one query each): every Pysa function with signatures is some
  signature row's callable; a
  `resolutions` reason and the extractor's call boundary agree per call site, both ways;
  Stage C is injective; `parameter_semantics` names an existing Pysa function; `facts.model_id`
  names its run's producer; a module's stored text is its bytes, and its role matches its run
  (ADR-0015);
- **graph** (C1, **Implemented** and **Tested**; ADR-0014), generated from the registry (§3.8), so
  the references and the mapping are one authority:
  - endpoint kinds;
  - node-valued references (from `node_columns`);
  - evidence and support exist;
  - `key:nodes`/`key:edges`;
  - lineage from raw rows (each source row yields its declared edges, or its derived row carries
    a provider's reason);
  - the partition of `pysa_calls` (lineage or counted remainder; the published-gap arm is empty
    since C3);
  - typed targets: a null target carries a reason, and no derivation supplies a catch-all one
    (this replaces slice 2's "a release call target names a declaration or gives a reason");
  - ids: each Rust recipe equals its SQL form.

  **Edit guards** (C3 review O2; C6 review F1). A lineage rule that re-reads its edge kind's own
  unfiltered source cannot fail on today's SQL: it guards an edit of that edge's derivation, not
  a data condition. Eighteen rules are such guards, declared in `cpg_schema::rules::EDIT_GUARDS`
  and counted apart: the lineage of `declares`, `has_parameter`, `encloses_call`,
  `has_argument`, `ast_child`, `owns_scope`, `lexical_parent`, `binds`, `reads_binding`,
  `captures`, `declared_in`, `has_type`, `type_arg`, `has_field`, `field_type`,
  `contains_passage` and `contains_block`, and `partition:pysa_calls-gaps` (its table is empty
  by construction since C3). The other 15 lineage rules compare a filtered or joined source with
  the edges, and each can fail. A reference whose target is built from its own source column is
  not generated. **Count** (2026-09-23, `rules().len()`): 496 rules, 478 of them falsifiable;
- every assertion cites existing findings and evidence;
- every public symbol in a brief exists in `exports`.

**Rules**
- **Read-only.** Validators only read and reject; they never repair.
- **Shared.** Validators are library code, used both by tests and before publication. There are
  no test-only copies.
- **Not delegated.** DataFusion `Constraints` are informational and are not enforced, so they are
  never relied on.

**Determinism**
- Every relation that is published, compared or snapshot-tested has a **total `ORDER BY`**.
- Every `row_number()` ends in a unique tie-break.
- Every cast in validation code uses `safe: false`.

> Decision: ADR-0014, ADR-0012, ADR-0015

---

## §12 Evaluation

**Proposed.** Source: IP L2116–L2131, L3026–L3072.

**Fixtures.** The IP L3030 table, plus:
- guard after parameter rebinding;
- shuffled input gives identical communities and concepts;
- LFR planted-partition graphs for community detection;
- FCA over a hand-computed context.

**Gold scoring** (§1.4). The gold and the analyzed release are one FastMCP version, guarded by
`scripts/check_gold.py`. A small committed extract of the gold families records each family's
`authoring_sha256`, `operations`, `task_aliases` and static-evidence spans. Scores:
- (a) best-match Jaccard between each gold family's `operations` and the public members of the
  compiled briefs;
- (b) hit@5 of `search_capabilities` on the gold `task_aliases`. It was the pre-registered
  **development metric** for ADR-0020's keep rule. Since ADR-0021 it is a record of brief
  retrieval (§9.8), and it is never used for parameter tuning;
- (c) recall of gold static-evidence spans by compiled evidence.

**The matcher, version 2** (pre-registered 2026-09-24 in ADR-0010's amendment, before any rescore;
the holistic assessment's A1, the ADR-0020 review's F1 and F9). Identity is the declaration node:
a gold operation resolves by exact path equality against the served `public_paths`, with no
fuzzy fallback, and a class operation matches only a brief seeded by that class. (a) is node
Jaccard, where a brief contributes its seed and unresolved operations stay in the union as
strings. (b) searches every task alias, and a hit is a returned brief whose seed is in the
family's node set. (c) is unchanged. All three are counted over all gold units (22 families, 44
aliases, 157 spans). A brief's lexical text names its seed's **own** spellings, each distinct
**name** token once (R2 F1, registered 2026-09-24 before any score); promotion matches any public
spelling of the seed; and each score records `matcher_version`. Version 1 compared access-path
strings; the scores below are version 1's. **Implemented** (2026-09-24): `scripts/gold_match.py`,
shared by `score_gold.py` and `ranking_check.py`, reads the served `public_paths` (bundle
`FORMAT` 2); each alias records its mode, degraded reason and ranked hits, and a live run with a
degraded alias is `blocked` (exit 2); `just score <generation> [embedder]`. Tested over
constructed rows (`tests/scripts/test_gold_match.py`: an inherited spelling, a class operation,
unresolved operations, a degraded live run).

**Ablation** (§9.8). Diff the published output with each technique disabled, apply the keep
rule, and record the result in the increment-3 review. **Done in slice 3.3** (2026-09-23): the
table and the keep decisions are in §9.8 and ADR-0020.

**Gold scoring, Measured** (2026-09-23, `scripts/score_gold.py`, live vectors, budget 20): the
old default scored (a) 0.0444 over 22 families, 8 touched; (b) hit@5 14 and hit@1 7 of 44
aliases; (c) 4 of 157 spans. The kept default scores (a) 0.0430, 7 touched; (b) hit@5 13 and
hit@1 9; (c) 5 of 157. 19 of the gold's 125 operations are outside the release (`mcp`,
`mcp_types`, `fastmcp_tasks`, `pydantic`, `starlette`, `uncalled_for`), so no brief can name
them; the scorer reports them apart, and they count against (a).

**The structured behavioral evaluation** (ADR-0021; operator decision 2026-09-23; **Proposed**).
- **Question sets.** `eval/behavior/<library>-<release>.toml` holds representative questions. Each
  question has target items: atomic claims cited to the library's source lines and docs, including
  **negative** items that an answer must not state.
- **Pre-registration.** A set is written from the library's own source and docs **before** any
  output of the stage it judges is read, and committed first. It is append-only: a changed item is
  a new item that `supersedes` the old one.
- **Assessment.** Each stage produces a packet with the target, the tools' answers, the brief if
  any, and mechanical marks where a check is mechanical. It is assessed per item as present /
  partial / absent / incorrect / misleading, in
  `docs/design_review/reviews/structured_eval_<date>.md`, and the operator reviews it.
- **No API agents** evaluate content.
- **Mechanical known answers:** the `flow_shapes` and `behavior_shapes` fixtures, and injected
  violations for every new rule.
- **The held-out check** is `eval/heldout/`, sealed until the end of increment 5.

**Agent evaluation** (increment 5). *Superseded by the structured evaluation above (operator,
2026-09-23; ADR-0021); kept as the record of what was planned:*
- About 20 held-out task prompts and 5–10 usage fixtures.
- Raw-evidence retrieval is compared against compiled briefs under similar context budgets.
- Questions: was the right built-in capability found; did the agent discover the important
  control; did it use a supported handoff; did it respect the limits; did it avoid rebuilding
  library behavior?

**LLM trigger (§B11).** After increment 3, `unresolved` slot counts by brief section are reported.
If the increment-5 evaluation attributes failures to those slots, an ADR adds a local generation
model under §10.4 grounding.

> Decision: ADR-0021, ADR-0013, ADR-0020

---
