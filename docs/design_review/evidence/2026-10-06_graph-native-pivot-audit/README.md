# Graph-native implementation audit diagnostics

Consumer: [implementation audit](../../reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md), F04/F05;
current disposition: [coordinator §7](../../../plans/graph-native-pivot-plan_2026-10-05.md#7-sole-finding-disposition-and-optional-capabilities).

Question: does cold audit detect changed actual search text, and can failed selection publication
already publish a new serving pin?

## Execution and results

Executed 2026-10-06 against unchanged production `b90cb611b6b11ff3c01fdadc7e199c765d28a8b6`
on the local Linux host, pinned `nightly-2026-09-29`, Python 3.14.7 and SurrealDB SDK/server 3.3.0.
The native server used the pinned image and owned disposable RocksDB prerequisite from
`scripts/surrealdb_fixture.py`. No operator endpoint or state was read or changed.

From repository root:

```bash
UV_NO_SYNC=1 uv run --no-sync python docs/design_review/evidence/2026-10-06_graph-native-pivot-audit/run_probes.py
```

The runner release-builds the current publisher/serving libraries through ordinary normalized
Cargo configuration, then compiles the two small Rust diagnostics against those artifacts. The
pinned nightly stores full Rust metadata separately from code, so both Cargo-reported paths are
supplied. Build output stays in the normal cache; binaries, credentials and state stay in owned
temporary directories. All temporary state and the fixture container were removed after execution.

| Diagnostic | Recorded observation | Audit result |
|---|---|---|
| `selection_failure.rs` | A directory collision at the final selection path makes selection refuse, but `selected.serving.json` already contains the new handle. | F05 reproduced; intended atomic selection guarantee **failed**. |
| `cold_audit_search.rs` | Actual Catalog compilation/admission/publication of the installed first-party synthesis-sources fixture; cold audit initially succeeds. Seven actual `search_api_options` rows are changed to unrelated text. Canonical records/definitions remain unchanged; cold audit still succeeds. | F04 reproduced; intended whole-representation drift detection **failed**. |

The diagnostic process itself **passed** by observing and asserting these defects. These are
reproducers, not acceptance controls asserting the desired product behavior. Once corrected, their
expectations need to change or their cases should move into the owning targeted functional controls.
The root build **passed**. Initial diagnostic setup attempts **failed** before either probe ran:
rlib-only metadata loading and then rmeta-only linking; the runner was corrected to supply both.
Product code, tests and acceptance expectations were unchanged.

Raw execution log remains at `/tmp/graph-native-pivot-audit-probes.log`. The final probe output was:

```json
{"probe":"selection_failure","selection_refused":true,"new_serving_pin_published":true,"defect_reproduced":true}
{"probe":"cold_audit_search_corruption","changed_rows":7,"canonical_content_unchanged":true,"audit_accepted":true,"defect_reproduced":true}
```

The Catalog capture helper and installed-input fixture construction reuse the existing journey's
setup. The adverse text and independent expected failure are authored by this audit. Imports/style and
an unused helper were trimmed after execution; the decisive mutation/assertions are unchanged.

## Limits

Search text is the executed drift variant; vector/eligibility/witness/scope omissions are established
by the audit's source inspection. A crash between renames, retirement DROP and late engine export
failure were not injected. No cache race was executed. This isolated Catalog diagnostic does not
execute the stopped compiler suite, every behavioral branch, live Qwen, a real-library pilot,
operator adoption, broad qualification or a performance campaign. There is no new standing gate.

Publication `UV_NO_SYNC=1 just docs-check` **passed**, 2026-10-06. Explicit
`uv run --no-sync ruff check --config 'force-exclude = false'` for `run_probes.py` **passed**
after style/import repairs; the initial excluded/no-files selection was not verification.
