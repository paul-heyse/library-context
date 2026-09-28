# PR0–PR1 catalog and optional-enrichment qualification

Status: **PR1 Implemented, Tested and deployed; PR0 comparative admission blocked, 2026-09-28.**
The [forward plan](../../../plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition)
owns finding disposition; [ADR-0072](../../../adr/0072-catalog-compile-profiles.md)
owns the decision. This evidence does not qualify PR2–PR6 or comparative superiority.

## Functional boundary

Catalog is the default compile profile. Ordered source/provider contracts, candidate bindings,
constructor associations and pinned source/type evidence travel through canonical Delta,
bundle13/projection3, migration009, Rust PostgreSQL hydration and FastMCP. Behavioral compilation
adds the retained analysis and brief capabilities. Unselected native artifacts are absent and
native initialization is conditional. Existing semantic contracts and future additive scope remain.

The evaluation runner uses stateless API function calls with explicit condition tools. The frozen
24-task development population and independently sealed 24-task candidate remain separate.
Twelve final A/B/C admission attempts at `build/product-eval/pr0-pr1-final-smoke/` are **blocked**
by unestablished FastMCP4.0.5 Context7 material parity. No model call, answer or score was produced;
confirmation was not executed. Candidate sealing does not repair its disclosed late timing or
establish independent distinctness/parity admission.

## Recovery controls

Protected assets stay in ignored operator storage; no database dumps or credentials are committed.
The baseline `build/postgresql-pr1-baseline-a6c9fcf/` retains the exact old CLI, Python/native packages,
recovery scripts, pins and a SHA-256 receipt. It contains the schema008 pre-cutover backup.

| Command / receipt | Result, 2026-09-28 |
|---|---|
| Retained runtime `scripts/postgres_backup.py backup .../pre-cutover.dump`, with retained Python packages on `PYTHONPATH` | **passed**: 69 tables, two ready generations; 11.71 s |
| Same retained script/runtime `restore-drill .../pre-cutover.dump`; `/tmp/lctx-pr1-legacy-restore.log` | **passed**: both legacy generations served with native analysis; 10.71 s |
| `uv run python docs/design_review/evidence/2026-09-28_catalog/recovery_probe.py build/pr1-mixed-recovery-v2`; receipt in that output directory | **passed**: disposable008→009 migration; all four generations preserved; new catalog and behavioral generations served; selected legacy refused; corrupted legacy artifact refused; restore 7.34 s |

Current readers admit only current formats. Backup inventory preserves known legacy transport
manifests and artifacts without pretending current readers can decode them. A selected legacy
pointer explicitly refuses usable current recovery. Old-runtime rollback restores the separate
schema008 backup; it does not weaken current database compatibility checks.

The first mixed rehearsal found an outage-drill primary-key collision when a file had multiple
registered locations. The corrected drill hashes each original location in its unavailable path;
the complete rerun above passed. Failed assets remain under `build/pr1-mixed-recovery/`.

## Focused receipts and remaining qualification

Catalog, corpus profile, candidates/generated/inherited constructors, singleton composition,
capability closure, source tampering and real PG/FastMCP controls passed. The bounded
[PR1 review](../../reviews/design_review_pr1-implementation_2026-09-28.md) lists precise commands and
scope; the [PR0 review](../../reviews/design_review_pr0-evaluation_2026-09-28.md) records 12 local
runner/original-evidence controls and the live comparison boundary.

## Real-library profiles

`uv run python docs/design_review/evidence/2026-09-28_catalog/pilot_probe.py build/pr1-pilots-v3`
**passed (2026-09-28)**. Each fresh FastMCP4.0.5 compile published through Delta and imported,
selected and served in disposable PostgreSQL. Each smoke fetched three catalog packets;
behavioral smoke additionally searched/hydrated all twenty briefs. All profiles expose 1,534 operations.
Fake-vector runs test mechanics; the two `vllm` runs use the controlled pinned Qwen8B1024 endpoint.

| Profile / embedder | Projection generation | Compile/import/serve seconds |
|---|---|---|
| catalog / fake | `765e57f70f3aa9bfe390eccf3c24bdd1207f97dec11778319f78ebaa4f8fea33` |119.61 |
| behavioral / fake | `aa9ce771273ee1418d305c15d15ff251ea7fc6fb5ef0f822914bfa468825c2ac` |253.49 |
| catalog / vllm | `115529089522683876f095aa4b21819938aab138260ecb43fdbcc5f522e5193a` |148.07 |
| behavioral / vllm | `2081c10e6ef2e803c832b19e04f595a2ff6cf0f26822461ac89a1122b9823ab4` |281.01 |

These are observations under concurrent compilation/tests, not comparative performance measurements.
`receipt.json` records identities/capabilities; `restore.json` records a **passed** 78-table,
four-generation restore in 24.69 s, including current serving and the captured behavioral/live selection.
Native initialization is absent for catalog and present for behavioral.

The live pair's nine catalog relations also passed a multiset contract comparison:
5,097 members, 6,126 bindings, 1,360 signatures, 5,139 parameters, 333 constructor associations,
1,586 source-evidence rows, 1,414 types, 1,555 type arguments and 5,198 type observations.
`catalog-contract-parity.json` records each excluded column: run-qualified fact references,
binding/evidence IDs derived from those references, and optional brief status/reason. Stable member,
declaration, signature and type IDs, source bytes/ranges/digests, ordinals, kinds and defaults remain
in the comparison. Direct row equality (`catalog-parity.json`) failed as expected because fact
identity includes the extraction run and changes sort order; it is not the contract comparison.

The pilot used build7. Build8 subsequently added mandatory raw-condition/type-link admission to
the catalog path and corrected proof-step condition references to the already hydrated union of
raw and analysis conditions. Neither changes emitted valid pilot rows. The final runtime reverified,
imported, reconciled and served both live artifacts at cutover. These receipts do not claim a fresh
build8 full-library compile; final-runtime canonical controls also passed in the full gate.

Final formatting ran after functional implementation. The code gate exposed additional
contract/fixture migrations: snapshots were inspected before acceptance, semantic fixture tests
now request behavioral analysis explicitly, and receiver/error assertions reflect the catalog contract.
The final `just test-all` **passed** without failure overrides: 446 Rust tests,184 Python tests,
18 real PostgreSQL Rust tests and2 PostgreSQL Python tests, plus format/lint/Pyrefly/rules,
38-record ADR/agent checks, 86 parsed fixtures, dependency policy/gold and SQLx metadata checks.
The complete gate log and counts are retained in `build/pr1-operator-cutover/gates/`.
Earlier failures and snapshot-generation runs are diagnostic evidence, not successful qualification.
No wheel was built; native modules use the existing editable environment.
`just docs-check` passed163 canonical pages with zero offline link errors; `just adr index` and
`just adr lint` passed with38 records. These document checks do not establish comparative value.

## Operator cutover and final recovery

**Passed, 2026-09-28.** The final runtime ran `target/release/lctx db migrate`, then `db check`.
For each live bundle above it ran `serving import-bundle --bundle BUNDLE`, then
`serving reconcile --generation DIGEST`. It explicitly selected the behavioral/live digest with
`serving select --library fastmcp --generation DIGEST`, using the supported exact default.
Both `uv run python -m lctx_mcp.smoke BUNDLE --embedder vllm` runs and artifact-verifying diagnostics
passed against the operator database. `build/pr1-operator-cutover/receipt.json` records the CLI hash,
identities and every step; `database-status.log` reports PG18.6 and a current schema, and serving
diagnostics report pgvector0.8.6, ready/available generations and exact admission.

| Command | Result |
|---|---|
| `uv run python scripts/postgres_backup.py backup build/pr1-operator-cutover/current.dump` | **passed:** receipt3, 78 tables, four ready generations, 18.14 s |
| `uv run python scripts/postgres_backup.py restore-drill build/pr1-operator-cutover/current.dump` | **passed:** all four preserved; both current profiles served with native absent/present; exact enriched selection restored; 12.49 s end-to-end, within 900 s local objective |
| `uv run python docs/design_review/evidence/2026-09-28_catalog/compare_profiles.py build/pr1-pilots-v3` | **passed:** all nine contract multisets; reproducible output in `catalog-contract-parity-final.json` |

The two legacy generations remain byte/artifact preserved with `legacy_runtime_required`;
serving them with the new runtime is **not_run**, deliberately unsupported. Their separately
restored schema008/matching-runtime receipt is above. Current recovery is not old-runtime rollback.
The temporary controlled embedding service used for these live checks is stopped after completion.

AP/F01 closure is limited to mandatory catalog and optional capability admission. PR2–PR6 and
broader semantic completion remain open; no comparative score or differentiation claim follows.
