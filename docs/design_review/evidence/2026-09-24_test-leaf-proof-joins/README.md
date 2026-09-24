# Test-leaf and entry-value proof join probe

**Question (2026-09-24).** Can one `flow_tests` row identify every Boolean leaf and
its operand use, including compound tests and distinct `match` arms? Can a source
guard be equated with an entry value across a call or write without a path proof?

**Method.** Read the pinned `cpg-flow` translator and fixture snapshot; query the
published FastMCP snapshot `ecf8b9cdc11ebaf4c1dae61fa9b35dee` through `lctx
query` (the snapshot's Delta/DataFusion reader); run `cargo test -p cpg-flow --test
flow_shapes --quiet` (**passed**, 28/28). These are existing Stage 2 facts and
source checks, not a Stage 3 proof-relation implementation.

1. The pilot's `fastmcp/resources/base.py` has a `flow_tests` row at byte span
   `11597..11833` whose condition contains distinct leaves at
   `11607..11670`, `11682..11699`, `11724..11751`, `11764..11773` and
   `11786..11827`. The inner test at `11724..11751` is
   `isinstance(raw_value,list)#…:s11724-11751`; its `raw_value` flow use is
   `11735..11744`. The enclosing test row alone does not select that leaf or
   operand. This is an **observed compound-test join ambiguity**, not evidence
   of an exact runtime type.
2. In the existing `matched(command)` fixture, both `case "start"` and `case None`
   test the same subject span. The pinned snapshot has separate synthetic
   `equals(command,"start")#…:p…` and `is_none(command)#…:p…` atoms in the
   two case regions. `cpg-flow/src/lib.rs` currently retains only the first
   `flow_tests` row for a span (`seen.insert(span)`). Therefore a later arm's
   atom can exist in the condition graph without appearing in the retained
   test row's support. The **proposed** `flow_test_leaves` relation must be
   emitted per predicate and atom before that deduplication.
3. Existing `alias_fallback`, `two_calls` and `nonlocal_change` fixtures show
   repeated sites, a repeated call and an intervening nonlocal write. The
   28/28 focused tests preserve their Stage 2 path conditions. They do **not**
   prove an entry-formal-to-guard identity transfer. The **proposed**
   `flow_test_value_links` producer must withhold a cross-site link through an
   unmodeled call or write, even if the place spelling or static type matches.

The pilot read-only queries were:

```sql
SELECT t.start_byte, t.end_byte, c.encoding
FROM flow_tests t
JOIN source_files s ON s.module_node_id = t.module_node_id
JOIN conditions c ON c.condition_id = t.condition_id
WHERE s.path = 'fastmcp/resources/base.py'
  AND t.start_byte >= 11597 AND t.end_byte <= 11833
ORDER BY t.start_byte LIMIT 6;

SELECT u.place, u.start_byte, u.end_byte
FROM flow_uses u
JOIN source_files s ON s.module_node_id = u.module_node_id
WHERE s.path = 'fastmcp/resources/base.py'
  AND u.start_byte >= 11597 AND u.end_byte <= 11833 AND NOT u.annotation
ORDER BY u.start_byte, u.end_byte;
```

**Design consequence.** `flow_test_leaves` owns leaf attribution; `flow_tests`
remains a span-level projection. `flow_test_types` and `flow_test_value_links`
cite the leaf row plus the exact use and return no positive proof if a synthetic
pattern's subject-to-use mapping is ambiguous. The new relation, validator and
query bridge are **Proposed**, not tested by this probe.
