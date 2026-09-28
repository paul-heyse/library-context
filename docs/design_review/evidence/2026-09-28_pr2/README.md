# PR2 surface specificity and shared contracts

Status: **Implemented, Tested and deployed, 2026-09-28.** The
[forward plan](../../../plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition)
owns finding disposition. [ADR-0074](../../../adr/0074-catalog-surface-associations.md) owns the
bounded surface and configuration decision; ADR-0073 owns the wire/derivation boundary.

## Implemented boundary

Rust owns all six existing MCP requests, complete nested responses and the capability-resource
request. Generated Schemars draft2020-12 schemas separate input/output; serde decoding enforces
finite request vocabularies, nominal hex IDs, Unicode scalar limits and explicit required/null
behavior. JSON Schema's mathematical integer accepts `1.0`; strict integer serde decoding refuses
that spelling, as the shared fixture records. Final normalized output budgets apply after tagging
and default insertion. Native normalization detaches from Python; cancellation retains worker
capacity until the admitted computation finishes.

The canonical additions are record-field syntax, ordered surface aspects, configuration declarations
and exact field associations. Source defaults and factories are never evaluated. Shared MRO/shadowing
and descriptor admission replace independent classifiers. Constructor storage proofs refuse hooks,
custom allocation/metaclasses, descriptor fields, opaque decoration and unmodeled effects. Reader
links do not prove survival through intervening mutation. Existing optional behavioral scope remains.

`CatalogFacts` and immutable `PreparedCatalog` separate loading/indexing from pure derivation and
materialization/embedding. Existing table dependencies name scanned relations. No production
incremental engine, new cache or heap analysis is introduced. Versions are compiler109, extractor35,
template21, bundle14/projection4 and migration010. Authored behavior-model catalog FORMAT7 is unchanged.

## Library experiments

`salsa_catalog_probe.rs` is a Cargo example over an already published pinned fact snapshot. It
compares clean and prepared derivation with Salsa 0.28.2, including the same full-output comparison
encoding/hash work in timed queries. Initial, unchanged, narrowed, expanded, restored and absent-root
cases retain every evidence field. The Debug encoding is probe-local, never a publication format.
The experiment does not qualify source/environment changes, production persistence or a speed claim.

`persistence-probe` is an isolated workspace. It pins the complete Salsa macro family to 0.28.2;
a floating macro helper patch did not compile against that family. Persistence preserves the original
serialization string inside a version/source envelope. Reordering ingredient JSON through a generic
map caused a restore panic and is not supported by this probe. Owner-envelope incompatibility is
refused before Salsa deserialization. This is a disposable DTO experiment, not serialization of ty
or production catalog state. Ascent remains the S4 recursive-summary comparison.

**Tested and Measured, 2026-09-28:** all eight roots cases matched clean and prepared output.
The [roots receipt](raw/salsa-roots.json) records 31 µs index preparation, unchanged memo reads
at 0–1 µs clock resolution, and changed-root execution at 170–1861 µs for this small fixture.
A redundant setter reran the catalog query; equal output allowed the dependent digest to validate.
These single-process observations are not an end-to-end speed comparison or a scaling result.
The [persistence receipt](raw/salsa-persistence.json) records a 478-byte envelope, no persisted-query
execution after restore, one transient-query execution and rejection of changed producer/source
identity. Ordered serialized ingredient data must remain intact.

## Qualification commands

During functional work, only compile checks and focused Rust/native/FastMCP/PG controls ran.
`pilot_probe.py`, `compare_profiles.py` and `recovery_probe.py` own the final real-library,
profile-equivalence and mixed-format recovery exercises. Protected credentials, database backups,
artifact copies and runtime binaries stay in ignored `build/` storage. Final receipts below followed
the complete functional implementation; no comparative Context7 superiority is claimed.

**Tested, 2026-09-28:** `just fmt` and the complete `just test-all` **passed**. The
[complete gate log](raw/test-all.log) records 458 Rust tests, the two generated-fixture legs,
188 Python tests, 19 real-PG Rust tests and two PG Python tests. It includes fmt-check, strict
Clippy/Ruff, Pyrefly, seven rule suites, 40-record ADR/agent checks, 86 fixture parses, dependency
policy/fork/shear, gold and SQLx metadata checked against a freshly migrated database.
Nominal-ID positive/compile-fail doctests and focused schema/native/MCP/pure-input controls also
passed. The two general Python PG skips were exercised by the explicit PG leg; the saved-reference
parity test remains a separate gate and was not selected by this PR2 command.

Earlier attempts failed on large-enum Clippy findings, stale version/snapshot/fixture expectations,
a misplaced capability-request check and the Delta Boolean-equivalence constraint described by
PR2/F05. Corrections and affected tests passed before the successful complete run. The output
snapshots were inspected before acceptance; the final gate used `INSTA_UPDATE=no`.

The operator's concurrent Cargo optimization-profile edits began after this run's full-workspace
Rust leg. Later catalog/CLI/PG/SQLx builds used the changed settings; this log qualifies the
unchanged PR2 source, not a controlled performance comparison or a complete rerun under one new
environment policy. Those environment edits are outside the PR2 commit. Live timings and recovery
identify their actual artifacts separately below.

`just adr index`, `just adr lint` and `just docs-check` **passed** on an isolated export of the
staged PR2 tree: 40 ADR records, 169 canonical pages and zero offline link errors. The final
shared-tree attempt encountered the operator's in-progress ADR-0026→0075 environment transition;
those concurrent edits were left untouched and excluded from the export. No product build or
Cargo target moved for this documentation check. `git diff --check` and staged patch whitespace
checks passed. The final documentation command log is retained as `raw/docs-check.log`.


## Preserved baseline

`build/pr2-baseline-8334c86/source/` holds source from 8334c86, a reconstructed schema009 CLI
(the original executable was absent), preserved pre-change native modules, Python/recovery code,
a runtime checksum receipt and protected `pre-cutover.dump` with artifacts. The fastdev environment
supplies dependencies; no wheel was built. The baseline build reused the main Cargo target only
for this isolated recovery reconstruction, then its CLI was copied into the retained directory.

`postgres_backup.py backup` **passed** for 78 tables and four ready generations in 18.45 s.
The matching retained-runtime `restore-drill` **passed**, 2026-09-28: all four generations and
artifacts survived, both prior current profiles served, and the selected behavioral generation was
preserved; 14.39 s total. Older formats were preserved with an explicit legacy-runtime requirement.
Logs are `build/pr2-baseline-backup.log` and `build/pr2-baseline-restore.log`. No reverse migration
or reader-compatibility bypass is used.

## Current-format recovery

`uv run python docs/design_review/evidence/2026-09-28_pr2/recovery_probe.py build/pr2-mixed-recovery`
**passed**, 2026-09-28. The [receipt](raw/mixed-recovery.json) records disposable schema009→010,
81 restored tables and all six retained/current fixture generations. Both current profiles served
through native/PG/MCP; all four legacy generations retained their exact manifests and artifacts.
The 10.40-second restore refused an incompatible selected legacy generation and a corrupted legacy
artifact. No operator database was modified by this rehearsal.

## Real-library profile qualification

**Tested and Measured, 2026-09-28:**
`uv run python docs/design_review/evidence/2026-09-28_pr2/pilot_probe.py build/pr2-pilots-v2`
passed all four fresh FastMCP4.0.5 combinations of catalog/behavioral and fake/live vectors.
The fake-vector legs qualify mechanics only. [Receipts](raw/pilots.json) include compile, import,
explicit selection and stdio MCP: live catalog 114.39 s, live behavioral 215.76 s. Each exposes
1,534 operations; catalog has 5,097 public members, 307 ordered surface observations, 644 fields
and 591 links (432 declaration-only, 136 exact-storage, 23 reader associations). Reader links are
not temporal value proofs. The controlled vLLM service uses the unchanged pinned 1024 specification.

`compare_profiles.py build/pr2-pilots-v2` **passed** for all
[twelve catalog relation multisets](raw/profile-parity.json), preserving multiplicity, original
source bytes, types, member/signature/field identities and ordered semantic slots. Exclusions are
explicit run-qualified citations/bindings/link identities and optional brief state, not source content.
The [four-generation restore](raw/pilot-restore.json) passed all 81 tables, both profiles and the
captured exact enriched selection in 21.45 s. These results do not complete general Stage3 semantics,
qualify ANN or establish superiority over Context7.

## Operator cutover and retained assets

**Implemented and Tested, 2026-09-28:** explicit `lctx db migrate`/`db check`, both live
`serving import-bundle`/`reconcile` operations, explicit `serving select`, two `lctx_mcp.smoke`
invocations and artifact-verifying status checks passed. The
[cutover receipt](raw/operator-cutover.json) records selected behavioral generation `e50090b2…`
and exact profile `ff645e4a…`; catalog-only `ddb9fbcb…` is also ready. All four older ready
formats2/12 and3/13 generations remain immutable and retained.

`postgres_backup.py backup build/pr2-operator-cutover/current.dump` passed in 29.38 s.
`postgres_backup.py restore-drill build/pr2-operator-cutover/current.dump` passed all 81 tables,
six retained/current generations, current-profile native/PG/MCP serving and exact selection in
21.61 s. The protected archive, receipt and artifacts remain together in ignored build storage.
The [runtime checksums](raw/operator-runtime.json) identify the actual deployed CLI/native modules,
before the concurrent Cargo profile change. The controlled vLLM process used for
qualification stopped with exit0 after the two live operator smokes. No wheel was built.
