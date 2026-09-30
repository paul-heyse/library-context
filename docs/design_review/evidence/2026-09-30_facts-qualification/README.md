# Facts qualification (P0–P2)

Consumer: cutover plan §4.2/Q and its assembled exit review. Evidence, never authority.
Implementation and focused tests precede integrated qualification. Current model schema digest:
`dad6dc7983136391c358394dfd3befb7a23aaf30e42719b8e0b57f513a6ed0ef`.
The schema snapshot migration adds typed flow observations/paths, materialized code-block
references, nominal derived receipt provenance and optional native attachment-kind fields.

`runtime-retirement.json` inventories the exact nine obsolete runtime paths removed at C3x.
No semantic or serving reader was running. All 13,093 protected acquisition, backup, benchmark
and adoption files retained identical sizes and modification times. Historical logs, receipts,
task reports and committed probe results remain; no legacy runtime generation is retained.

Focused checks on 2026-09-30 passed: the original 47-family fixture corpus in both profiles; native-signature, dictionary-key and Unicode controls bring the registered corpus to 50 families;
three real PostgreSQL fixture comparisons; typed call/flow/type/document/deployment controls;
attachment scalar oracle; shuffled, relocated and partitioned determinism; required producer
failure cleanup; unsupported-frontier/low-memory CLI controls. Receipts in plan §4.2.
Qualification commands and pilot receipts are recorded below; the final profile/control outcome is stated at the end.

The first full FastMCP acquisition refused 960 generated `_lctx_blocks` files (390,270 bytes)
in the pinned checkout before generation creation. These untracked legacy outputs were inventoried
and removed; locked acquisition inputs stayed intact. The rerun reached extraction and exposed
native provider correspondence gaps; each attempt aborted and was repaired before rerunning.

The first complete Rust test run executed 662 tests: 653 passed, 9 failed, 11 skipped. Failures:
four lost nested-expression function registrations after dependency pruning; one stale dormant
rule snapshot; the model snapshot; two hand-written test schedules missing new flow outputs; and
a stale signature coverage count. These require focused repairs/reruns before the complete gate.

Native signature corrections: ADR-0095 preserves positional-only/positional-or-keyword provider
order; ADR-0096 adds an unavailable native form preserving duplicate expanded slots and annotations,
with Partial signature coverage and binding refusal. Model15 and native-symbol5 focused tests passed;
the schema snapshot migration appends code3 without renumbering. SQLx completed reads/transactions
now shrink protocol buffers. Permanent generation2 and stage1 controls passed after this correction.

Resource scope: admission and retained typed state share a reservation budget. Native parse/solver
heaps, allocator retention, transient adapter collections, bounded SQLx pre-admission rows and
within-operation buffer high-water are measured external allowances. Shared Arrow buffers are
conservatively charged per holder; B-tree calibration and post-mutation nested growth remain named
limits. These receipts do not establish an absolute process-RSS cap.

Further native pilot corrections: ADR-0097 separates structural TypedDict field membership from
callable parameters; ADR-0098 retains expanded slot text while refusing unsupported callable
interpretation. Model type14/call15, native type4 and PG type1/call2 controls passed; the native
fixture exercises unavailable signatures, while unavailable callable-type correspondence is
source-inspected and shared-model mutation-Tested. Full catalog then exposed embedded NUL in
`literal_values.string_value` (PostgreSQL SQLSTATE 22021, COPY line3030). ADR-0099's `Utf8Text`
lowers valid Unicode to Binary/bytea without escaping; its semantic text key remains unchanged.

`python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test unicode_values
-p lctx-postgres --features testing --test unicode_values -p cpg-extract --test typed_types`
**passed** seven tests (model1, real PG1, native5). Initial test compilation lacked a budget helper
and referenced an unnecessary dependency; the first model expectation also omitted Batch's ID
ordering. These test defects were repaired and the same command passed. Native literals NUL and
`é\0終` and a NUL TypedDict field were observed; invalid UTF-8 decode refuses. Optional slot names
are source-inspected, rather than attributed to a native fixture that did not emit them.

The schema snapshot diff was read before `cargo insta accept --snapshot
model_describe__model_describe.snap`: five fields change Text to Binary and the model digest changes.
`cargo test --release -p lctx --test model_describe`, `just fmt`, `just lint` and `just docs-check`
**passed** on 2026-09-30. That docs publication had 225 canonical pages and no link errors; the final current publication receipt is recorded below.
The earlier assembled gate passed 669 Rust tests (11 skipped), 219 Python tests (56 skipped),
real-PG repeat controls and doctests before ADR-0099; it is superseded by fresh qualification for
the changed schema. Initial rule failures were repaired at the named read-only SQL owners and
whole-provider fail-stop panic boundary, with scoped review; five rule tests passed. The independent
flow oracle command `uv run pytest tests/scripts/test_flow_soundness.py -q` passed 18 tests.

## Final Q execution

`uv run python .../pilots.py` resets the regenerable semantic store and runs FastMCP 4.0.5's full
locked closure plus declared pinned docs/examples/tests, catalog then behavioral twice. It checks
facts-frontier publication, no automatic selection, behavioral repeat content equality and distinct
catalog/behavioral content. `pilot-receipt.json` and `raw/` retain commands, JSON outputs and time-v
process measurements. `measurements.py` queries published artifact sizes through leased DataFusion
providers, records stage time/sampled RSS/peak reservations and preserves generation details.
`refusals.py` challenges capture and provider-stage budgets, checks unchanged published inventory
and a clean store after each refusal. All three final profiles and both refusal controls passed; the current result is below.

The Q gate is `just test-all`: fmt-check, release Clippy, nextest, Python tests/typecheck, rules,
ADR/agents, fixture parsing, dependency/gold controls, disposable-PG repetition and positive/
compile-fail Rust doctests. The fresh gate passed after the final binding-owner repair (674 Rust,219 Python,217 real-PG repeat tests, plus doctests and all other components). Its receipt is a composite of the recorded
failures and repaired complete reruns; no initially clean run or product-quality claim is implied.

ADR-0100 follows the next catalog validation refusal, `a bound method binds a callable` after
515.155 seconds. The native builder already produces explicit Other/Truncated child residuals and
propagates opaque fidelity. Four typed envelope ports now permit only those residuals alongside
their previous forms; roles/order/nonempty checks and ordinary wrong-arm refusals remain. Shared
closure still requires DisplayOnly and matching provider/context transitively. The failed generation was removed; a new successful catalog confirmed 59 bound-method
children of kind Other, variant `unnamed_native_overload`, through the published leased CLI
query (`raw/catalog-bound-residuals.stdout`). No Truncated bound-function row was observed.
`python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test domain_types
-p lctx-postgres --features testing --test domain_types` **passed** 15 model tests and one real-PG
suite. The model covers four ports, both residual kinds, strong-fidelity refusal and foreign-context
refusal; PG adds sixteen envelope controls. Both bounded reviews accept this correction scoped.
The snapshot changes only its model digest; its diff was read before acceptance. The pre-0100
672-Rust/219-Python complete gate passed. The final changed-digest `just test-all` also passed:
673 Rust tests (11 skipped), 219 Python tests (56 skipped), 216 disposable-PG repeat tests
(one test and one binary skipped), plus positive/compile-fail doctests. `gate-receipt.json` and
`raw/qualification-gate.log` retain this composite receipt. Catalog publication passed in
484.392 seconds without selection at that checkpoint. Later source-only producer repairs require all three final profiles to be rerun.

The first behavioral attempt was deliberately interrupted after a short GDB stack sample located
CPU work in our `Index::runtime` repeated global lookup, rather than native ty. Its staging
generation `e510db6ea028a3580b8cde71c240b323` was aborted through the CLI; the published catalog
remained intact. This is an interrupted attempt, not a successful pilot or typed-resource control.
`pilot-runtime-index-receipt.json` and `raw/behavioral-stack-sample.log` preserve the observation.
`runtime-index-repair.patch` compares the exact old source recovered from our own compiled
`include_str!` against the repair. Charged source/scope/owner partitions preserve original order
and equality predicates; import classification is computed once globally, retaining cross-source
resolution behavior. No semantic owner or model declaration changes. Producer fingerprints change,
so the pilots and complete gate are rerun. The reviewer independently compared the sources.
`python3 scripts/build_environment.py -- cargo test --release -p cpg-extract --test typed_flow
--test determinism` **passed** six tests; the runtime-special/shadowing control now spans17 modules.

The next behavioral run extracted and sealed, then failed the shared same-place reaching
invariant after439.919 seconds; cleanup removed its generation. `pilot-reaching-place-receipt.json`
and `raw/behavioral-reaching-place.stderr` retain the failure. An early producer diagnostic
(`raw/behavioral-reaching-diagnostic.stderr`) identified FastMCP logging's comprehension walrus:
ty legitimately records comprehension evaluation scope, while the lexical model binds its target
in the enclosing function. The adapter now selects an exact site/name binding owner for stores
and unanimous binding-owner scopes for reads, preserving native attribution and the invariant.
A minimal walrus regression failed before repair. Five typed-flow and two determinism tests passed
after repair, including nine scope/member cases with explicit walrus owner checks.
`flow-binding-owner-receipt.json` records the bounded correction; the model digest is unchanged.

## Current qualification result (2026-09-30)

**Passed, facts frontier:** the complete `just test-all` gate (674 Rust/11 skipped,219 Python/56
skipped,217 tests in the PostgreSQL repeat suite/one test and one binary skipped, plus doctests and the remaining
checks); all three full pinned FastMCP pilots; profile-correct Flow coverage; no automatic selection;
behavioral repeated-content equality; both resource-refusal controls; and leased input-size queries.
Serving/MCP product evaluation, held-out confirmation and performance comparison are **not_run**
because phases 3–5 remain unavailable. This is a composite receipt after the failures recorded above.

| Profile | Elapsed seconds | Peak reservation bytes | time-v maximum RSS bytes |
|---|---:|---:|---:|
| catalog | 474.317 | 2350303801 | 4158078976 |
| behavioral | 628.956 | 2693597645 | 4234682368 |
| behavioral-repeat | 669.475 | 2693597645 | 4227559424 |

Per-provider timings and 20ms `/proc/self/status` VmRSS samples are in `measurement-receipt.json`;
whole-process time-v measurements are retained separately. These are shared-host measurements,
not a performance comparison or an absolute memory cap. Source/solver heaps, allocator retention,
transient native adapter state, SQLx within-operation high-water and sizing calibration remain
named allowances. P3 reader/provider growth triggers the retained resource-review corrections.

Both behavioral generations have content digest
`13bb1140f0b3dd35391002b3c2b7e338144932713f5a1c5a3771034ca0fa6a5b`; catalog differs:
`3fea0558aa3ccc3895f86d8ea7b86b18a38201c4cfce668138f7f1701a08cb73`.
All three are published, unselected and have no live writer/readers at the runner's inventory.
Each captured 10149 artifacts totaling 69507733 bytes; the largest Python source is
`google/genai/types.py`, 883926 bytes. The leased residual-child query confirms 59 native
`unnamed_native_overload` Other bound-method children in each profile.

`refusals.py` **passed**: 65536-byte capture refusal before generation creation; 268435456-byte
provider refusal in Pyrefly after acquire/deployment succeeded. The failed lexical-scope-support
reservation requested 143360 bytes with 268367657 bytes retained. Both preserved a byte-identical
generation registry and passed `store check`. These CLI controls do not expose a final reservation
counter; zero-release claims belong only to the focused controls.

The assembled P0–P2 exit review is **Accept scoped** after independent source/evidence inspection.
`just docs-check` **passed**: 226 canonical pages, zero link errors. Final STATUS and library-catalog
refresh are execution metadata; serving and resource exclusions above remain unchanged.

A final documentation rerun failed with ENOSPC after the first successful226-page publication.
`raw/docs-publication-space-failure.log` preserves the failure. The measured behavioral repeat
was then retired through the generation store: it was unselected, had zero readers/no writer and
identical recorded content, with no remaining runtime consumer. Scoped physical inventory:
2399879168 bytes. `duplicate-retirement-receipt.json` records Removed, a passed store check and
the final two-generation inventory. Catalog and primary behavioral facts remain available; pinned
inputs, shared compilation caches, backups and benchmark captures were preserved. The final
documentation rerun below supersedes this publication failure.

Final `just docs-check` rerun **passed**,226 canonical pages and zero link errors.
`uv run python scripts/library_utilization.py` identified expected drift (exit1); the required
last `just library-catalog` **passed**, regenerating65 libraries/91 capabilities from352 files
and219748 resolved nodes. `catalog-receipt.json` records this final refresh. STATUS is the current
P0–P2 checkpoint, with phase3 next and the resource/product exclusions retained.
