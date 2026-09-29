# Attachment index resource contract — bounded review

Independent reviewer Dalton, 2026-09-29; core 3.0, code-intelligence 1.1, repository binding.
Design/target review of model attachment construction/query buffers, reservation ownership and
failure cleanup. **Outcome: Accept, bounded; no findings.**

The sorted flat occurrence index reserves entries, interval maxima, copied paths and duplicate-ID
scratch before allocation. Sorting is in place. Query capacity is bounded by the smaller of the
matching group and the alternative limit; each entry is visited once, and pushes stop at that limit.
Ambiguous results retain their alternative reservation after the index is dropped. Other outcomes
free their buffers before releasing the charge. Overflow, validation failure, memory refusal and
duplicate-ID paths release owned reservations. Work exhaustion is BudgetExceeded; memory refusal is
an attempt error, not an empty answer. Caller-owned input batches remain separately charged.

A1 localizes buffer ownership in attachment and admission in the shared pool. A2 expresses reservation
lifetime through owning index/results. A3 reuses the common budget contract without another allocator.
The indexed answer continues to match the independent scalar oracle for admitted operations.

**Tested, 2026-09-29:** reviewer independently executed
`python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test domain
--test domain_resources`: passed, 17 domain and three resource controls. New controls cover concurrent
pool pressure, construction/query refusal, ambiguity lifetime, duplicate cleanup and work refusal.

This verifies requested attachment-buffer accounting, not process RSS, parser/runtime overhead,
validator/COPY totals or assembled P0-D resources. Broad gates were not_run. Current open resource
obligations remain in [cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition).
