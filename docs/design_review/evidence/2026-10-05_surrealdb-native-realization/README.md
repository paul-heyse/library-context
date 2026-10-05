# SurrealDB-native realization of served journeys and validated, pinnable snapshots

**Consumer:** the SurrealDB pivot design review (Phase B, worker W1).

**Status:** concluded 2026-10-05 (the review was stopped at operator direction).
- Synthetic and model-shaped probes: **complete**.
- P2 at real scale on the staging generation's **facts frontier**: **complete**, attempt 3; see "Real data: facts frontier".
- P1 served catalog journeys on real data: **blocked**. No catalog relations exist because no generation was published.
- Everything else at real scale: `not_run (stopped at operator direction, 2026-10-05)`.
- Containers, volume and `w1_probe` are removed.

**Directive (operator):**
- Judge whether SurrealDB delivers the *functional outcome and guarantees* in its own native way. Current PostgreSQL mechanisms (one schema per generation, full copies, privilege-enforced immutability) are not requirements.
- Record no timing or benchmark comparisons. At scale, record only qualitative feasibility: completes, stalls, memory growth, refusals.

## Questions

**P1, journeys.** Can served agent journeys be expressed as SurrealDB graph queries and still return exactly the closure an independent PostgreSQL join returns? This includes:
- absent kept distinct from unknown;
- explicit truncation at bounds;
- contract failures on incomplete closure.

**P2, snapshot.** Can the snapshot intent be realized natively? The intent:
- complete, closed and attributable;
- never visible partially or before validation;
- immutable once published;
- pinnable by readers;
- reproducible;
- selection separate from publication;
- with structural sharing between snapshots.

P2 also asks how the model's integrity constraints map, and whether seeded violations are refused or caught.

## Versions and machine

- **SurrealDB.** `surrealdb/surrealdb@sha256:681c6c22c287421b5c7d99e0fde79b6e0d32c36c1ddeaab2762a1661cb04cd20`; `/version` reports `surrealdb-3.3.0`.
  - Server mode on RocksDB (`rocksdb:///data/db`), Docker volume `w1-surreal`, container `w1sdb`.
  - Memory capped at 8 GiB (created with 24 GiB, then lowered to 8 GiB with `docker update` before the final runs).
  - `SURREAL_HTTP_MAX_SQL_BODY_SIZE=256MiB`.
  - Root credentials are in a mode-0600 file under `build/review-probes/surrealdb-native-realization/root.pass`; they are never printed.
- **Model.** `target/release/lctx model describe --format json`, model digest `0ea88254…096f`, which is the same model digest as the generation being compiled. Repository HEAD `f66eb15a` plus the working tree.
- **Reference.** PostgreSQL 18.6 on the local cluster. The disposable database `w1_probe` holds the synthetic references; database `lctx` is never written.
- **Client.** Python 3.14.7, standard library only (HTTP `/sql` and `/signin`).
- **Machine.** Linux 7.0.0-38, x86_64, 32 CPUs, 188 GB RAM. A concurrent `lctx compile` was running under its own 64 GiB budget.

## Representation chosen (native, not like-for-like)

Built from the A3b analysis: shared content-addressed records whose snapshot membership is held as validity intervals.

- **Identity record per model relation.** `<relation>:`<id hex>`` holds the key fields, which determine the content id. It is shareable across snapshots without conflict.
- **Payload record.** Relations with non-key fields (321 of 878) get `<relation>__p:[<id hex>, <payload hash>]`, holding the non-key fields and `identity_of: record<relation> REFERENCE`.
  - A payload change makes a new payload record; the identity is still shared.
  - References always target identity records, so a payload change does not propagate a new version up the graph.
- **`spans: array<array<int>>`.** Snapshot-ordinal intervals: `[from]` is open, `[from, to]` is closed. A record is live at S when a span contains S.
  - Loading S opens spans for new records.
  - Records absent from S get their open span closed at S.
  - Records that are present and unchanged are untouched, so a load writes only what changed.
  - Closing a span at S never changes what any snapshot before S sees.
- **Immutability.** Every content field is `READONLY`. `spans` is the only mutable field, and only the owner (loader) writes it.
- **Reader enforcement (native).**
  - Readers are record users of access method `agent`. `SIGNIN` creates a `lease` record pinned to `selection:current` and returns it as the session identity. `CONTEXT { snap: $auth.snap }` exposes the pinned ordinal.
  - Every table has `PERMISSIONS FOR select WHERE fn::live(spans, $session.data.snap) FOR create, update, delete NONE`.
  - So a reader sees exactly its pinned snapshot, including through `<~` reverse references and record-link fetches, and cannot write.
- **Lifecycle.**
  - `snapshot:<S>` records the ordinal, state (staging/validated/published/failed/retired), source, model digest and a per-relation manifest of row count and digest over sorted keys.
  - Validation is a generated pass run as owner over records live at S.
  - Publication is one transaction: `snapshot:S` becomes `published` and `selection:current` becomes S. Selection stays separate from publication.
  - A failed staging snapshot is aborted: records whose only span is `[S]` are deleted and spans closed at S are reopened.
- **Retirement.** Records whose spans are all closed at or before the oldest retained snapshot are deleted. The harness only deletes after checking that the snapshot has no unexpired lease. Lease checking is application logic; SurrealDB does not enforce it.

## Schema generation (mechanical)

`gen_schema.py` reads the model declarations (`lctx model describe --format json`) and emits SurrealQL DDL into `build/review-probes/surrealdb-native-realization/schema.surql`: 8,234 statements and 1,203 tables (878 identity tables, 321 payload tables, plus `snapshot`, `selection`, `lease` and `reader`).
- **Applied:** all 8,234 statements returned OK on the STRICT database.
- **First attempt refused:** sum-type assertions written as one long AND/OR chain exceeded the 3.3 parser's expression recursion depth limit (`raw/schema-apply-attempt1.txt`). Flat array predicates (`[...].any(..)`, `[...].all(..)`) are accepted.

How each kind of model constraint maps (counts are from `raw/schema-totals.json`; the per-relation detail is in `raw/schema-mapping.json`):

| Model constraint | SurrealDB mapping | Enforced where |
|---|---|---|
| Typed fields (5,359) | `TYPE` (int, bool, string, bytes, float, array, `option<>` for nullable) | write time (refused) |
| int16/int32 ranges (992) | `ASSERT` range | write time |
| Codebooks (847) | `ASSERT $value IN [codes]` | write time |
| Digest shape (289) | string, `ASSERT` length 64 + hexadecimal (`string::is_hexadecimal`) | write time |
| Finite f64 (22) | `ASSERT` not NaN or ±inf | write time |
| Sum types (138 tags) | `ASSERT` on the tag: the arm's required fields present, other arms' fields NONE (flat array predicate) | write time |
| References (3,309) | `record<target> REFERENCE`. **Existence is not checked on write** (A3); cycles also rule out `ASSERT record::exists` during a staged load | validation pass (`closure:*`, live in the same snapshot) |
| Subtype references (28) | target tag equals the declared subtype | validation pass |
| Primary-key uniqueness / content identity | record id = content id: a duplicate in one statement is refused; an existing id with different content is refused by `READONLY` through the loader's `INSERT … ON DUPLICATE KEY UPDATE`, which re-asserts every field | write time |
| One payload per identity per snapshot | `<~(t__p FIELD identity_of)` count | validation pass |
| Content id = BLAKE3 of the typed key encoding | **not expressible** in SurrealQL; stays with the model's Rust code | external |
| 231 model invariants, 39 publication checks (opaque Rust state machines) | **not expressible** generically; they would run as the existing Rust validators over the snapshot's records | external (not run) |
| Vocabulary publication epochs (`introduced_epoch`, `at_epoch` prefixes) | not carried; snapshot ordinals would replace them | design gap, not probed |

## Commands

```bash
E=docs/design_review/evidence/2026-10-05_surrealdb-native-realization
B=build/review-probes/surrealdb-native-realization
IMG=surrealdb/surrealdb@sha256:681c6c22c287421b5c7d99e0fde79b6e0d32c36c1ddeaab2762a1661cb04cd20
mkdir -p $B && target/release/lctx model describe --format json > $B/model-describe.json
(umask 077; python3 -c "import secrets;print(secrets.token_hex(16))" > $B/root.pass)
docker volume create w1-surreal
docker run -d --name w1sdb --user root --memory 8g --memory-swap 8g -p 127.0.0.1:18800:8000 -v w1-surreal:/data \
  -e SURREAL_HTTP_MAX_SQL_BODY_SIZE=256MiB -e SURREAL_USER=root -e SURREAL_PASS="$(cat $B/root.pass)" \
  $IMG start --log warn rocksdb:///data/db
python3 $E/gen_schema.py $B/model-describe.json $B/schema.surql $E/raw/schema-mapping.json > $E/raw/schema-totals.json
(cd $E && python3 probe_synthetic.py)      # P1 + P2 on model-shaped synthetic data -> raw/synthetic-*
(cd $E && python3 probe_real.py dryrun --dry-schema s1)   # harness check of the real-data driver on the synthetic PG source -> raw/real-dryrun-*
(cd $E && python3 probe_real.py <generation-id-hex> --workers 6)   # real generation (pending) -> raw/real-*
(cd $E && python3 probe_idcollision.py)    # -> raw/idcollision.json
$E/probe_version_index.sh                  # own capped containers -> raw/version-index.txt
# teardown
docker rm -f w1sdb && docker volume rm w1-surreal
psql -h 127.0.0.1 -U lctx_superuser -d postgres -c 'DROP DATABASE IF EXISTS w1_probe'
```

The image runs as a non-root user, and RocksDB could not create its directory on a fresh named volume ("Permission denied"). The container therefore runs with `--user root`.

## Synthetic dataset

`synth.py` generates model-shaped rows for any relation:
- values follow the declared types, codebooks, sum arms and subtype constraints;
- every reference is closed, either by reusing a row or creating one on demand; a row is registered before its references are filled, so cycles close;
- ids are random 16-byte values (content-id derivation is not reproduced).

`dataset.py` builds the journey edge cases:

| Case | Contents |
|---|---|
| none | member with no invocations, options or facets |
| truncated | 130 briefs through 3 seeds |
| mixed | two invocations; options whose defaults have kinds 0/1/2/3; one facet without and one with a qualification; matching summary invocations and outcomes |
| contract | summary invocation without an outcome |
| random0–2 | random members |
| missing | an id that was never created |

The final run's dataset (`raw/synthetic-run.log`) is 374 S1 rows across 126 relations.

## Results (synthetic, final run, 2026-10-05)

Results are in `raw/synthetic-results.json` and `raw/synthetic-run.log`. Command: `python3 probe_synthetic.py`.

| Probe | Outcome | What it showed |
|---|---|---|
| p2.s1.validate | passed | 3,951 generated checks, no failures |
| p1.journeys.s1.closure-equal | passed | J1 briefs, J2 options/evidence/defaults and J3 behaviour facets/outcomes equal the PostgreSQL join reference exactly for all 8 cases (`raw/journeys-s1.json`) |
| p1.journeys.edge-cases | passed | (1) Truncation: total 130, page 100, omitted 30, truncated true. (2) A missing member is not visible, kept distinct from a visible member with empty sections. (3) Unknown vs absent: a facet without qualification is `unavailable` (1 of 2), distinct from no facet; default kinds 0/1/2/3 preserved. (4) A summary invocation without an outcome is reported as a contract failure |
| p2.seed.dangling-reference | passed | Load accepted (no write-time existence check); the validation pass caught `closure:catalog_member_invocations.member`; state `failed`; publication refused; abort left S1 intact |
| p2.seed.duplicate-id-different-content | passed | Refused at write by `READONLY` ("Found changed value for field `path` … readonly") |
| p2.seed.type-violation | passed | Refused at write (`Expected int but found 'not-an-int'`) |
| p2.seed.out-of-codebook | passed | Refused at write by the codebook `ASSERT` |
| p2.seed.duplicate-id-one-statement | passed | Refused (`already exists`); nothing stored |
| p2.seed.duplicate-id-in-snapshot | passed | Refused by the loader before any write (the same id twice in one snapshot) |
| p2.immutability | passed | The owner's content update was refused by `READONLY`. Reader create/update/delete returned **OK with `[]`, silently**, and had no effect |
| p2.no-partial-visibility | passed | During S2's load and validation, 426 polls by a reader pinned to S1 matched the S1 view exactly. `snapshot:2` was never visible. A reader signing in mid-load was pinned to S1 and could not see S2-only records |
| p2.pinned-reader-after-publish | passed | After S2 was published, the S1-pinned token still returned S1 results (equal to the S1 PostgreSQL reference) |
| p2.s2-reader-equals-reference | passed | A new reader is pinned to S2; its journeys equal the PostgreSQL reference for the perturbed S2 |
| p2.structural-sharing | passed | S2 (10 brief payloads changed, 5 briefs removed, 3 briefs plus a member and an option added): identity 6 new / 369 shared / 5 closed; payload 15 new / 181 shared / 15 closed. Unchanged content was neither copied nor rewritten |
| p2.retire-gc | passed | Retirement was blocked while S1 had unexpired leases (harness-checked). After expiry, 20 records live only in S1 were deleted; S2 journeys unchanged |
| id collision (`probe_idcollision.py`) | passed (trap confirmed) | Three records `[digest, 1]`, `[digest, 1dec]`, `[digest, 1f]` all exist, but a standard-index lookup and HNSW kNN(k=3) each return **one**. String keys (the chosen representation) return all three (`raw/idcollision.json`) |
| VERSION + index (`probe_version_index.sh`) | passed (trap confirmed; knob ineffective) | On versioned SurrealKV, `WHERE k='c' VERSION T1` with an index returns `[]` for a record that existed at T1; `WITH NOINDEX` returns it. `SURREAL_SURREALKV_VERSIONED_INDEX=true` reached the engine (log line) and changed nothing. The chosen representation does not use VERSION (`raw/version-index.txt`) |

**Run 1 failure, kept as evidence.** In run 1, `CONTEXT` read `selection:current`.
- Over HTTP, a token issued before S2 was published returned **S2** results after publication: the pinned-reader probe **failed** (`raw/synthetic-run1-context-from-selection.log`, `raw/journeys-s1-pinned-after-publish-run1.json`).
- `CONTEXT` is evaluated on each authentication, and HTTP authenticates every request, so pinning must live in the identity. That is why the fixed design returns a lease as `$auth`.
- Not tested: whether a WebSocket session keeps the earlier design pinned.

## Traps found here (SurrealDB 3.3.0)

1. **Unqualified reverse references.** `<~t` unions every field of `t` that references the record. A payload table with a model field also named `owner` produced wrong counts until queries used `<~(t FIELD f)`. Served reads that mirror `read_for(field)` must name the field.
2. **`DEFINE FIELD … OVERWRITE` replaces silently.** A generated field that collided with a model field name (`owner`) replaced the earlier definition without error. The generator now uses `identity_of`, which collides with no model field.
3. **Negative array indexes return NONE.** `[[1],[2,3]][-1]` is NONE without an error; use `array::last`.
4. **Sum assertions as long AND/OR chains hit the parser depth limit;** flat array predicates work.
5. **Reader writes blocked by `PERMISSIONS … NONE` return OK and `[]`,** not an error.
6. **`CONTEXT` is per authentication, and HTTP authenticates every request,** so a pin must live in the identity (above).
7. **`ON DUPLICATE KEY UPDATE` validates the full record** (a partial `$input` is refused for missing required fields). Combined with `READONLY` it gives a native content-equality check on sharing.
8. A named Docker volume needs `--user root` for RocksDB (above).
9. Carried from A3/A3b and confirmed here: nested-number id collisions in indexes; VERSION plus index.

## Real data: facts frontier of an unpublished, unvalidated generation

### Data used

Generation `786cd6d58dc5dccea686c0a54a8f0dcd` (FastMCP 4.0.5, behavioral profile, requested through catalog).
- **State:** `staging`; it was never validated or published. The compile failed after the facts stage when its own 30 s `idle_in_transaction_session_timeout` killed its lifecycle connection (product defect, logged by the coordinator).
- **Access:** read only, as `lctx_superuser` with `default_transaction_read_only = on`. Database `lctx` was never written. The generation was not aborted or altered.
- **Frontier:** 245 facts relations, 233 of them non-empty, **16,563,044 rows**. This is the sum of `stage_receipts.row_count` for the 217 ordinary relations, read from their main tables, plus `publication_outputs.row_count` for the 28 vocabulary relations sealed by `assemble`.
- **Vocabulary deltas:** the vocabulary was **never merged**, because publication-group closure never happened. Its rows exist only in private delta tables (`__delta_<blake3("<stage>/<relation>")[:40]>`, the naming in `ddl.rs::delta_name`); the main tables are empty. `real_source.facts_frontier` reads those deltas. That is my reconstruction of the closed vocabulary, not the system's merge.
- **Closure:** no reference leaves the frontier.
- **Not available:** normalized, analysis and catalog relations.

### What ran

Command: `probe_facts.py 786cd6d58dc5dccea686c0a54a8f0dcd --workers 8`. Server `w1sdb` on RocksDB, **memory cap raised to 48 GiB** for this load.

Attempt 3 ran start to finish (`raw/facts-results.json`, `raw/facts-run.log`, `raw/facts-memory.tsv`, `raw/facts-journey-j4.json`). The earlier attempts are kept as evidence:
- **Attempt 1** failed reading rows: `str.splitlines()` splits JSON text on U+2028 and similar separators. The fix splits on `\n` only (`raw/facts-run-attempt1.log`).
- **Attempt 2** recorded the same observations as attempt 3, but two harness predicates were wrong. The seed checks treated the count list `[0]` as a leftover, and the S2 check compared payload keys `[id, hash]` with bare ids. Both are fixed; the attempt-2 files are kept.

| Probe | Outcome | Qualitative observation |
|---|---|---|
| p2.facts.load-s1 | passed | Completed. 16,563,044 identity records plus 1,035,687 payload records. Per-relation counts equal the receipts exactly. No stall and no refusal. Container memory grew during load and validation, peaking at **34.2 GiB of the 48 GiB cap** (attempt 2: 28.6 GiB), and was not released before the run ended. |
| p2.facts.validate-s1 | passed | Completed. 868 generated checks (reference closure within the snapshot, subtype, one payload per identity); 0 failures. The model's 231 invariants and BLAKE3 id derivation were not run (external). |
| p1.facts.j4-evidence-closure-equal | passed | A facts-only evidence-closure query (occurrence → type observations → term, qualification, supports → run, surface, evidence) equals PostgreSQL joins over the same staging tables for 33 cases (busiest occurrence: 92 observations; one occurrence without observations; 30 sampled; one missing id). Real data had no section above the 100 bound, so truncation is only proven on synthetic data. **This is not a served journey.** |
| p2.facts.seed.dangling-reference | passed | Accepted at write. Delta validation (5 checks on the changed relation and its referrers) caught it; publication refused; abort by journal left S1 intact with no leftovers |
| p2.facts.seed.duplicate-id-different-content | passed | Refused by `READONLY` when the changed relation is re-asserted |
| p2.facts.seed.type-violation, out-of-codebook | passed | Refused at write |
| p2.facts.seed.content-change-unverified-path | passed (blind spot shown) | Relation key digests do not see a same-id content change. Without re-assertion, only the model's id recomputation (external) refuses it |
| p2.facts.no-partial-visibility | passed | 5 polls during the S2 load and validation: the pinned S1 reader and a new mid-load sign-in both saw S1. Few polls, because S2 wrote only two relations |
| p2.facts.validate-s2-delta | passed | 9 delta checks on changed relations and their referrers |
| p2.facts.pinned-reader-after-publish, s2-reader-sees-s2 | passed | The S1 token kept the old payload values (10) and the removed leaf records (5) after publication. A new S2 reader saw the changed values and the 3 added records |
| p2.facts.structural-sharing | passed | S2 wrote 3 identities and 10 payloads and closed 5 and 10. **243 of 245 relations were shared by manifest digest without writing**; 16,563,039 identity records were shared |
| p2.facts.retire-gc | passed | Retirement was blocked while 6 S1 leases were live. After expiry, the 15 S1-only records were deleted using the S2 journal |

Seeds used `carry` (relations not touched by the seed take their manifest entry from S1 without being read). The S2 perturbation read and digest-compared every relation.

### Not run (stopped at operator direction, 2026-10-05)

- **P1 served catalog journeys (J1–J3) on real data:** **blocked**. No catalog relations exist. They remain proven only on synthetic data.
- **Full re-validation of S2:** `not_run (stopped at operator direction, 2026-10-05)`. Only delta validation was run.
- **The model's 231 invariants and 39 publication checks over SurrealDB-held data, and BLAKE3 id recomputation:** `not_run (stopped at operator direction, 2026-10-05)`.
- **A load on the SurrealKV engine and a WebSocket-session pinning check:** `not_run (stopped at operator direction, 2026-10-05)`.
- **Sharing between two real compiled generations:** `not_run (stopped at operator direction, 2026-10-05)`. Only one, unpublished generation exists.

### Teardown (done 2026-10-05)

`docker rm -f w1sdb`, `docker volume rm w1-surreal`, and `DROP DATABASE w1_probe`. The `w1-version-*` containers were already removed by `--rm`. To rebuild, see "Commands".

## Real-data driver (ready, harness-checked)

`probe_real.py` runs the real-scale parts against a published generation. It reads the generation read-only through `real_source.py`, which streams `to_jsonb` rows through psql with `FETCH_COUNT`. Row conversion round-trips exactly against the synthetic PostgreSQL source: 126 relations, 0 mismatches.

For real data the loader runs with `verify_shared=False`, so a shared record costs no write and loading is O(Δ). Content equality then rests on content-addressed keys. The guarded path is exercised by the duplicate-content seed.

What it covers:
- **Load.** Loads S1 and compares per-relation counts with the `pg_stat` estimates.
- **Validation.** Runs the generated validation pass in parallel.
- **Journeys (P1).** Picks real journey cases with SQL: most briefs, no invocations, an unavailable facet, most options, members with summary invocations, 30 hash-sampled members, and one missing id. It compares SurrealDB journeys against joins on the generation itself.
- **Seeds.** Re-runs the four seeded violations at scale.
- **S2.** Builds a perturbed S2 over lazily loaded real rows. During the load, a pinned reader polls and a mid-load sign-in is checked. After publication it checks the pinned reader against the S1 reference and a new reader against an S2 reference built from the 11 journey relations in `w1_probe`.
- **Memory.** Samples container memory every 15 s to `raw/real-memory.tsv`.

**Dry run** (`--dry-schema s1`, synthetic source, 2026-10-05): every step **passed** (`raw/real-dryrun-results.json`, `raw/real-dryrun.log`). This checks the harness only; it is not a real-data result.

**Do not read a staging generation.** An attempted read of the compiling (staging) generation's tables waited on the compiler's relation lock (`pg_stat_activity` wait_event `Lock/relation`) and was cancelled. Real reads happen only after publication.

## Not run (real scale), and why it matters (written before the facts run; superseded where the facts run answered)

| Part | Question it would settle | Why it matters to the verdict |
|---|---|---|
| P1 journeys on the real generation | Do the same queries return closures equal to SQL joins over the real FastMCP catalog, with real truncation and unknown cases? | Synthetic data proves the semantics; real data shows whether real shapes (fan-out, cycles, nullable payload references) break them |
| P2 full-generation load (~11M rows, 878 relations plus 321 payload tables) on RocksDB | Does it complete, stall (cf. #7536), or grow memory without bound (cf. #7424) under the 8–24 GiB cap? | Feasibility of SurrealDB as primary store versus projection |
| P2 validation pass at scale (3,951 generated checks, record-link fetches per row) | Does it complete? | Whether the native validation path is viable or the Rust validators must read SurrealDB |
| P2 perturbed second generation on real data | Sharing statistics; pinned reader across publication | Structural-sharing benefit at real scale |
| Real sharing between two compiled generations | What fraction of identities and payloads two real compiles share | Whether sharing is a real lever (A3b B1) |

## Files

- **Probe sources:**
  - `probe_real.py`, `real_source.py`: real-generation driver and read-only source (including `facts_frontier`).
  - `probe_facts.py`: P2 at real scale over the staging facts frontier.
  - `gen_schema.py`: model → DDL.
  - `snapshot.py`: loader, validation pass, publish, abort.
  - `synth.py`, `dataset.py`: synthetic data.
  - `journeys.py`: SurrealQL and PostgreSQL reference queries.
  - `pgref.py`: disposable PostgreSQL reference.
  - `sdb.py`: HTTP client.
  - `probe_synthetic.py`, `probe_idcollision.py`, `probe_version_index.sh`.
- **Raw outputs (`raw/`):**
  - `schema-apply-attempt1.txt`, `schema-apply.txt`, `schema-totals.json`, `schema-mapping.json`;
  - `synthetic-results.json`, `synthetic-run.log`, `journeys-s1.json`, `journeys-s2.json`, `journeys-s1-pinned-after-publish.json`;
  - run-1 files with `run1` in the name;
  - `idcollision.json`, `version-index.txt`;
  - `real-dryrun-results.json`, `real-dryrun.log`;
  - `facts-results.json`, `facts-run.log`, `facts-memory.tsv`, `facts-journey-j4.json`, plus the `*-attempt1`/`*-attempt2` files.
- **Regenerable, under `build/review-probes/surrealdb-native-realization/` (gitignored):** `model-describe.json`, `schema.surql`, `root.pass`.
