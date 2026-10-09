# Fast hashing for graph systems: where XXH3 fits and what it displaces — agent reference

**Scope:** how fast non-cryptographic hashing should be used with graph-shaped data, execution and dependencies:
- Rust crates `xxhash-rust` 0.8.19 and `twox-hash` 2.1.5.
- Python `xxhash` (python-xxhash) 4.0.1, which bundles libxxhash 0.8.3.
- The companion hashers that do the jobs XXH3 should not do.

The graph settings covered are graph databases (SurrealDB 3.3.2 from Rust, ArcadeDB 26.10.1 from Python), graph-driven execution and orchestration, dependency resolution, graph algorithms, and in-memory graph representation. The question throughout is where graph-plus-hash designs displace hand-declared mechanisms and where they do not. Snapshot date: 2026-10-08.

**Provenance.** Seven executed harnesses under `/home/claude/graph-hash-harness/` (written `$H` below): `primitives`, `engines`, `algos`, `depres`, `surreal`, `arcade`, `columnar`. Machine: one 2-vCPU KVM guest, Intel Xeon @ 2.10 GHz (AVX2, AVX-512F/BW/VL, AES-NI, SHA-NI), rustc 1.97.0 (2d8144b78 2026-07-07), Python 3.13.16. Sources were read at the commits and versions named in each section. Tags: **[exec]** executed in a harness (Appendix A says where); **[src]** read in source at the stated version; **[doc]** official documentation, spec or primary paper; **[vendor]** vendor-run benchmark; **[inf]** inference from the evidence; **[unverified]** could not be confirmed.

**Noise caveat.** Other agents shared the two vCPUs during many runs (load average 0.06–3, logged per results file). Figures are medians of 5–7 interleaved repetitions unless stated otherwise. **Treat differences under ~30% as parity.** Decision rules rest on ratios of 2× or more. Deterministic counts (task executions, invalidated artifacts, tuples, SCCs, duplicate rows) are machine-independent and are the primary evidence. Wall times are secondary. Where the research notes disagree, executed results win, and the conflict is stated.

**Bottom line.** In a graph-heavy codebase XXH3 has one natural home: **content fingerprints**. XXH3-128 over an explicit canonical byte encoding, propagated as Merkle fingerprints in topological or SCC order, costs ~37–75 ns per node in a 1M-node pass (~20–50 ns above an unhashed traversal) and is 7–26× cheaper than BLAKE3 or SHA-256 at node sizes (16 B–1 KiB). XXH3 is the **wrong** hash-table hasher: on hits Fx is 3.0–3.7× faster and foldhash 2.2–3.9× faster on graph-shaped keys. The bigger lever is not hashing at all: dense `u32` ids, CSR adjacency and bitset visited sets made BFS 3–7× faster than any hash-set visited set. Fingerprints that cross machines, tenants or trust boundaries need 128 bits at minimum, and SHA-256/BLAKE3 where an adversary can write.

The measured displacements are large but conditional. A content-addressed executor cut a comment-only edit from 232 task executions to 1 and an edit→revert from 232 to 0. Deep Merkle keys over a real call graph left 0 stale derived artifacts where file-mtime re-indexing left 6–344 stale. Deterministic content keys made re-ingest idempotent in both databases and, with a unique index, concurrency-safe in ArcadeDB (SurrealDB concurrency was not tested); without the index ArcadeDB produced duplicate edges under concurrency. Native CSR beat recursive SQL by roughly 10³–10⁵× on a 5,000-deep chain.

The honest counter-results matter as much. A dirty-bit + restat baseline matched content addressing on comment-only edits with zero hashing. In-process incremental engines (Salsa, Buck2 DICE) get early cutoff from value equality and revision numbers, not hashing. An undeclared input left the same 32 stale outputs under every incremental strategy. In-process SQLite loaded a 20k/60k graph 38× faster than ArcadeDB over HTTP and beat its default and cold closures. Adopt in this order, lowest risk and highest value first (§10): identity and DB schema → hash-based change detection → in-memory representation → Merkle memo keys → resolution and locks → algorithm engines.

---

## 0. Selection rules

### 0.1 The four hashing jobs

Every hashing decision below maps to one of these jobs. Mixing them up is the most common failure: XXH3 as a `HashMap` hasher, FxHash as a fingerprint, or a 64-bit XXH key in a shared cache.

| Job | What it must provide | Use | Width | Never |
|---|---|---|---|---|
| **J1 hash-table hashing**: visited sets, adjacency maps, interner tables, solver/Datalog indexes, e-graph memo | Low latency on 4–32 B keys; good top-7 bits (hashbrown takes its control tag there); in-process only | First choice: dense `u32` ids + `Vec`/`FixedBitSet`, no hashing at all. Otherwise `rustc_hash::FxBuildHasher` or `foldhash::fast` (foldhash when key layout is unknown or bit-packed). `nohash-hasher` only for keys that already are uniform 64-bit fingerprints. std `RandomState` (SipHash-1-3) for attacker-chosen keys | 64 | `Xxh3DefaultBuilder`, `BuildHasherDefault<twox_hash::XxHash3_64>`, `twox_hash::xxhash64::State`; persisting Fx/foldhash values |
| **J2 content fingerprints**: task and memo keys, output fingerprints, Merkle propagation, change detection, early cutoff | Uniform; stable across processes, versions, platforms and languages; fast at 16 B–1 KiB and in bulk | XXH3-128 over an explicit canonical encoding. xxhash-rust, twox-hash and python-xxhash agree bit-for-bit [exec] | 128 by default. 64 only for per-record change checks or per-iteration labels (§1.1) | `#[derive(Hash)]`/the `Hash` trait, `json.dumps`, pickle, serde over `HashMap`, engine hashes (`pl.Series.hash`, DuckDB `hash()`) |
| **J3 identity at scale / across trust boundaries**: shared or remote caches, lockfile integrity, permanent public ids | Negligible accidental collision at 10⁹+ items; resistance to adversarial writers | XXH3-128 inside one trust domain. SHA-256 or BLAKE3 when other parties write, or ids are public and permanent | ≥128; 160–256 for cryptographic hashes | XXH3-64 or XXH64 as a shared cache key at scale (P(any collision) is 2.7% at 10⁹ items) |
| **J4 sketches**: MinHash, Bloom/xor filters, HyperLogLog/HyperANF, WL rounds, CDC and prolly-tree boundaries, partitioning | Uniform under truncation; fixed, versioned seed when persisted | `xxh3_64_with_seed` (stable since XXH3 0.8.0); `foldhash::quality` for in-memory-only sketches ([foldhash README](https://github.com/orlp/foldhash)) | 32–64 | Fx/foldhash output over integer ids (structured: 0 collisions where 128 were expected [exec]); randomly keyed defaults for persisted sketches |

### 0.2 Graph operation → job → choice → evidence

| Graph operation | Job | Use | Key evidence |
|---|---|---|---|
| BFS/DFS visited set over node ids | J1, avoided | `Vec<bool>`, `FixedBitSet` (1/8 the memory, 9 µs clear), or epoch-stamped `Vec<u32>` (O(1) reset) | 88–115 ms (bool array, bitset, epoch stamps) vs 271–629 ms for every `HashSet<u32>` variant (2²⁰ nodes, 8.4 M edges) [exec] |
| Adjacency for traversal-heavy code | J1, avoided | Hand-rolled CSR (counting sort) or `petgraph::csr::Csr` | BFS 105/117 ms vs `petgraph::Graph` 783, `GraphMap` 578–648, `HashMap<u32,Vec<u32>>` 250–342 [exec] |
| Graph keyed by DB record ids or u64 fingerprints | J1, once | Relabel through `HashMap<u64,u32,Fx>`, then CSR + `Vec<bool>` | Build 457 vs 989–1,816 ms; BFS 104 vs 539–1,101 ms; 70 vs 153 MiB [exec] |
| Node/edge maps with trusted keys | J1 | `FxBuildHasher` or `foldhash::fast` | 14–19 ns per hit at 1 M keys vs 45–71 for `Xxh3DefaultBuilder` and 58–101 for SipHash [exec] |
| Map keyed by XXH3 fingerprints | J1 | `BuildNoHashHasher<u64>` | 15.4 vs 19.5 ns (Fx). On raw, shifted or packed ids inserts went quadratic (3–39 µs per insert) [exec] |
| Node names (strings) | J1 | string-interner (foldhash default), or lasso `Rodeo` with an explicit `FxBuildHasher` | 32–43 ns per string vs 90 for lasso's SipHash default [exec] |
| Structural dedup of terms, ASTs or plans | J1 | Hash-consing: `FxHashMap<Node, Id>` whose keys hold child ids | 19.85 M → 4,272 nodes; equality 27 µs → 1.2 ns [exec] |
| E-graph memo + union-find | J1 | egg / egglog (FxHash internally) | Paper: congruence 88×, equality saturation 21× (geometric mean) ([egg paper](https://arxiv.org/abs/2004.03082)) |
| Datalog joins over graph relations | J1 or none | datafrog (sorted vectors, no hashing); ascent (Fx `DashMap`); crepe (std, swap via `run_with_hasher`) | [src] §6.7 |
| Solver tables (resolvers) | J1 on interned ids | Fx, ahash or trivial integer hashes, as every resolver studied does | Fx 32–48 ns vs `Xxh3DefaultBuilder` 66–73 ns per 24-B name lookup [exec] |
| Task cache key (args + code version + input value fingerprints) | J2 | XXH3-128 over a length-prefixed LE encoding | Comment-only edit 232→1 executions; edit→revert 232→0 [exec] |
| Output fingerprint for early cutoff | J2 | XXH3-128 one-shot over the output bytes | Fingerprinting 10 k × 64 KiB outputs added ~6% to a cold build [exec] |
| Early cutoff inside one process | none | Value equality + revision counters (Salsa `values_equal`, DICE `EqualityBehavior::Compare`) | [src] §4.3 |
| Cache shared across machines or tenants | J3 | SHA-256/BLAKE3 as in REAPI; XXH3-128 only inside one trust domain | REAPI digest enum has no XXH3 ([proto](https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto)) |
| LLM labels/embeddings/summaries derived from a code graph | J2 | Identity = XXH3-128(path); content = XXH3-128(token stream); summaries keyed by SCC-aware deep Merkle | 0 stale vs 6–344 stale under file-mtime re-indexing [exec] |
| "Do we need to re-resolve?" | J2 | XXH3-128 of the sorted canonical manifest set | Invariant to declaration order; changes with constraints [exec] |
| Per-node lock fingerprint | J2 | XXH3-128(own content ‖ sorted dependency fingerprints) | Invalidates exactly the ancestors [exec] |
| Lockfile artifact integrity | J3 | SHA-256 (Cargo.lock, uv.lock, go.sum) | 1.25–1.27 GiB/s with SHA-NI [exec]; §5 |
| Anything emitted or iterated into output | (determinism) | `IndexMap`, `BTreeMap` or an explicit sort | petgraph's `dijkstra` map order changes per process [exec]; pubgrub #373 (§3.4) |
| Merkle hash over a DAG | J2 | XXH3-128, children in canonical order | 75 ns/node (1 M nodes) vs 277–390 for BLAKE3/SHA-256 [exec] |
| Merkle hash over a cyclic graph | J2 | Tarjan SCC condensation + sorted multisets + identity in node content | 0 failures over 400 relabels; 84.5 ms full recompute at 200 k nodes [exec] |
| Graph bucketing / near-duplicate graphs | J2/J4 | WL refinement with XXH3 labels as a bucket key, then exact confirmation (VF2, nauty-pet) | 68 M adjacency entries/s/thread; collides on 2×C3 vs C6 at every h [exec] |
| Exact graph identity | J2 over a canonical form | `nauty-pet` canonical form → XXH3-128 | [doc]; not benchmarked |
| Neighbourhood Jaccard; near-duplicate dependency sets | J4 | MinHash (probminhash) over seeded XXH3-64 neighbour hashes | [src]; not benchmarked |
| Membership pre-check before a DB round trip | J4 | xorf `BinaryFuse` over XXH3-64 keys (static sets); Bloom for growing sets | [src]; not benchmarked |
| Per-node reach or blast-radius counts at scale | J4 | HyperANF/HyperBall (webgraph-algo) | [paper]; not benchmarked |
| Vertex partitioning across workers | J1/J4 | `hash(id) mod N` over a well-mixed hash (the Pregel default) | ([Pregel](https://kowshik.github.io/JPregel/pregel_paper.pdf)) |
| Incremental topological order under edge inserts | none | `petgraph::acyclic::Acyclic::try_add_edge` (Pearce–Kelly) | [exec] §6.8 |
| SurrealDB node key | J2/J3 | `RecordId::new(tb, xxh3_64(name) as i64)` plus a READONLY natural-key guard; 128-bit as `[hi, lo]`, 32-hex or UUID | Point access is `RecordIdScan` with no index [exec] |
| ArcadeDB node key | J2/J3 | `key128`: 32-hex STRING with `UNIQUE_HASH` on the supertype | RIDs are reused after delete [exec] |
| Edge identity / dedup | J2 | SurrealDB: `[in, out]` array id or `LIGHTWEIGHT`. ArcadeDB: `UNIQUE(@out,@in)` + `IF NOT EXISTS`, or a hashed `ekey` | Re-ingest 0.55 vs 1.48 s; 19 vs 22 edges under concurrency [exec] |
| Change-detection column | J2 (64-bit is fine) | `content_hash` as signed 64-bit; manifest diff or guarded UPDATE | 50 changed of 5 k in 43 ms; 20 changed of 20 k in 0.33 s [exec] |
| Memo / cache table in the DB | J2 | XXH3-128 Merkle key over the closure's content hashes | Hit → leaf change → miss, with no invalidation code [exec] |
| Fingerprint stored in a DB integer column | (encoding) | `h as i64` for equality; `(h ^ 1<<63) as i64` for ordering or ranges; 32-hex for cross-language use | Bit-cast order matched u64 order at 0% of positions [exec] |
| Chunk boundaries (CDC, prolly trees) | J4 | FastCDC gear hash; Dolt uses XXH3 truncated to 32 bits | [exec] / [src] §7.3 |
| Chunk/blob ids | J2/J3 | XXH3-128 inside one trust domain; BLAKE3/SHA-256 across | XXH3-128 at 5.2–6.4 GiB/s vs SHA-256 1.19 per chunk [exec] |
| Columnar joins (Polars/DuckDB) | J1, internal | Engine-internal; never persist. For stable column hashes use polars-hash or DuckDB `hashfuncs` | `hashfuncs` `xxh3_128` swaps its 64-bit halves [exec] |
| Small keys in Python | J1 | Builtin `hash` (salted per process) in-process. python-xxhash only for persisted fingerprints or bulk bytes | ~110 ns per xxhash call vs 79 for builtin `hash` [exec] |

### 0.3 Which XXH3 implementation (J2/J3)

| Situation | Pick | Evidence (quiet-machine re-run unless noted) |
|---|---|---|
| Node-sized inputs ≤256 B (default build) or ≤64 B (native build) | either: parity | 16 B: 4.35 vs 4.62 ns; 256 B: 22.2 vs 21.2 ns (default build); 64 B native: 6.65 vs 6.86 ns. At 1 KiB twox leads on the default build (41.2 vs 57.7 ns) [exec] |
| Portable x86-64 binary, bulk ≥1 KiB | `twox-hash` (runtime AVX2 dispatch) | 1 MiB XXH3-64: 27.2 vs 17.9 GiB/s (xxhash-rust stays on SSE2) [exec] |
| Pinned AVX-512 host with `-Ctarget-cpu=native` | `xxhash-rust` (AVX-512 path) | 1 MiB XXH3-64 42.3 vs 29.2 GiB/s; 256 B 15.7 vs 25.6 ns [exec] |
| XXH64 with `target-cpu=native` on this CPU | `xxhash-rust` | twox-hash XXH64 drops to 4.84–4.89 GiB/s at ≥16 KiB; xxhash-rust stays at 10.3 GiB/s. On the default build both are 10.1. Confirmed on a quiet machine (Appendix A) [exec] |
| Python | python-xxhash 4.0.1, bytes only | Bulk 6.35–6.68 GiB/s; ~110 ns per call; `str` raises `TypeError` [exec] |
| Python, millions of small records | Move the loop to Rust (PyO3), or hash columns in bulk (polars-hash, DuckDB `hashfuncs`) | PyO3 canonical hash 8.65 vs 18.03 µs/record in pure Python [exec] |
| `HashMap`/`HashSet` hasher | neither | J1 row of §0.1 |

Both crates and python-xxhash produced identical outputs on 6 struct vectors up to 586 B; xxhash-rust and python-xxhash also agreed on WL graph hashes and 27 `canon/v1` vectors (twox-hash was not run on those) [exec]. Persist an algorithm/seed/encoding tag (e.g. `xxh3-v0.8/seed0/canon-v1`) next to every stored hash.

### 0.4 Displacement verdicts at a glance (detail and counter-cases in §8)

| # | Clunky, hand-declared pattern | Graph + hash replacement | Headline evidence | Old approach still better when |
|---|---|---|---|---|
| 1 | Dirty bits, mtimes, TTLs | Content-addressed DAG (constructive traces, XXH3-128 keys, early cutoff) | 232→1 executions (comment-only edit), 232→0 (touch, revert) [exec]; Bazel, Shake, rustc, Nix CA | Tasks cost about as much as hashing; edit cones are ~total; only comment-style cutoff is needed (restat matches it with no hashing); external state is unhashable (TTL survives) |
| 2 | Re-deriving LLM labels/embeddings per file or by mtime | Identity hash + token-stream content hash + SCC deep Merkle | 0 stale vs 6–344 stale (file-mtime) and 6–443 stale (shallow) [exec] | Call graph too approximate; high-fan-in edits cascade (446 artifacts); tiny repos |
| 3 | DB-generated or app-allocated ids | Content-derived keys + unique guard | Idempotent re-ingest; 50/50 rows under 4-thread UPSERT; RIDs are reused [exec] | No stable natural name; ids must be dense/ordered; single writer with a sequence |
| 4 | `updated_at`, version counters, polling | Stored content hash; manifest diff / guarded write / change feed | 43 ms over 5 k ids; 0.33 s over 20 k pairs; no-op writes emit no events [exec] | Audit and time travel; causal ordering; cross-node consumers |
| 5 | App-side SELECT-then-INSERT edge dedup | Deterministic edge ids + unique indexes | 22 edges (3 duplicates) without the index vs 19 with it [exec] | Parallel edges are real; insert-only bulk loads of known-new data |
| 6 | String- or sparse-u64-keyed maps in hot loops | Intern to dense `u32` + CSR | BFS 5.2× faster, build 2.2× faster (vs the best hasher) [exec] | Tiny graphs; deletion-heavy graphs; ids that must be stable across processes |
| 7 | Deep equality, bespoke dedup | Hash-consing | Dedup 587 → 0.13 ms; 476 MB → 0.38 MB [exec] | The producer already holds shared handles; little sharing; long-lived processes without interner GC |
| 8 | Hand-pinned dependency lists, hand-ordered steps | Solver + hash-fingerprinted lock + topological order | Conflict resolved with no pin; exact ancestor invalidation [exec]; Cargo, uv, Go | Single-version monorepo (MVS or "everything at HEAD"); fewer than ~10 steps |
| 9 | Hand-written "loop until no change" | Datalog semi-naive evaluation | Same 44,918 tuples; naive loop did 2.7×10⁹ probes (strawman) [exec] | Single-relation reachability (BFS + bitset); non-monotone logic |
| 10 | Recursive SQL for deep traversal | Native CSR / graph library | 5,000-deep chain: 0.1–0.4 ms vs 1.0–12.2 s [exec] | Shallow set-at-a-time queries over data already in SQL; in-process SQLite beat ArcadeDB on load and cold closure [exec] |
| 11 | Structural diffs of large stores, fixed-size chunking | Merkle/prolly trees + CDC | 193 vs 266,305 nodes visited; 1 vs 10,924 chunks re-stored [exec] | One-off comparisons (the Merkle build is ~7× one deep diff); in-place fixed-width updates |

---

## 1. The four jobs in depth: widths, collision budgets and canonicalization

### 1.1 Widths and collision budgets

The chance of any collision among *n* random values is ≈ n²/2^(b+1). Truncated XXH3-64 matches this random model [exec] (`$H/primitives/results/e_collisions.default.md`). The test hashed n distinct items with seeded XXH3-64, kept the low *b* bits and separately the high *b* bits, and counted colliding pairs over 5 seeds:

| b | n | expected pairs | u64 LE counter (low bits, obs/exp) | `"node:{i}"` (low bits, obs/exp) | high bits (counter / string) |
|---|---|---|---|---|---|
| 24 | 262,144 | 2,048 | 1.03 | 1.01 | 0.99 / 0.99 |
| 28 | 1,048,576 | 2,048 | 1.02 | 0.99 | 1.03 / 1.00 |
| 32 | 4,194,304 | 2,048 | 0.98 | 1.00 | 1.01 / 0.99 |
| 36 | 16,777,216 | 2,048 | 1.00 | 1.00 | 1.03 / 0.99 |
| 36 | 1,048,576 | 8 | 0.85 | 0.73 | 0.83 / 0.93 (Poisson noise) |

Analytic extrapolation, as expected colliding pairs and P(≥1 collision):

| bits | n = 10⁶ | n = 10⁸ | n = 10⁹ | n = 10¹² |
|---|---|---|---|---|
| 32 | 116 / 1.00 | 1.16e6 / 1.00 | 1.16e8 / 1.00 | 1.16e14 / 1.00 |
| 48 | 0.002 / 0.002 | 17.8 / ≈1 | 1,776 / 1.00 | 1.78e9 / 1.00 |
| 64 | 2.71e-8 | 2.71e-4 | **0.027** | **27,105 pairs** |
| 128 | 1.47e-27 | 1.47e-23 | 1.47e-21 | **1.47e-15** |

Item count *n* at which P(≥1 collision) reaches 10⁻⁶ / 10⁻³ / 0.5: 32 bits 93 / 2,932 / 77,163; 64 bits **6.07×10⁶** / 1.92×10⁸ / 5.06×10⁹; 128 bits 2.61×10¹⁶ at 10⁻⁶ [exec].

Width by use:

| Use | Width | Reason |
|---|---|---|
| Per-record change check (the stored hash vs the new hash of the *same* key) | 64 is fine | One comparison per record, not an n² namespace: a collision means one missed change in ~2⁶⁴ edits. Cargo truncates to 64 bits for exactly this per-unit check ([cargo hasher](https://github.com/rust-lang/cargo/blob/master/src/util/hasher.rs)) [inf] |
| Per-iteration WL labels inside one computation | 64 | A collision mislabels one vertex; it does not produce a wrong cache hit |
| Node identity key behind a natural-key guard (READONLY name, unique index) | 64 up to ~10⁷ entities (P ≈ 2.7×10⁻⁶) | The guard turns a collision into a hard error [exec §7.1] |
| Memo, task and cache keys, content addresses, dedup keys | **128** | 1.47×10⁻²¹ at 10⁹ items |
| Cache shared across machines or tenants | 128 minimum; cryptographic if others can write | Turborepo (XXH64) and Nx (XXH3-64) use 64-bit keys for remote caches: P ≈ 2.7×10⁻⁶ at 10⁷ entries, 2.7% at 10⁹ [inf] |
| Truncated to 32 bits | only below ~3,000 items at a 10⁻³ risk | Never use for ids |

**Map hashers are not fingerprints.** FxHash (rustc-hash 2.x) over integer ids 0..n produced **0 colliding pairs where 128 were expected**, in the low bits, the high bits, and for strided ids `i·4096`. XXH3 over the same ids gave 120 and 135. That is structure (a multiply-and-rotate acts like a Weyl sequence), not quality. The same structure makes Fx degrade on keys whose entropy sits only in bits ≥ ~38 (§2.2) [exec]. Never truncate Fx or foldhash output into a fingerprint or sketch key. Over `"node:{i}"` strings, all four hashers (Fx, foldhash fast and quality, XXH3) were near random: 114–154 pairs per cell against 128 expected [exec].

### 1.2 Canonicalization is the hard part

A fingerprint is only as stable as the bytes it hashes. These rules held in every executed harness:

1. Write an explicit encoder. Never hash `json.dumps`, pickle, protobuf bytes, serde output over `HashMap`, or `#[derive(Hash)]`.
2. Use type tags and fixed-width little-endian integers, with u64 length prefixes for strings, bytes and containers.
3. Sort map keys by their encoded UTF-8 bytes and reject duplicates. Sort child-fingerprint multisets when edge order is not semantic. Hash in order when it is (argument positions, pipeline stages).
4. Canonicalize floats (−0.0 → +0.0, every NaN → `0x7ff8000000000000`) or forbid them, as DRISL does ([DRISL](https://dasl.ing/drisl.html)).
5. Reject integers outside i64 rather than wrapping them.
6. Prefix a domain and version tag (`canon/v1\0`, `task/v1`, `scc/v1`), and use seeds or tags to separate the node and edge domains.
7. Hash with XXH3-128 and export it as zero-padded lowercase hex. For XXH3-128 that means `{h:032x}` = python `hexdigest()`, high 64 bits first.
8. Persist the scheme tag next to every digest.

**Measured canonicalization pitfalls**

| Pitfall | Behaviour | Fix |
|---|---|---|
| `json.dumps(sort_keys=True)` vs RFC 8785 JCS | Python sorts by code point (`{"\ue000":1,"😀":2}`, i.e. U+E000 before U+1F600); JCS by UTF-16 code units (`{"😀":2,"\ue000":1}`). Floats render as `1.0`, `1e-07`, `-0.0` vs JCS `1`, `1e-7`, `0`. JCS raises on NaN and on integers beyond 2⁵³ [exec `$H/columnar/probe_jcs.out`]; ([RFC 8785](https://www.rfc-editor.org/rfc/rfc8785)) | Use an explicit encoder. Use JCS only for signed-JSON interop: `serde_jcs` 0.2.0 matched Python `rfc8785` 0.1.4 on 3/3 cases, including astral keys [exec] |
| cbor2 6.1.5 `canonical=True` | Integer-key order is `[10, 23, -1, 24, 100, -25, -100, 1000]`; RFC 8949 §4.2.1 bytewise order is `[10, 23, 24, 100, 1000, -1, -25, -100]`. String-only keys agree [exec]; ([RFC 8949](https://www.rfc-editor.org/rfc/rfc8949)) | Don't assume RFC 8949 compliance for non-string keys |
| serde over `HashMap` | Three runs printed three key orders; `BTreeMap` was stable [exec `$H/columnar/hm`]. Every serde format, postcard included, serialises in iteration order | `BTreeMap`, or sort before encoding |
| `Hash` trait / `#[derive(Hash)]` | `str` appends `0xFF`; slices prefix a `usize` length; there is no cross-Rust-version guarantee | Explicit bytes; in-process use only |
| protobuf | "Deterministic serialization is not canonical"; "hashes of serialized protos are fragile" ([protobuf.dev](https://protobuf.dev/programming-guides/serialization-not-canonical/)) | Use REAPI-style canonical rules (tag order, no unknown or duplicate fields) |
| postcard | Wire format stable since 1.0, but it "does NOT enforce canonicalization" ([postcard](https://postcard.jamesmunns.com/wire-format)) | — |
| bincode 3.0.0 | A tombstone: `compile_error!("https://xkcd.com/2347/")` and an "unmaintained" README ([crates.io](https://crates.io/crates/bincode/3.0.0)), verified by unpacking it | Pin bincode 2.0.1 `serde::encode_into_slice` with fixint LE, or write the encoder by hand |
| Unicode NFC version skew | Python 3.13 ships Unicode 15.1; Rust `unicode-normalization` 0.1.25 ships 17.0. 20 code points normalise differently, e.g. Rust NFC(U+105D2 U+0307) = U+105C9 while Python leaves both [exec `$H/columnar/nfc_compare.out`] | Normalise once at ingestion and store the result; or pin both sides to one Unicode version; or reject code points unassigned in the older version |
| python-xxhash 4.0 | `str` raises `TypeError: Strings must be encoded before hashing`; `input=` is renamed `data=`; `VERSION_TUPLE` is removed ([CHANGELOG](https://github.com/ifduyue/python-xxhash/blob/master/CHANGELOG.rst)) [exec] | `.encode("utf-8")` after applying the NFC policy |
| DuckDB `hashfuncs` `xxh3_128` | Returns `0x78af5f94892f3950_06b05ab6733a6185` for `'abc'`; python gives `06b05ab6733a6185_78af5f94892f3950`. **The 64-bit halves are swapped** [exec, DuckDB 1.5.6 + hashfuncs `0dec806`] | Swap the halves, or compare `xxh3_64` (which matched) |
| Engine-internal hashes | `pl.Series.hash` "does not guarantee stable results across different Polars versions"; DuckDB `hash()` is a murmur-style mix ([duckdb hash.hpp](https://github.com/duckdb/duckdb/blob/main/src/include/duckdb/common/types/hash.hpp)) | polars-hash `nchash.xxh3_64` and DuckDB `hashfuncs` `xxh3_64` both equal python-xxhash [exec] |
| networkx WL hash | Values changed in 3.5 for directed and attribute-less graphs ([graph_hashing.py](https://github.com/networkx/networkx/blob/main/networkx/algorithms/graph_hashing.py)) | Persist the algorithm version (§6.2) |
| Pickle-derived cache keys | LangGraph's default key is `pickle.dumps((_freeze(args), _freeze(kwargs)), protocol=5)` ([_cache.py](https://github.com/langchain-ai/langgraph/blob/main/libs/langgraph/langgraph/_internal/_cache.py)): hits are environment-local [inf] | Supply a canonical `key_func` |
| Backend fallbacks | Hamilton main uses `xxhash.xxh3_128` and falls back to MD5; its own comment says fingerprints are "stable within an environment, not across installs with a different backend" ([fingerprinting.py](https://github.com/apache/hamilton/blob/main/hamilton/caching/fingerprinting.py)) | Make the dependency mandatory |
| Separator-free concatenation | Nx `assemble_ranked_hash` streams input hash strings into `Xxh3` with no separators ([task_hasher.rs](https://github.com/nrwl/nx/blob/master/packages/nx/src/native/tasks/task_hasher.rs)); networkx WL joins labels with no separators, producing collisions (§6.2) | Length-prefix everything |

### 1.3 Cross-language recipe: `canon/v1` (Rust ↔ Python, verified 27/27)

Pure Python, pure Rust and a PyO3 wheel produced **byte-identical encodings and XXH3-128 digests on 27/27 vectors**. The vectors covered: null and bools, i64 extremes, `1.0` vs `1`, `0.1`, ±0.0, NaN, inf, empty/ASCII/NFC/NFD/astral strings, bytes, nested lists and maps, mixed-plane keys, the `{"ab":["c"]}` vs `{"a":["bc"]}` concatenation trap, and a 3-leaf Merkle root (`86b024deb17fe139add19c9fb33e56aa` on both sides) [exec]. "café" in NFC and NFD hash identically because NFC is applied before encoding.

```python
# harness: /home/claude/graph-hash-harness/columnar/canon.py  (imported by pyo3_parity.py and merkle_diff.py)
# canon/v1, all integers little-endian:
#   0x00 null | 0x01 false | 0x02 true | 0x03 int: i64 (out of range -> error)
#   0x04 float: f64 bits; -0.0 -> +0.0; every NaN -> 0x7ff8000000000000
#   0x05 string: NFC, UTF-8, u64 byte length, bytes | 0x06 bytes: u64 length, raw
#   0x07 list: u64 count, items in order | 0x08 map: u64 count, entries sorted by NFC-UTF-8 key bytes
# digest = xxh3_128(b"canon/v1\x00" + encode(value))
import struct, unicodedata, math
import xxhash

QNAN = 0x7FF8000000000000

def _str_bytes(s: str) -> bytes:
    return unicodedata.normalize("NFC", s).encode("utf-8")

def encode(v, out: bytearray) -> None:
    if v is None: out += b"\x00"
    elif v is False: out += b"\x01"
    elif v is True: out += b"\x02"
    elif isinstance(v, int):
        if not -(1 << 63) <= v < (1 << 63): raise OverflowError(v)
        out += b"\x03" + struct.pack("<q", v)
    elif isinstance(v, float):
        if math.isnan(v): bits = QNAN
        else: bits = struct.unpack("<Q", struct.pack("<d", 0.0 if v == 0.0 else v))[0]
        out += b"\x04" + struct.pack("<Q", bits)
    elif isinstance(v, str):
        b = _str_bytes(v); out += b"\x05" + struct.pack("<Q", len(b)) + b
    elif isinstance(v, (bytes, bytearray, memoryview)):
        b = bytes(v); out += b"\x06" + struct.pack("<Q", len(b)) + b
    elif isinstance(v, (list, tuple)):
        out += b"\x07" + struct.pack("<Q", len(v))
        for x in v: encode(x, out)
    elif isinstance(v, dict):
        items = {}
        for k, x in v.items():
            if not isinstance(k, str): raise TypeError("map keys must be str")
            kb = _str_bytes(k)
            if kb in items: raise ValueError(f"duplicate key after NFC: {k!r}")
            items[kb] = x
        out += b"\x08" + struct.pack("<Q", len(items))
        for kb in sorted(items):
            out += struct.pack("<Q", len(kb)) + kb
            encode(items[kb], out)
    else:
        raise TypeError(type(v))

def canon_bytes(v) -> bytes:
    out = bytearray(b"canon/v1\x00"); encode(v, out); return bytes(out)

def canon_hash(v) -> str:
    return xxhash.xxh3_128_hexdigest(canon_bytes(v))

def merkle_node(payload, child_digests_hex):
    """Ordered Merkle-DAG node: H(tag || canon(payload) || count || child digests as 16 big-endian bytes)."""
    out = bytearray(b"dagnode/v1\x00"); encode(payload, out)
    out += struct.pack("<Q", len(child_digests_hex))
    for h in child_digests_hex: out += bytes.fromhex(h)
    return xxhash.xxh3_128_hexdigest(bytes(out))
```

```rust,ignore
// excerpt of /home/claude/graph-hash-harness/columnar/canon-rs/src/main.rs  (Rust side; binary takes vectors.json)
use serde_json::Value;
use unicode_normalization::UnicodeNormalization;
use xxhash_rust::xxh3::xxh3_128;
const QNAN: u64 = 0x7ff8_0000_0000_0000;
fn nfc(s: &str) -> Vec<u8> { s.nfc().collect::<String>().into_bytes() }
fn put_len(out: &mut Vec<u8>, n: usize) { out.extend_from_slice(&(n as u64).to_le_bytes()); }
fn put_f64(out: &mut Vec<u8>, v: f64) {
    let bits = if v.is_nan() { QNAN } else if v == 0.0 { 0u64 } else { v.to_bits() };
    out.push(0x04); out.extend_from_slice(&bits.to_le_bytes());
}
fn encode(v: &Value, out: &mut Vec<u8>) {
    match v {
        Value::Null => out.push(0x00),
        Value::Bool(false) => out.push(0x01),
        Value::Bool(true) => out.push(0x02),
        Value::Number(n) => { let i = n.as_i64().expect("int outside i64"); out.push(0x03); out.extend_from_slice(&i.to_le_bytes()); }
        Value::String(s) => { let b = nfc(s); out.push(0x05); put_len(out, b.len()); out.extend_from_slice(&b); }
        Value::Array(xs) => { out.push(0x07); put_len(out, xs.len()); for x in xs { encode(x, out); } }
        Value::Object(m) => { // ($f64 / $bytes tagged objects handled first in the full file)
            let mut entries: Vec<(Vec<u8>, &Value)> = m.iter().map(|(k, x)| (nfc(k), x)).collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            for w in entries.windows(2) { assert!(w[0].0 != w[1].0, "duplicate key after NFC"); }
            out.push(0x08); put_len(out, entries.len());
            for (kb, x) in entries { put_len(out, kb.len()); out.extend_from_slice(&kb); encode(x, out); }
        }
    }
}
fn canon_bytes(v: &Value) -> Vec<u8> { let mut out = b"canon/v1\x00".to_vec(); encode(v, &mut out); out }
// digest: format!("{:032x}", xxh3_128(&canon_bytes(v)))
```

PyO3 build: `pyo3 = { version = "0.29.3", features = ["extension-module", "abi3-py310"] }` with maturin 1.15.0 produced `canon_py-0.1.0-cp310-abi3-manylinux_2_34_x86_64.whl` [exec]. Its value is one implementation shared by both languages, not speed: 8.65 vs 18.03 µs/record (2.1×), because walking Python objects dominates.

**Fixed-schema struct encoders** (graph-node records) are the cheaper alternative when the schema is known. Shared test vector, matched by Rust (both crates) and python-xxhash 4.0.1 / libxxhash 0.8.3 on 6 vectors up to 586 B [exec `$H/primitives/results/python_check.md`]: bytes `2a000000000000000300efbeadde000000000000f83f0f000000706b672f636f72653a3a47726170680400000001000000020000000300000040420f00` → XXH3-64 `4455514264387335601` (`0x3dd52a9dfc7e35b1`), XXH3-128 `20744ca1dced2128da4db39423570b23`.

```rust,ignore
// excerpt of /home/claude/graph-hash-harness/primitives/src/bin/f_db_integers.rs
/// u64 id | u16 kind | u32 flags | f64 weight bits | u32 len + name UTF-8 | u32 count + count×u32 deps; all LE.
fn encode(id: u64, kind: u16, flags: u32, weight: f64, name: &str, deps: &[u32]) -> Vec<u8> {
    let mut o = Vec::new();
    o.extend_from_slice(&id.to_le_bytes());
    o.extend_from_slice(&kind.to_le_bytes());
    o.extend_from_slice(&flags.to_le_bytes());
    let w = if weight == 0.0 { 0.0f64 } else { weight }; // canonicalize -0.0 → +0.0
    o.extend_from_slice(&w.to_bits().to_le_bytes());
    o.extend_from_slice(&(name.len() as u32).to_le_bytes());
    o.extend_from_slice(name.as_bytes());
    o.extend_from_slice(&(deps.len() as u32).to_le_bytes());
    for d in deps { o.extend_from_slice(&d.to_le_bytes()); }
    o
}
```

```python
# excerpt of /home/claude/graph-hash-harness/primitives/scripts/check_python.py  (byte-identical to the Rust encoder)
def encode(id_, kind, flags, weight, name, deps):
    if weight == 0.0:
        weight = 0.0  # canonicalize -0.0
    nb = name.encode("utf-8")
    return (struct.pack("<QHId", id_, kind, flags, weight) + struct.pack("<I", len(nb)) + nb
            + struct.pack("<I", len(deps)) + struct.pack(f"<{len(deps)}I", *deps))
```

### 1.4 Storing fingerprints in signed integer and string columns

SurrealDB integers and ArcadeDB LONG are both signed 64-bit. Measured on 10⁶ random fingerprints (round trips asserted) and 200 k for sort order [exec `$H/primitives/results/f_db_integers.default.md`]:

| Encoding | Lossless | Matches unsigned sort order | Equals python-xxhash form | Use for |
|---|---|---|---|---|
| `h as i64` (bit-cast) | yes | **0.00%** of positions (order wraps at index 100,011) | `int.from_bytes(digest, 'big', signed=True)` | equality-only columns and keys (joins, `WHERE fp = ?`) |
| `(h ^ 1<<63) as i64` (sign flip / offset binary) | yes | 100% | no (bytes differ from the digest) | range scans, range partitioning, `ORDER BY` |
| `{h:016x}` / `{h:032x}` zero-padded lowercase hex | yes | 100% | `hexdigest()` exactly | cross-language identity, logs, URLs |
| unpadded hex / decimal string | yes | 0.01% / 0.00% | — | never as sortable keys |
| 128-bit as 16 big-endian bytes | yes | 100% | `digest()` exactly | where BINARY is usable (not ArcadeDB indexed equality, §7.2) |
| 128-bit as `(hi as i64, lo as i64)` | yes | **0.0%** | — | equality only |
| 128-bit as sign-flipped `(hi, lo)` | yes | 100% | — | ordered composite keys |

50.01% of random fingerprints become negative under the bit-cast, and they sort first. Python must apply the same two's-complement conversion: `u - (1 << 64) if u >= (1 << 63) else u`. Otherwise values ≥ 2⁶³ overflow or are rejected by signed columns. The cross-language check shows that this **conversion, not the hash**, is what drifts between languages [exec]. Test value: XXH3-128(`"node:42"`) = `1ea4757a4200fc7978aedbffed07c5a6`, i.e. `(hi, lo)` as i64 = `(2208018885272206457, 8696129822738859430)` [exec].

```rust,ignore
// excerpt of /home/claude/graph-hash-harness/primitives/src/bin/f_db_integers.rs
let col: i64 = h as i64;                    // lossless storage, NOT order-preserving
let ord: i64 = (h ^ (1u64 << 63)) as i64;   // order-preserving: ORDER BY / range scans match u64 order
let back: u64 = (ord as u64) ^ (1u64 << 63);
let hex32 = format!("{h128:032x}");          // == python xxhash.xxh3_128_hexdigest(b)
```

---

## 2. Hashing primitives on graph-shaped keys (measured)

All numbers in this section come from `$H/primitives`: default `x86-64` build (SSE2 at compile time), median of 7 interleaved repetitions. The full commands are in Appendix A.

### 2.1 Hash-table hashers by key shape (J1)

1M-key tables (much larger than L2), ns per operation, given as hit / miss. Insert means a fresh map without pre-sizing, so growth is included [exec `a_hashmap_keys.default.md`]:

| Hasher (std `HashMap` unless noted) | u32 dense ids | u32 sparse random | u64 pre-hashed fp | `(u32,u32)` edge | `&str` 16–32 B | insert u32 / u64 fp / `&str` |
|---|---|---|---|---|---|---|
| std `RandomState` (SipHash-1-3) | 60.6 / 23.4 | 58.4 / 22.9 | 80.1 / 26.9 | 101 / 32.8 | 164 / 55.3 | 79.4 / 108 / 399 |
| `foldhash::fast` | 16.4 / 5.47 | 15.2 / 5.31 | 19.5 / 6.83 | 18.2 / 6.25 | 99.7 / 23.0 | 50.7 / 53.5 / 163 |
| `foldhash::quality` | 21.2 / 6.96 | 22.0 / 6.80 | 26.2 / 8.07 | 27.5 / 7.41 | 89.4 / 23.7 | 48.8 / 54.7 / 178 |
| `rustc_hash::FxBuildHasher` | **14.2 / 4.09** | **14.8 / 5.34** | 19.5 / 6.79 | 19.4 / 6.82 | **64.3 / 18.4** | 47.1 / 54.1 / 145 |
| `ahash::RandomState` | 19.0 / 10.2 | 19.4 / 10.2 | 23.8 / 12.4 | 24.2 / 13.6 | 83.9 / 25.1 | 50.9 / 55.6 / 171 |
| xxhash-rust `Xxh3DefaultBuilder` | 45.9 / 18.3 | 45.3 / 17.6 | 58.3 / 20.9 | 70.8 / 25.9 | 224 / 64.8 | 81.5 / 95.6 / 372 |
| custom XXH3 one-shot-per-`write` `Hasher` | 23.4 / 8.81 | 23.1 / 8.17 | 25.2 / 9.35 | 33.4 / 13.3 | 88.0 / 26.5 | 53.3 / 55.6 / 196 |
| twox-hash `xxhash64::State` | 48.9 / 20.0 | 51.0 / 19.5 | 56.0 / 20.5 | **211 / 56.4** | 207 / 75.0 | 89.5 / 95.2 / 384 |
| hashbrown + `DefaultHashBuilder` (foldhash) | 16.6 / 5.54 | 19.4 / 5.84 | 18.6 / 6.86 | 18.4 / 6.31 | 77.6 / 23.2 | 49.8 / 48.0 / 159 |
| `nohash-hasher` (identity) | 7.30 / 1.29 *(invalid use)* | 22.9 / **55.3** *(invalid use)* | **15.4 / 5.98** | — | — | 85.7 / 54.0 / — |
| dense `Vec<u32>` index | **3.21 / 0.84** | — | — | — | — | 5.66 |
| sorted `Vec` + `binary_search` | 239 / 40.0 | 205 / 203 | 318 / 306 | 401 / 386 | 2025 / 2101 | — |

At 16K keys (L2-resident, so hashing dominates) integer and tuple hits cost Fx 2.50–3.52 ns, foldhash::fast 2.89–3.32, `Xxh3DefaultBuilder` 9.24–16.2, SipHash 12.2–19.2, dense `Vec` 0.545. Memory per entry at 1M entries without pre-sizing: `HashMap<u32,u32>` 18.0 B, `HashMap<u64,u32>` 34.0 B, `HashMap<(u32,u32),u32>` 26.0 B, dense `Vec<u32>` 4.0 B.

`std::collections::HashMap` and `hashbrown::HashMap` 0.17.1 performed the same within noise when given the same hasher [exec]. With `-Ctarget-cpu=native`, ahash gains most, because AES-NI only becomes available at compile time: `&str` hits drop to 67.0 ns, the best in that run (Fx 71.3) [exec `a_hashmap_keys.native.md`].

The **custom XXH3 Hasher** that halves XXH3's penalty (it still loses to Fx by 1.3–1.7×):

```rust,ignore
// excerpt of /home/claude/graph-hash-harness/primitives/src/bin/a_hashmap_keys.rs
#[derive(Default, Clone, Copy)]
pub struct Xxh3Oneshot(u64);
impl std::hash::Hasher for Xxh3Oneshot {
    #[inline] fn write(&mut self, b: &[u8]) { self.0 = xxhash_rust::xxh3::xxh3_64_with_seed(b, self.0); }
    #[inline] fn write_u32(&mut self, i: u32) { self.write(&i.to_le_bytes()) }
    #[inline] fn write_u64(&mut self, i: u64) { self.write(&i.to_le_bytes()) }
    #[inline] fn finish(&self) -> u64 { self.0 }
}
type Xxh3OneshotBuilder = std::hash::BuildHasherDefault<Xxh3Oneshot>;
```

Why XXH3 loses here: through the `Hasher` trait, XXH3's streaming state (360–576 B) and per-call setup outweigh its bulk speed for 4–32 B keys. `twox_hash::xxhash64::State` is pathological for tuple keys (11× Fx), because `(u32,u32): Hash` issues two `write_u32` calls through XXH64's streaming buffer [exec].

### 2.2 Key-pattern robustness and the `nohash` trap

`HashSet<u64>` at n = 2²⁰, given as insert / hit / miss ns. A variant was aborted once inserts exceeded 2 s [exec `h_key_patterns.default.md`]:

| Key pattern | Fx | foldhash::fast | SipHash | nohash | `Xxh3DefaultBuilder` |
|---|---|---|---|---|---|
| sequential `i` | 41.0 / 13.1 / 4.30 | 41.6 / 13.9 / 5.73 | 61.4 / 54.6 / 22.1 | 77.6 / 8.08 / 2.36 | 73.8 / 44.1 / 17.9 |
| random 64-bit (fingerprints) | 43.0 / 13.1 / 5.51 | 45.1 / 13.5 / 5.73 | 61.0 / 51.7 / 21.0 | 43.5 / 12.3 / 5.20 | 69.6 / 43.3 / 18.1 |
| packed edge `(u<<32)\|v`, u,v < 1024 | 44.8 / 13.6 / 5.84 | 44.3 / 14.5 / 5.95 | 61.7 / 52.1 / 21.2 | **ABORTED: 6.4 µs/insert** | 74.4 / 40.6 / 17.3 |
| `i << 32` | 43.3 / 14.3 / 4.44 | 48.2 / 12.7 / 5.10 | 66.0 / 68.5 / 27.7 | **ABORTED: 38.9 µs/insert** | 83.7 / 41.2 / 17.3 |
| `i << 40` | **89.7 / 22.8 / 38.9** | 39.9 / 16.2 / 5.34 | 66.4 / 55.7 / 21.9 | **ABORTED: 27.6 µs/insert** | 78.9 / 42.5 / 18.1 |
| `i * 4096` (aligned offsets) | 43.0 / 16.1 / 4.40 | 44.2 / 14.3 / 5.76 | 69.4 / 54.2 / 23.6 | **ABORTED: 5.2 µs/insert** | 78.1 / 45.7 / 18.0 |
| type tag `((i%4)<<56)\|i/4` | 42.1 / 12.4 / 4.33 | 42.3 / 13.7 / 5.80 | 61.1 / 52.9 / 21.5 | **ABORTED: 3.3 µs/insert** | 67.2 / 39.1 / 16.9 |

One mechanism explains every failure. hashbrown 0.17 takes the bucket index from the low bits and the 7-bit control tag from the **top 7 bits** (`let top7 = hash >> (MIN_HASH_LEN * 8 - 7)`, [tag.rs](https://docs.rs/crate/hashbrown/0.17.1/source/src/control/tag.rs)). An identity hash of a small integer has zero top bits, so every tag matches and the tag filter does nothing (sparse-u32 misses: 55 ns vs Fx 5.3). rustc-hash 2.x computes `(hash + i)·K` and finishes with `rotate_left(26)` ([rustc-hash lib.rs](https://docs.rs/crate/rustc-hash/2.1.3/source/src/lib.rs)); for `i << 40` the product's low 40 bits are zero, so after the rotate the top tag bits are constant and Fx misses rise to 38.9 ns.

Rules: **foldhash::fast is the safe default when key layout is unknown or bit-packed**. It has no slow pattern in the 7 tested and costs ≤1.11× Fx on inserts and hits (≤1.34× on misses). Use nohash **only** for uniformly random 64-bit values. foldhash describes itself as "minimally DoS-resistant", and its output is not consistent across versions or platforms ([foldhash README](https://github.com/orlp/foldhash)). FxHash makes no DoS claim at all ([rustc-hash lib.rs](https://docs.rs/crate/rustc-hash/2.1.3/source/src/lib.rs)).

### 2.3 Fingerprint throughput (J2/J3)

One-shot latency, ns per call, cache-hot. These are the quiet-machine re-runs at load average 0.06 and 0.82 [exec `c_fingerprint.{default,native}.quiet.md`]:

| Algorithm | 16 B | 64 B | 256 B | 1 KiB | 16 KiB | 1 MiB |
|---|---|---|---|---|---|---|
| XXH3-64 xxhash-rust | 4.35 | 6.60 | 22.2 | 57.7 | 830 | 54,436 |
| XXH3-64 twox-hash | 4.62 | 7.00 | 21.2 | 41.2 | 464 | 35,862 |
| XXH3-128 xxhash-rust | 6.74 | 9.74 | 26.8 | 61.9 | 817 | 54,434 |
| XXH3-128 twox-hash | 6.94 | 10.2 | 28.9 | 49.3 | 458 | 35,624 |
| XXH64 (either crate) | 6.85–6.94 | 17.0–17.7 | 36.5–36.6 | 105 | 1,463–1,472 | ~97,000 |
| BLAKE3 1.8.7 (1 thread) | 62.6 | 71.3 | 280 | 1,055 | 3,010 | 186,496 |
| SHA-256 (sha2 0.11, SHA-NI) | 74.6 | 117 | 249 | 815 | 12,085 | 767,150 |
| SipHash-1-3-128 | 20.9 | 34.5 | 87.2 | 304 | 4,625 | 298,871 |
| *native build:* XXH3-64 xxhash-rust (AVX-512) | 4.10 | 6.65 | **15.7** | **30.7** | **352** | **23,106** |
| *native build:* XXH64 twox-hash | 7.19 | 18.0 | 38.5 | 160 | **3,121** | **201,906** |

Bulk throughput at 1 MiB, in GiB/s:

| Algorithm | Default build | `target-cpu=native` |
|---|---|---|
| XXH3-64 xxhash-rust | 17.9 | **42.3** |
| XXH3-64 twox-hash | 27.2 | 29.2 |
| XXH3-128 xxhash-rust | 17.9 | 35.9 |
| XXH3-128 twox-hash | 27.4 | 28.5 |
| XXH64 xxhash-rust | 10.1 | 10.3 |
| XXH64 twox-hash | 10.1 | **4.84** |
| BLAKE3 | 5.24 | — |
| SHA-256 | 1.27 | — |

When inputs cycle through a 4 MiB window (beyond the per-core L2), small-input latency rises for every algorithm: roughly 2–2.7× for XXH3, XXH64 and SipHash, 1.2–1.5× for SHA-256 and BLAKE3. At 16 B XXH3-64 goes from 4.35 to 10.5 ns, SHA-256 from 74.6 to 90.8 ns [exec].

At node sizes (16 B–1 KiB) XXH3 is 7–26× cheaper than BLAKE3 or SHA-256, typically 10–20×. At ≥16 KiB BLAKE3's chunk parallelism engages (`CHUNK_LEN = 1024`, [blake3 lib.rs](https://docs.rs/crate/blake3/1.8.7/source/src/lib.rs)) and the gap narrows to 3.4–6.7× single-threaded; the gap to SHA-256 stays 14–27×. **The twox XXH64 regression under native flags is real on this CPU**: first seen under load, then confirmed at load 0.82. The earlier crate reference saw XXH64 at ~11–12 GiB/s "under every build" on a different host, so it is CPU-dependent; benchmark XXH64 on the target CPU before relying on native flags.

### 2.4 Encoding cost dominates hashing cost

Graph-node struct `Node { id: u64, kind: u16, flags: u32, weight: f64, name: String (16–32 B), deps: Vec<u32> (4–12) }`, in ns per node. Quiet default-build re-run [exec `c_fingerprint.default.quiet.md`]:

| Pipeline | Avg bytes | Encode ns | Encode + hash ns |
|---|---|---|---|
| hand fixed-width LE encoder (reused `Vec`) → XXH3-64 | 86 | 18.8 | **25.9** |
| hand LE → XXH3-128 | 86 | 18.6 | 29.7 |
| hand LE → BLAKE3 | 86 | 18.8 | 163 |
| hand LE → SHA-256 | 86 | 19.5 | 143 |
| streaming each field into `Xxh3Default` (no buffer) | 86 | — | 99.0 |
| bincode 2.0.1 `serde::encode_into_slice` (fixint LE, reused buffer) → XXH3-64 | 94 | **12.4** | **21.3** |
| bincode 2 `encode_to_vec` → XXH3-64 | 94 | 154 | 145 *(noise; allocation-dominated)* |
| postcard `to_slice` (reused 512 B) → XXH3-64 | 73.3 | 95.9 | 106 |
| postcard `to_allocvec` → XXH3-64 | 73.3 | 202 | 212 |
| serde_json `to_writer` (struct order; **not canonical**) → XXH3-64 | 179 | 226 | 241 |
| serde_json sorted keys (via `Value`/`BTreeMap`) → XXH3-64 | 179 | 828 | 858 |
| serde_json sorted keys → SHA-256 | 179 | 806 | 984 |
| `#[derive(Hash)]` → FxHasher (in-process only) | — | — | 7.43 |
| `#[derive(Hash)]` → foldhash::quality (in-process only) | — | — | 15.7 |

**Choose the encoder before the hash.** Moving from sorted-key JSON to a hand LE encoder saves ~800 ns per node. Moving from SHA-256 to XXH3-128 saves ~115 ns. Streaming 2–8-byte fields into `Xxh3Default` is ~4× slower than encoding into a reused buffer and calling one one-shot.

### 2.5 Merkle propagation and incremental re-propagation

DAG with N = 2²⁰ nodes and 4,193,826 edges, 0–8 children per node, 64 B of own content, `fp[i] = H(content_i ‖ fp[children…])` [exec `g_merkle.default.md`]:

| Scheme | Random children: total ms / ns per node | Local children (within 64): ms / ns per node |
|---|---|---|
| no hash (traversal + add/rotate) | 26.7 / 25.4 | 18.5 / 17.6 |
| XXH3-64, 8 B child fps (ordered) | 49.5 / 47.2 | 30.1 / 28.7 |
| XXH3-64 content + wrapping-add multiset of children | 49.2 / 46.9 | 26.4 / 25.1 |
| XXH3-64, children sorted (set semantics) | 96.6 / 92.1 | 70.7 / 67.5 |
| **XXH3-128, 16 B child fps** | **78.6 / 74.9** | **39.0 / 37.2** |
| BLAKE3 truncated to 128 | 290 / 277 | 198 / 189 |
| SHA-256 | 337 / 322 | 238 / 227 |
| BLAKE3-256 | 409 / 390 | 271 / 258 |

Incremental XXH3-128 re-propagation after one content change, with the dirty set being the node plus its ancestors. Each result was verified equal to a full recompute [exec]:

| Changed node | Dirty nodes | Share of graph | ms |
|---|---|---|---|
| index 1,048,566 | 1 | 0.0% | 0.065 |
| index 524,288 | 18 | 0.0% | 0.074 |
| index 104,857 | 597 | 0.1% | 0.505 |
| index 100 | 913,625 | **87.1%** | **158** (vs 78.6 for a full pass) |

That is ~173 ns per dirty node incremental against ~75 ns per node for a full pass, so **break-even sits near a 40–45% dirty fraction** (derived). Above it, recompute everything. Sorting children for set semantics costs about as much as the hashing itself, so pay for it only when child order is not already canonical. The wrapping-add multiset combine is linear and therefore forgeable: use it only for in-process memo keys, never across a trust boundary.

```rust,ignore
// excerpt of /home/claude/graph-hash-harness/primitives/src/bin/g_merkle.rs  (incremental re-propagation, verified == full)
// dirty set = changed + all ancestors (BFS over parents), recomputed in topological (index) order
let mut dirty = fixedbitset::FixedBitSet::with_capacity(n);
let mut stack = vec![changed as u32]; dirty.insert(changed);
while let Some(u) = stack.pop() { for &p in &par[poff[u as usize] as usize..poff[u as usize + 1] as usize] { if !dirty.put(p as usize) { stack.push(p); } } }
let mut buf = Vec::with_capacity(256);
for i in dirty.ones() {
    buf.clear(); buf.extend_from_slice(&dm.content[i]);
    for &c in dm.children(i) { buf.extend_from_slice(&fp[c as usize]); }
    fp[i] = x128(&buf);   // x128 = xxhash_rust::xxh3::xxh3_128(b).to_le_bytes()
}
```

### 2.6 Interning node names

Heavy duplication: 2²⁰ inputs drawn with skew from 65,533 distinct strings, average 24.9 B [exec `d_interning.default.md`]:

| Interner | Id | Intern ns/string | Resolve ns/id | B per distinct |
|---|---|---|---|---|
| lasso `Rodeo` default (std SipHash) | u32 dense | 90.3 | 1.02 | 66.9 |
| lasso `Rodeo<Spur, FxBuildHasher>` | u32 dense | 42.9 | 1.02 | 66.9 |
| string-interner `DefaultStringInterner` (foldhash) | u32 dense | **32.3** | 1.57 | **49.0** |
| `HashMap<String,u32,Fx>` + `Vec<String>` | u32 dense | 39.2 | 1.42 | 141 |
| hand arena + `hashbrown::HashTable<u32>` keyed by XXH3-64, equality-verified | u32 dense | 32.9 | 4.96 | 49.0 |
| hash-as-id `xxh3_64(str)`, no table | u64 sparse | **7.55** | n/a | 0 |
| hash-as-id + `HashMap<u64,String,nohash>` reverse map | u64 sparse | 22.6 | 4.97 | 91.3 |

With all strings distinct (2²⁰ unique, avg 26.4 B), costs are 169 ns for string-interner, 196 for lasso+Fx, 569 for lasso default, and 288 for the hand XXH3 arena [exec]. lasso's default `Rodeo` uses std `RandomState` unless the `ahasher` feature is on ([lasso lib.rs](https://docs.rs/crate/lasso/0.7.3/source/src/lib.rs)).

Hash-as-id is fast but its ids are 8-byte, non-dense and collision-exposed (P ≈ 3×10⁻⁸ at 2²⁰ distinct strings), cannot index a `Vec`, and need a reverse map to resolve that erases most of the gain. The equality-checked XXH3 table keeps the speed and stays correct under collisions:

```rust,ignore
// excerpt of /home/claude/graph-hash-harness/primitives/src/bin/d_interning.rs
let h = xxhash_rust::xxh3::xxh3_64(s.as_bytes());
let found = tab.find(h, |&id| { let (a, b) = spans[id as usize]; &arena[a as usize..b as usize] == s }).copied();
let id = found.unwrap_or_else(|| {
    let id = spans.len() as u32; let a = arena.len() as u32; arena.push_str(s); spans.push((a, arena.len() as u32));
    tab.insert_unique(h, id, |&j| { let (a, b) = spans[j as usize]; xxhash_rust::xxh3::xxh3_64(arena[a as usize..b as usize].as_bytes()) });
    id
});
```

Dependency hygiene: those 8 crates pulled **four hashbrown versions**: 0.14.5 (lasso), 0.15.5 (petgraph), 0.16.1 (string-interner) and 0.17.1 [exec, `cargo tree -i`].

---

## 3. Identity and representation in memory

### 3.1 Visited sets and adjacency layouts (BFS, measured)

Graph: N = 1,048,576 nodes, M = 8,388,608 directed edges with uniform endpoints. BFS reaches 99.97% of nodes [exec `b_bfs.default.md`, replicate `b_bfs.replicate.md`].

| Visited set (CSR fixed) | BFS ms (run 1 / replicate) | vs `Vec<bool>` | Memory MiB |
|---|---|---|---|
| `Vec<bool>` | 91.1 / 88.2 | 1.00× | 1.0 |
| `fixedbitset::FixedBitSet` | 104 / 92.3 | 1.05–1.14× | 0.1 |
| epoch stamps `Vec<u32>` | 114 / 115 | 1.25–1.31× | 4.0 |
| `roaring::RoaringBitmap` | 318 / 325 | 3.5–3.7× | 0.1 |
| `HashSet<u32>` + nohash / Fx / foldhash::fast | 271–310 | 3.0–3.4× | 10.0 |
| `HashSet<u32>` + foldhash::quality / ahash / XXH64 | 324–400 | 3.7–4.4× | 10.0 |
| `HashSet<u32>` + `Xxh3DefaultBuilder` | 568–572 | 6.3–6.5× | 10.0 |
| `HashSet<u32>` std SipHash | 616–629 | 6.9–7.0× | 10.0 |

Resetting the visited set for repeated traversals (N = 2²⁰): `Vec<bool>::fill` 86.9 µs, `FixedBitSet::clear` 9.05 µs, epoch `+= 1` 0.053 µs, `HashSet::clear` 71.2 µs.

| Adjacency (`Vec<bool>` visited) | Build ms | BFS ms | Retained MiB | Bytes/edge |
|---|---|---|---|---|
| hand CSR (counting sort) | **120** | **105** | **36.0** | 4.5 |
| `petgraph::csr::Csr` (sort + dedup, `from_sorted_edges`) | 397 | 117 | 40.0 (peak 104) | 5.0 |
| `Vec<Vec<u32>>` | 763 | 180 | 67.7 | 8.5 |
| `HashMap<u32,Vec<u32>>` Fx / foldhash / SipHash | 865 / 1015 / 1456 | 250 / 251 / 342 | 109.7 | 13.7 |
| `petgraph::Graph` (with_capacity; manual BFS) | 162 | **783** | 136.0 | 17.0 |
| petgraph `GraphMap` (IndexMap) Fx / foldhash / std | 3572 / 4006 / 6430 | 578 / 596 / 648 | **512.2** | 64.0 |

Library traversal with `petgraph::visit::Bfs`, including the visit-map allocation: `Graph` 1001 ms, `csr::Csr` 120 ms, `GraphMap<Fx>` 829 ms [exec].

Mechanisms: **`petgraph::Graph` is a linked list**: `Edge { weight, next: [EdgeIndex; 2], node: [NodeIndex; 2] }` threads outgoing lists through the edge vector ([graph_impl/mod.rs](https://docs.rs/crate/petgraph/0.8.3/source/src/graph_impl/mod.rs)), so converting to CSR pays for itself after about **one BFS** here. **`GraphMap`** stores `nodes: IndexMap<N, Vec<(N, CompactDirection)>, S>` plus `edges: IndexMap<(N,N), E, S>` with default `S = RandomState` (SipHash) ([graphmap.rs](https://docs.rs/crate/petgraph/0.8.3/source/src/graphmap.rs)). `Visitable::Map` is `FixedBitSet` for `Graph`, `StableGraph`, `Csr` and `MatrixGraph`, but a `hashbrown::HashSet<N>` (default foldhash, whatever `S` is) for `GraphMap` [src]. `Csr::from_sorted_edges` requires edges that are "sorted and unique" ([csr.rs](https://docs.rs/crate/petgraph/0.8.3/source/src/csr.rs)).

**Sparse 64-bit ids** (stand-ins for DB record ids or fingerprints):

| Representation | Build ms | BFS ms | Retained MiB |
|---|---|---|---|
| `HashMap<u64,Vec<u64>>` + `HashSet<u64>`, SipHash | 1816 | 1101 | 153.4 |
| … Fx / foldhash / nohash (valid: ids are random) | 989–1035 | 539–600 | 153.4 |
| **relabel u64→u32 (`HashMap<u64,u32,Fx>`) + hand CSR + `Vec<bool>`** | **457** | **104** | **70.0** |

```rust,ignore
// excerpt of /home/claude/graph-hash-harness/primitives/src/bin/b_bfs.rs  (CSR build + relabel of sparse u64 ids)
pub struct Csr { off: Vec<u32>, tgt: Vec<u32> }
impl Csr {
    /// Counting-sort build: O(N + M), two passes over the edge list.
    fn build(n: usize, edges: &[(u32, u32)]) -> Self {
        let mut off = vec![0u32; n + 1];
        for &(u, _) in edges { off[u as usize + 1] += 1; }
        for i in 0..n { off[i + 1] += off[i]; }
        let mut pos = off.clone();
        let mut tgt = vec![0u32; edges.len()];
        for &(u, v) in edges { let p = &mut pos[u as usize]; tgt[*p as usize] = v; *p += 1; }
        Csr { off, tgt }
    }
    #[inline] fn nbrs(&self, u: u32) -> &[u32] { &self.tgt[self.off[u as usize] as usize..self.off[u as usize + 1] as usize] }
}
// relabel: one Fx map pass from DB/fingerprint ids to dense u32, then CSR
let mut map: HashMap<u64, u32, rustc_hash::FxBuildHasher> = HashMap::default();
let mut dense = Vec::with_capacity(e64.len());
for &(u, v) in e64 {
    let l = map.len() as u32; let a = *map.entry(u).or_insert(l);
    let l = map.len() as u32; let b = *map.entry(v).or_insert(l);
    dense.push((a, b));
}
let g = Csr::build(map.len(), &dense);
```

Visited-set abstraction used for all the BFS rows:

```rust,ignore
// excerpt of /home/claude/graph-hash-harness/primitives/src/bin/b_bfs.rs
trait Visited { fn first_visit(&mut self, x: u32) -> bool; }
impl Visited for Vec<bool> { fn first_visit(&mut self, x: u32) -> bool { let b = &mut self[x as usize]; let r = !*b; *b = true; r } }
impl Visited for FixedBitSet { fn first_visit(&mut self, x: u32) -> bool { !self.put(x as usize) } }
struct Epoch { stamp: Vec<u32>, epoch: u32 } // O(1) reset: epoch += 1
impl Visited for Epoch { fn first_visit(&mut self, x: u32) -> bool { let s = &mut self.stamp[x as usize]; let r = *s != self.epoch; *s = self.epoch; r } }
```

Removing hashing from the traversal loop (dense ids, CSR, bitsets) beat the best hasher choice by 2.4–5.2× in every graph-shaped BFS test (2.4× for `HashMap<u32,Vec<u32>>` adjacency, ~3× for visited sets, ~5× for sparse u64 ids). The hasher choice itself moved results by ≤1.3× among Fx, foldhash and ahash [exec]. **These rules matter more than any hasher rule.**

### 3.2 Choosing a representation

| Need | Use | Avoid |
|---|---|---|
| Freeze-then-traverse (most analytics) | intern → sort edges → `Csr::from_sorted_edges` or hand CSR; `neighbors_slice(a)` is zero-copy | `Graph` for heavy traversal; `GraphMap` above ~10⁵–10⁶ edges |
| Mutation with stable indices | `StableGraph` ("does **not invalidate** any unrelated node or edge indices when items are removed") ([stable_graph](https://docs.rs/crate/petgraph/0.8.3/source/src/graph_impl/stable_graph/mod.rs)) | `Graph::remove_node` (`swap_remove` invalidates the last index) |
| Node weight *is* the id; O(1) edge-existence tests | `GraphMap` with `with_capacity_and_hasher(.., FxBuildHasher)` | the default `S = RandomState` |
| Parallel CSR at billions of edges | `graph` 0.3.2 / `graph_builder` 0.4.2 (rayon) ([graph](https://docs.rs/crate/graph/0.3.2/source/src/lib.rs)) | — |
| Rust ↔ Python shared algorithms | `rustworkx-core` 0.18.1 (petgraph + `IndexMap`/foldhash `DictMap`, insertion-ordered output) ([dictmap.rs](https://docs.rs/crate/rustworkx-core/0.18.1/source/src/dictmap.rs)) | — |
| Implicit graphs via successor closures | `pathfinding` 4.16.0 (`FxIndexMap` internally) ([lib.rs](https://docs.rs/crate/pathfinding/4.16.0/source/src/lib.rs)) | — |
| Persisted or shipped node sets in a huge id space | `roaring` 0.11.5 `RoaringBitmap` / `RoaringTreemap` | roaring as a hot BFS visited set (3.5× slower) |
| Bulk-synchronous analytics | CSR + bitset frontiers, or GraphBLAS masked SpMV ([graphblas.org](https://graphblas.org/)) | per-node `HashMap` frontiers |

### 3.3 Interning and hash-consing

Interning turns J1 hashing of strings into a one-time cost. After it, ids are `u32` values that index `Vec`s and bitsets. Choose the interner by lifetime [src]:

| Interner | Lifetime / memory behaviour | Source |
|---|---|---|
| `internment::Intern<T>` | never freed (`&'static`) | — |
| `ArcIntern` (ahash `DashMap`) | refcounted | [arc.rs](https://docs.rs/crate/internment/0.8.6/source/src/arc.rs) |
| `hashconsing` 1.8.0 (`HConsed` = uid + `Arc`; equality iff uids equal) | weak tables | [hashconsing](https://docs.rs/crate/hashconsing/1.8.0/source/src/lib.rs) |
| lasso `Rodeo` / string-interner | explicit owner, resolvable back to `&str` | — |
| arena + `Vec<Node>` + `FxHashMap<Node, Id>` (the egg pattern) | fastest; freed all at once | — |

Real systems that intern (sources in §4–§5): pubgrub `HashArena` (`IndexSet<P, FxBuildHasher>` → `Id<P>`); Cargo leaked `InternedString`/`PackageId` with pointer equality; Salsa `#[salsa::interned]` over an FxHash `HashTable`; Nx's `u32` "append-only interner for hash instructions".

**Dense interned ids are process-local** because they depend on insertion order. Never persist them. Keep a `fingerprint → dense id` map at the persistence boundary; that map can use `NoHashHasher`.

Hash-consing extends interning to structured nodes: the memo key is (operator, child ids), so equality becomes an integer comparison and trees become DAGs. Measured on 10,000 formulas over a shared 10-level vocabulary (average unfolded size 1,985 nodes) [exec `$H/engines/results/hash_cons.md`]:

| Representation | Nodes | Heap bytes | Allocations | Build ms |
|---|---|---|---|---|
| naive `Box` trees | 19,853,520 | 476,484,480 | 19,843,521 | 1,448 |
| `Rc` sharing via build-time memo | 10,652 | 500,640 | – | 0.77 |
| hash-consed, FxHash table | **4,272** | **375,888** | **25** | 173 |
| hash-consed, foldhash / SipHash / `Xxh3DefaultBuilder` table | 4,272 | – | – | 228 / 561 / **763** |

| Operation | Naive | Hash-consed |
|---|---|---|
| equality, formula vs its clone (full walk) | 26,958 ns | `Id == Id` 1.18 ns |
| dedup of 10 k formulas | 587 ms (XXH3-128 Merkle walk per tree, then set-insert) | 0.129 ms (FxHashSet of Ids) |

At depth 6 (less sharing) the same pattern holds: 1.74 M → 3,870 nodes, dedup 52.1 → 0.092 ms. The interning overhead is ~8.7 ns per constructed node with FxHash, which is pure cost when there is little sharing.

```rust
// harness: /home/claude/graph-hash-harness/engines (bin ex_e_hashcons)
//! Excerpt (e): hash-consing interner — structural sharing + O(1) equality. cargo run --release --bin ex_e_hashcons
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Id(u32);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Node { Var(u32), Const(i64), Add(Id, Id), Mul(Id, Id), Neg(Id) } // children are Ids ⇒ key is O(1) to hash

#[derive(Default)]
pub struct Interner { nodes: Vec<Node>, table: FxHashMap<Node, Id> } // small fixed-size keys: Fx/foldhash, not XXH3

impl Interner {
    pub fn mk(&mut self, n: Node) -> Id {
        let n = match n { // canonicalize commutative ops so a+b and b+a intern to the same Id
            Node::Add(a, b) if a > b => Node::Add(b, a),
            Node::Mul(a, b) if a > b => Node::Mul(b, a),
            n => n,
        };
        let nodes = &mut self.nodes;
        *self.table.entry(n).or_insert_with(|| { nodes.push(n); Id(nodes.len() as u32 - 1) })
    }
    pub fn get(&self, id: Id) -> Node { self.nodes[id.0 as usize] }
    pub fn len(&self) -> usize { self.nodes.len() }
    /// Bottom-up analyses become array passes: ids are created children-first, so index order is topological.
    pub fn sizes_as_tree(&self) -> Vec<u64> {
        let mut s = vec![0u64; self.nodes.len()];
        for (i, n) in self.nodes.iter().enumerate() {
            s[i] = 1 + match *n { Node::Var(_) | Node::Const(_) => 0, Node::Neg(a) => s[a.0 as usize],
                Node::Add(a, b) | Node::Mul(a, b) => s[a.0 as usize] + s[b.0 as usize] };
        }
        s
    }
}

fn main() {
    let mut it = Interner::default();
    let (x, y) = (it.mk(Node::Var(0)), it.mk(Node::Var(1)));
    // (x+y) * (y+x) built twice from scratch, as two independent parses would
    let build = |it: &mut Interner| { let a = it.mk(Node::Add(x, y)); let b = it.mk(Node::Add(y, x)); it.mk(Node::Mul(a, b)) };
    let (e1, e2) = (build(&mut it), build(&mut it));
    assert_eq!(e1, e2);                       // O(1) structural equality: compare two u32s
    assert_eq!(it.len(), 4);                  // x, y, x+y, (x+y)*(x+y): sharing is maximal
    // a 40-level tower t_{k+1} = t_k * t_k: 2^41−1 tree nodes, 41 interned nodes
    let mut t = x;
    for _ in 0..40 { t = it.mk(Node::Mul(t, t)); }
    assert_eq!(it.sizes_as_tree()[t.0 as usize], (1u64 << 41) - 1);
    assert_eq!(it.len(), 4 + 40);
    assert!(matches!(it.get(t), Node::Mul(a, b) if a == b));
    println!("ex_e_hashcons: all assertions passed ({} nodes stand for a {}-node tree)", it.len(), (1u64 << 41) - 1);
}
```

### 3.4 Determinism: hashers leak into algorithm output

| Finding | Evidence |
|---|---|
| petgraph 0.8.3 algorithms return or iterate foldhash-randomised hashbrown maps: `dijkstra`, `astar`, `articulation_points`, `coloring`, `dominators`, `floyd_warshall`/`johnson`, `k_shortest_path`, `maximal_cliques`, `min_spanning_tree`, `feedback_arc_set` | grep of [src/algo](https://docs.rs/crate/petgraph/0.8.3/source/src/algo/) [src] |
| Over 3 separate process runs, `dijkstra`'s result-map iteration order and the `maximal_cliques` output order changed. Distances (digest `5906ab788d635059`) and the DSATUR assignment (`ac7f30fd2d5dee91`) stayed identical, though DSATUR's stability is observed, not proven | [exec `$H/algos/src/bin/petgraph_determinism.rs`] |
| Two std `HashMap`s with the same 12 keys iterate in different orders. `FxHashMap` order is fixed across runs but depends on insertion order. `IndexMap` iterates in insertion order. `FxBuildHasher.hash_one(7u32) = 0x9d12ca918e61d971` in every run | [exec `keys_and_order.rs`] |
| **pubgrub issue #373**: Nickel's error snapshot came out in a different order on 32-bit Alpine. "FxHashMap has a stable iteration order — except when switching target." 0.4.0 made `DependencyConstraints` Vec-backed to fix it | ([pubgrub#373](https://github.com/pubgrub-rs/pubgrub/issues/373)) [WebFetch summary]; ([CHANGELOG](https://github.com/pubgrub-rs/pubgrub/blob/main/CHANGELOG.md)) [src] |
| Root cause: rustc-hash 2.1.3 uses `K = 0xf1357aea2e62a9c5` on 64-bit and `0x93d765dd` on 32-bit, with a `usize` state, so Fx *values* and iteration order depend on pointer width | ([rustc-hash lib.rs](https://github.com/rust-lang/rustc-hash/blob/master/src/lib.rs)) [src] |
| egg's `deterministic` feature swaps every internal `HashMap`/`HashSet` for `IndexMap`/`IndexSet` | ([egg util.rs](https://docs.rs/crate/egg/0.11.0/source/src/util.rs)) [src] |

Rules: never let hash-map iteration order reach output, golden tests, lockfiles, content-addressed results or cache keys (use `IndexMap`, `BTreeMap` or an explicit sort). Never persist Fx, foldhash, ahash or std hash values. FxHash's fixed seed gives reproducible but insertion-order-dependent iteration, which is not enough for "same graph, different construction order ⇒ same output".

---

## 4. Execution and orchestration graphs

### 4.1 Build Systems à la Carte: where hashing enters

Mokhov, Mitchell and Peyton Jones split a build system into a **scheduler** and a **rebuilder** ([JFP 2020](https://doi.org/10.1017/S0956796820000088); [ICFP 2018 PDF](https://www.microsoft.com/en-us/research/wp-content/uploads/2018/03/build-systems.pdf)). Table 2 of the paper:

| Rebuilder \ Scheduler | Topological | Restarting | Suspending |
|---|---|---|---|
| Dirty bit | Make | Excel | – |
| Verifying traces | Ninja | – | Shake |
| Constructive traces | CloudBuild | Bazel | (Cloud Shake) |
| Deep constructive traces | Buck (v1) | – | Nix |

What each rebuilder stores [doc]:

| Rebuilder | Stores | Consequence |
|---|---|---|
| Dirty bit | one bit per key, or a filesystem mtime | Make's mtime is "not under the control of MAKE" and assumes timestamps only move forward (backup software can break that) |
| Verifying traces | `Hash v` per dependency and per result ("can store only hashes of those values") | one trace per key loses hits when a dependency changes and then changes back |
| Verifying step traces (Shake) | dependency *keys* plus two logical times (`built`, `changed`), no hashes: "a typical cryptographic hash takes up 32 bytes, while a key … is an Int taking only 4 bytes" | on change-then-change-back, step traces rebuild where verifying traces skip |
| Constructive traces | the value or a CAS reference, many traces per key | makes shared/cloud caches possible |
| Deep constructive traces | hashes of terminal inputs only | shallow builds, but "no early cutoff" |

**Early cutoff** (an unchanged result means dependents do not run) needs values compared at intermediate nodes. The paper keeps `Hash v` abstract and never requires a cryptographic hash; cryptographic strength matters only when constructive traces are shared across a trust or scale boundary.

Seven orthogonal design axes for any graph executor. Each row lists the options, then exemplar systems in the same order, separated by "·" [inf]:

| Axis | Options | Exemplars |
|---|---|---|
| Dependency discovery | static (declared, derived from signatures or from the package graph) · selective · dynamic (recorded at run time) | Make, Ninja, Turborepo/Nx, Hamilton · Dune · Shake, Salsa, DICE, comemo, rustc, Nix |
| Scheduler | topological · restarting · suspending · push dataflow | Make/Ninja/Turbo/Nx · Excel/Bazel · Shake/Salsa/DICE/rustc/Nix · DD/DBSP |
| Rebuilder | dirty bit · verifying traces · step traces · constructive · deep constructive | Make/Cargo default · Ninja/rustc/comemo · Shake/Salsa/DICE · Bazel, Turbo/Nx, Prefect, LangGraph, Hamilton · Buck1, Nix input-addressed |
| Invalidation | eager push via reverse deps · lazy pull-verify | DICE, Adapton, Excel · Salsa, rustc, Shake |
| Early-cutoff signal | none · value equality · fingerprint equality · output content address | Make · Salsa, DICE, Shake · rustc, comemo, Hamilton · Bazel outputs, Nix CA |
| Hash roles | J1 · J2 · J3 · J4 | §4.2 |
| Persistence scope | in-memory · per-machine on disk · shared cloud cache | Salsa, DICE · rustc incremental dir, Cargo `.fingerprint` · Bazel RE, Turbo/Nx remote, Nix binary cache |

### 4.2 What real systems hash, and with what

Snapshot 2026-10-08, read from source at the stated commits or versions unless marked.

| System (version) | J1 tables | J2 fingerprint / early-cutoff signal | J3 cross-machine identity | Notes and source |
|---|---|---|---|---|
| **Bazel REAPI** (remote-apis `6def1c5`) | – | – | `Digest { hash: lowercase hex, size_bytes }`; enum SHA256, SHA1, MD5, VSO, SHA384, SHA512, **MURMUR3** ("not a cryptographic hash function and its collision properties are not strongly guaranteed"), SHA256TREE, BLAKE3, GITSHA1. **No XXH3** | The action cache keys `digest(Action)`; the CAS keys blobs by content; the input root is a Merkle tree of canonical `Directory` protos (sorted children, unique names); `salt` "allows disowning an entire set of ActionResults that might have been poisoned"; timeout is part of the key ([proto](https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto)) |
| Bazel local (master `8429fc3`, latest tag 9.3.0) | – | file digests | SHA-1/256/384/512, BLAKE3, GITSHA1 via `--digest_function` (`LOSES_INCREMENTAL_STATE`) | `--unix_digest_hash_attribute_name` reads precomputed digests from xattrs to save hashing I/O ([BlazeServerStartupOptions.java](https://github.com/bazelbuild/bazel/blob/master/src/main/java/com/google/devtools/build/lib/runtime/BlazeServerStartupOptions.java)). The default digest function was not verified from source |
| **Buck2 DICE** (`516d04a`) | `BuckHasher` wraps `fxhash::FxHasher64` | **none**: `EqualityBehavior::Compare(fn(&T,&T)->bool)` or `AlwaysUnequal`, plus a linear version counter | Buck2 CAS: `DigestAlgorithmFamily { Sha1, Sha256, Blake3, Blake3Keyed }` | Eager reverse-dependency dirtying gives "O(invalidated subset) traversals"; "Returning equality for different values would produce inconsistent graph state"; git-only, "experimental and largely being rewritten" ([key.rs](https://github.com/facebook/buck2/blob/main/dice/dice/src/api/key.rs), [incrementality PDF](https://github.com/facebook/buck2/blob/main/dice/dice/docs/DiceIncrementalityAlgorithms.pdf), [cas_digest.rs](https://github.com/facebook/buck2/blob/main/app/buck2_common/src/cas_digest.rs)) |
| **Nix** (master `81da930`, tag 2.35.2) | – | input-addressed: none (deep constructive) | Store path digest = **SHA-256 compressed to 160 bits**, Nix32-encoded, over `type:sha256:<inner>:<store>:<name>` | Floating CA derivations restore early cutoff but are **still experimental** ([store-path.md](https://github.com/NixOS/nix/blob/master/doc/manual/source/protocols/store-path.md), [experimental-features.cc](https://github.com/NixOS/nix/blob/master/src/libutil/experimental-features.cc)) |
| **rustc** incremental (`a30aa90`) | FxHash *(assumed, unverified)* | **128-bit `Fingerprint(u64,u64)`** via `StableSipHasher128` (SipHash-1-3); red-green try-mark-green | `DefPathHash` (128-bit) across crates | "Computing fingerprints is quite costly. It is the main reason why incremental compilation can be slower than non-incremental"; `eval_always`, `no_hash`, projection-query "firewalls" ([dev guide](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html), [fingerprint.rs](https://github.com/rust-lang/rust/blob/main/compiler/rustc_data_structures/src/fingerprint.rs)) |
| **Cargo** (`8d91d31`) | `FxHashMap` aliases | `StableSipHasher128` **truncated to u64** (16 hex chars), embedding dependency fingerprints (Merkle-like); **sources by mtime** by default | – | Unstable `checksum-freshness` compares size, then SHA-256/BLAKE3 emitted by rustc. "Filesystem mtime tracking is notoriously imprecise and problematic" ([fingerprint/mod.rs](https://github.com/rust-lang/cargo/blob/master/src/compiler/fingerprint/mod.rs), [dep_info.rs](https://github.com/rust-lang/cargo/blob/master/src/compiler/fingerprint/dep_info.rs)) |
| **Salsa** 0.28.5 (`30b614d`) | `FxBuildHasher` + `hashbrown::HashTable` (interning) | **none**: `values_equal` backdating + `Revision` (`verified_at`, `changed_at`) + durability levels | n/a (in-memory) | Backdating requires no cycle heads, a non-provisional old memo, and durability that does not decrease ([backdate.rs](https://github.com/salsa-rs/salsa/blob/master/src/function/backdate.rs), [interned.rs](https://github.com/salsa-rs/salsa/blob/master/src/interned.rs), [durability.rs](https://github.com/salsa-rs/salsa/blob/master/src/durability.rs)) |
| **comemo** 0.5.1 (Typst) | `FxHashMap<u128,…>` | **128-bit SipHash-1-3** of the key part of the input and of each tracked call's return value (`(call, u128)` constraints) | n/a | `pub fn hash<T: Hash>(value: &T) -> u128 { … SipHasher13 … finish128() }` ([hash.rs](https://github.com/typst/comemo/blob/main/src/hash.rs), [tree.rs](https://github.com/typst/comemo/blob/main/src/tree.rs)) |
| **Turborepo** 2.11.7 | – | git blob OIDs for files (CRLF→LF) | **XXH64 (seed 0)** over a canonical single-segment Cap'n Proto `TaskHashable` → 16 hex big-endian; artifacts optionally signed with HMAC-SHA256 | `task_dependency_hashes` are input-derived, so likely deep-constructive with no output cutoff *[inf, not traced]* ([traits.rs](https://github.com/vercel/turborepo/blob/main/crates/turborepo-hash/src/traits.rs), [lib.rs](https://github.com/vercel/turborepo/blob/main/crates/turborepo-hash/src/lib.rs), [signature_authentication.rs](https://github.com/vercel/turborepo/blob/main/crates/turborepo-cache/src/signature_authentication.rs)) |
| **Nx** 23.3.0 | `u32` interner for hash instructions | **XXH3-64** (xxhash-rust), rendered as a **decimal** string | the same XXH3-64 as the local/remote cache key | `hash_array` joins with `","`; `assemble_ranked_hash` concatenates without separators; `TaskOutput` inputs give output-based cutoff where configured *[inf]* ([hasher.rs](https://github.com/nrwl/nx/blob/master/packages/nx/src/native/hasher.rs), [task_hasher.rs](https://github.com/nrwl/nx/blob/master/packages/nx/src/native/tasks/task_hasher.rs)) |
| **DBSP / Feldera** `dbsp` 0.363.0 | **XXH3-64 via `Xxh3Default` + `Hash` trait, for sharding** ("Default hashing function used to shard records across workers") | Z-set deltas, not hashes | – | Bloom filters over key hashes in storage (J4) ([hash.rs](https://github.com/feldera/feldera/blob/main/crates/dbsp/src/hash.rs), [filter.rs](https://github.com/feldera/feldera/blob/main/crates/dbsp/src/storage/file/filter.rs)) |
| **Dagster** 1.13.25 | – | **SHA-256** of `code_version` + upstream data versions sorted by key | – | `code_version` is **hand-declared**; `StaleStatus {MISSING, STALE, FRESH}` ([data_version.py](https://github.com/dagster-io/dagster/blob/master/python_modules/dagster/dagster/_core/definitions/data_version.py)) |
| **Hamilton** 1.90.0 released / main | – | Released: MD5/SHA-224 data fingerprints. Main (unreleased): **XXH3-128** (python-xxhash), MD5 fallback. Code version = SHA-256 of the source with docstrings and comments stripped | – | DAG derived from function parameter names; the cache key uses **dependency data versions**, which gives early cutoff ([cache_key.py](https://github.com/apache/hamilton/blob/main/hamilton/caching/cache_key.py), [fingerprinting.py](https://github.com/apache/hamilton/blob/main/hamilton/caching/fingerprinting.py), [graph_types.py](https://github.com/apache/hamilton/blob/main/hamilton/graph_types.py)) |
| **Prefect** 3.8.8 | – | **MD5** (`usedforsecurity=False`) over sorted-key JSON, falling back to cloudpickle; `DEFAULT = INPUTS + TASK_SOURCE + RUN_ID` | – | Default caching is scoped to one run because of `RUN_ID` ([cache_policies.py](https://github.com/PrefectHQ/prefect/blob/main/src/prefect/cache_policies.py), [hashing.py](https://github.com/PrefectHQ/prefect/blob/main/src/prefect/utilities/hashing.py)) |
| **Flyte** v2 | – | inputs + task name + interface hash + cache version (auto mode = source hash) | – | ([Flyte docs](https://www.union.ai/docs/v2/flyte/user-guide/task-configuration/caching/)) [doc] |
| **LangGraph** 1.2.14 | – | **XXH3-128** hex of the pickled frozen args (`CachePolicy(key_func, ttl)`); TTL retained | – | Task ids for checkpoint format v>1 are `xxh3_128_hexdigest` formatted 8-4-4-4-12 (v1 used uuid5); `Requires-Dist: xxhash>=3.5.0` ([_algo.py](https://github.com/langchain-ai/langgraph/blob/main/libs/langgraph/langgraph/pregel/_algo.py), [types.py](https://github.com/langchain-ai/langgraph/blob/main/libs/langgraph/langgraph/types.py)) |
| Airflow 3.3.2 | – | no native content-keyed task cache found **[unverified absence]** | – | ([PyPI](https://pypi.org/project/apache-airflow/)) |

### 4.3 Key insight: equality inside a process, fingerprints across boundaries

The evidence splits cleanly by persistence scope [inf]:

| Scope | Early-cutoff signal | Examples |
|---|---|---|
| **Same process, previous value still in memory** | **value equality + revision counters, no hashing**: exact, collision-free, and a 4-byte revision beats a 32-byte hash (the Shake argument) | Salsa, DICE |
| **Persisted locally, one trust domain** | **64–128-bit fast fingerprints**: the previous value is gone, so compare fingerprints instead of loading it ("avoids loading the old result from disk", rustc guide). XXH3-128's home | rustc 128, Cargo 64 per unit, comemo 128 |
| **Shared across machines or tenants** | **≥128 bits; cryptographic when an adversary can write** | REAPI SHA-256/BLAKE3, Nix SHA-256/160, Buck2 BLAKE3-keyed. **Turborepo (XXH64) and Nx (XXH3-64) are the outliers**; Turborepo's HMAC protects integrity, not key collisions |

So adding XXH3 fingerprints to an in-memory engine like Salsa pays off only for persistence across restarts or remote caches (Salsa 0.28.x was not seen to persist memos *[unverified]*). A cached 128-bit fingerprint field on a large value is still useful as an O(1) pre-check before an expensive `values_equal` [inf]. Merkle composition appears wherever J2/J3 meet a graph, always behind canonical ordering [src]: REAPI `Directory` digests with sorted children; Cargo fingerprints embedding dependency fingerprints; Nix fingerprints referencing store paths; Turborepo `task_dependency_hashes` behind `sort_unstable_by` and canonical Cap'n Proto; Dagster's `sorted(keys, key=str)`; Nx's rank sort.

### 4.4 Measured: content-addressed executor vs naive, dirty-bit and dirty+restat

**Workload** [exec `$H/engines/results/dag_exec.md`, quiet main run, load 1.02]: 10,000 tasks in 10 layers, 20,874 edges; leaf "compile" drops `#` comment lines; leaf cone sizes median 232, p90 349, max 647. "CA" = content-addressed constructive traces keyed by XXH3-128(task id, version, args, input *value* fingerprints). "dirty+restat" = Ninja's `restat` policy ([Ninja manual](https://ninja-build.org/manual.html)) with output-byte equality in place of mtime and one stored version.

Cells are **task executions / wall ms** (median of 7):

| Scenario | naive | dirty-bit | dirty+restat | CA full | CA rayon (2 thr) | CA hybrid |
|---|---:|---:|---:|---:|---:|---:|
| cold build | 10000 / 152.1 | 10000 / 150.3 | – | 10000 / 151.8 | 10000 / 83.2 | – |
| no-op rebuild | 10000 / 155.4 | 0 / 0.3 | 0 / 0.4 | 0 / 1.3 | 0 / 1.5 | 0 / 0.4 |
| touch only (mtime bump, same bytes), median leaf | 10000 / 162.3 | 232 / 4.1 | 1 / 0.4 | **0** / 1.5 | 0 / 1.9 | 0 / 0.4 |
| value edit, median leaf | 10000 / 158.1 | 232 / 3.9 | 232 / 3.9 | 232 / 6.1 | 232 / 4.2 | 232 / 3.8 |
| comment-only edit, median leaf | 10000 / 154.3 | 232 / 3.8 | **1 / 0.4** | **1** / 1.5 | 1 / 1.5 | 1 / 0.4 |
| edit→revert, median leaf | 10000 / 169.6 | 232 / 4.0 | 232 / 3.9 | **0** / 1.2 | 0 / 1.5 | 0 / 0.4 |
| comment-only edit, p90 leaf | 10000 / 149.0 | 349 / 6.1 | 1 / 0.4 | 1 / 1.5 | 1 / 1.8 | 1 / 0.4 |
| edit→revert, p90 leaf | 10000 / 157.2 | 349 / 6.0 | 349 / 5.7 | 0 / 1.2 | 0 / 1.6 | 0 / 0.5 |

All strategies produced outputs identical to a from-scratch build. Re-keying all 10 k tasks ("CA full") costs ~1 ms. "CA hybrid" re-keys only the frontier (touched tasks and tasks with a changed input) and measured at dirty-bit cost in every scenario.

**The honest restat finding.** The comment-only cutoff comes from **comparing values after re-execution, not from hashing**: dirty+restat gets 232 → 1 with one stored version and zero hashing. Content addressing adds three things: (1) **restoring any previously seen version** (edit→revert and branch switching 232 → 0, where restat still needs 232); (2) **immunity to mtime-only touches** (232 → 0 vs dirty-bit); (3) **a key meaningful across processes and machines**, enabling shared/remote caches (from the paper; remote caching not measured).

**Counter-cases, measured:**

| Condition | Result | Source |
|---|---|---|
| 30% of interior tasks lossy (output is one of 4 values) | Interior cutoff: a value edit runs 217 instead of 232 (median) and 233 instead of 349 (p90) under restat/CA/hybrid; dirty-bit stays at 232/349 | [exec `dag_exec_coarse.md`] |
| Zero-cost tasks (contended run) | Value edit: dirty-bit 0.3 ms vs CA full 1.3, hybrid 0.5. No-op: 0.3 vs 1.3. Bookkeeping dominates when tasks are µs-scale | [exec `dag_exec_work0.md`] |
| 64 KiB outputs, no work (contended run) | Cold: naive 770 vs CA full 819 ms (≈ +49 ms ≈ 0.61 GiB of XXH3-128). Edit→revert: dirty-bit/restat 24.4–24.5 vs hybrid 0.4 ms | [exec `dag_exec_work0_64k.md`] |
| Deep narrow DAG (1,000 layers × 10, median cone 9,947) | A value edit costs ≈ the same for every sequential strategy (212–219 vs naive 209 ms). CA rayon 126 ms. CA rayon no-op 24.5 vs 1.2 ms (1,000 per-level barriers) | [exec `dag_exec_deep.md`] |
| **Undeclared input** (task 5000 reads ambient config that is not in its key) | **32 stale outputs under dirty-bit/restat and the same 32 under CA (0 executions)**. Only naive re-execution is correct | [exec `dag_exec.md`] |
| Store growth | The CA store keeps every version (10,001 entries after a cold build + one edit). GC was not prototyped | [exec] |

Rules [inf]: CA bookkeeping costs ≈100 ns per task, so re-keying everything is fine up to ~10⁵–10⁶ tasks when tasks are ≫1 µs; otherwise re-key only the frontier. If you need only comment-style cutoff (no revert, branch switching or shared cache), restat is enough. Model every ambient read as an explicit input node whose value hash enters the key; rustc's `eval_always` concedes the same point.

**Reference implementation: content-addressed executor** (assertions encode the claims):

```rust
// harness: /home/claude/graph-hash-harness/engines (bin ex_a_ca_executor)
//! Excerpt (a): content-addressed DAG executor (constructive traces + early cutoff). cargo run --release --bin ex_a_ca_executor
use rustc_hash::FxHashMap;
use std::sync::Arc;
use xxhash_rust::xxh3::{xxh3_128, Xxh3Default};

pub struct Task { pub version: u32, pub args: Vec<u8>, pub inputs: Vec<u32> } // task id = index
pub type Run = fn(id: usize, t: &Task, inputs: &[&[u8]]) -> Vec<u8>;
#[derive(Clone)] struct Entry { out: Arc<[u8]>, fp: u128 }
#[derive(Default)] pub struct Store { map: FxHashMap<u128, Entry> } // key → output (+ its fingerprint); keeps ALL versions

/// key = XXH3-128 over a canonical, length-prefixed encoding; inputs enter by VALUE fingerprint, not by name/mtime.
fn task_key(id: usize, t: &Task, input_fps: &[u128]) -> u128 {
    let mut h = Xxh3Default::new();
    h.update(b"task/v1");
    h.update(&(id as u64).to_le_bytes());
    h.update(&t.version.to_le_bytes());
    h.update(&(t.args.len() as u64).to_le_bytes()); h.update(&t.args);
    h.update(&(input_fps.len() as u64).to_le_bytes());
    for fp in input_fps { h.update(&fp.to_le_bytes()); }
    h.digest128()
}

/// Kahn topological schedule. Returns (#executions, output fingerprint per task).
pub fn build(tasks: &[Task], run: Run, store: &mut Store) -> (usize, Vec<u128>) {
    let n = tasks.len();
    let mut indeg: Vec<usize> = tasks.iter().map(|t| t.inputs.len()).collect();
    let mut dependents = vec![Vec::new(); n];
    for (v, t) in tasks.iter().enumerate() { for &u in &t.inputs { dependents[u as usize].push(v); } }
    let mut ready: Vec<usize> = (0..n).filter(|&v| indeg[v] == 0).collect();
    let (mut fps, mut outs) = (vec![0u128; n], vec![None::<Arc<[u8]>>; n]);
    let (mut execs, mut head) = (0, 0);
    while head < ready.len() {
        let v = ready[head]; head += 1;
        let t = &tasks[v];
        let key = task_key(v, t, &t.inputs.iter().map(|&u| fps[u as usize]).collect::<Vec<_>>());
        let e = match store.map.get(&key) {
            Some(e) => e.clone(), // early cutoff (inputs' values unchanged) or constructive restore (old version)
            None => {
                let ins: Vec<&[u8]> = t.inputs.iter().map(|&u| &outs[u as usize].as_ref().unwrap()[..]).collect();
                let out = run(v, t, &ins); execs += 1;
                let e = Entry { fp: xxh3_128(&out), out: out.into() };
                store.map.insert(key, e.clone()); e
            }
        };
        fps[v] = e.fp; outs[v] = Some(e.out);
        for &w in &dependents[v] { indeg[w] -= 1; if indeg[w] == 0 { ready.push(w); } }
    }
    assert_eq!(head, n, "dependency cycle");
    (execs, fps)
}

/// Demo task body: task 0 "compiles" source (drops `#` comment lines); others concatenate-and-hash inputs.
fn run(id: usize, t: &Task, ins: &[&[u8]]) -> Vec<u8> {
    if ins.is_empty() { return t.args.split(|&b| b == b'\n').filter(|l| !l.starts_with(b"#")).flatten().copied().collect(); }
    let mut h = Xxh3Default::new(); h.update(&(id as u64).to_le_bytes());
    for i in ins { h.update(&(i.len() as u64).to_le_bytes()); h.update(i); }
    h.digest128().to_le_bytes().to_vec()
}

fn main() {
    let src = |s: &str| s.as_bytes().to_vec();
    let mut tasks = vec![
        Task { version: 1, args: src("# v1\nx = 1"), inputs: vec![] },
        Task { version: 1, args: vec![], inputs: vec![0] },
        Task { version: 1, args: vec![], inputs: vec![0, 1] },
        Task { version: 1, args: vec![], inputs: vec![2] },
    ];
    let mut store = Store::default();
    assert_eq!(build(&tasks, run, &mut store).0, 4);           // cold
    assert_eq!(build(&tasks, run, &mut store).0, 0);           // no-op
    tasks[0].args = src("# reworded\nx = 1");
    assert_eq!(build(&tasks, run, &mut store).0, 1);           // comment-only: leaf re-runs, output fp same ⇒ cutoff
    tasks[0].args = src("# reworded\nx = 2");
    assert_eq!(build(&tasks, run, &mut store).0, 4);           // real change propagates
    tasks[0].args = src("# v1\nx = 1");
    assert_eq!(build(&tasks, run, &mut store).0, 0);           // revert: every key already in the store
    tasks[3].version = 2;
    assert_eq!(build(&tasks, run, &mut store).0, 1);           // bumping a task's version invalidates only it
    println!("ex_a_ca_executor: all assertions passed ({} store entries)", store.map.len());
}
```

**Frontier-only re-keying** (the "CA hybrid" column):

```rust,ignore
// excerpt of /home/claude/graph-hash-harness/engines/src/bin/dag_exec.rs  (executed as part of `cargo run --release --bin dag_exec`)
fn build_hybrid(dag: &Dag, spec: &Spec, st: &mut CaState, edited: &[usize], s: &mut Stats) {
    let n = dag.inputs.len();
    let mut changed = vec![false; n];
    let mut touched = vec![false; n];
    for &e in edited { touched[e] = true; }
    let mut buf = Vec::with_capacity(512);
    for &v in &kahn_order(dag) {
        let v = v as usize;
        if !(touched[v] || dag.inputs[v].iter().any(|&u| changed[u as usize])) { continue; }
        let key = task_key(&mut buf, v, spec, dag.inputs[v].iter().map(|&u| st.fps[u as usize]));
        s.keys += 1;
        let e = match st.store.get(&key) {
            Some(e) => e.clone(),
            None => {
                let ins: Vec<&[u8]> = dag.inputs[v].iter().map(|&u| &st.outs[u as usize][..]).collect();
                let out = execute(v, spec, &ins);
                s.execs += 1;
                let e = Entry { fp: xxh3_128(&out), out: out.into() };
                st.store.insert(key, e.clone());
                e
            }
        };
        if e.fp != st.fps[v] { changed[v] = true; st.fps[v] = e.fp; st.outs[v] = e.out; }
    }
}
```

### 4.5 Rust crates for building executors (crates.io, 2026-10-08)

| Crate | Version / last release | Use | Hashing |
|---|---|---|---|
| salsa | 0.28.5 / 2026-09-24 (edition 2024, MSRV 1.88) | in-process, demand-driven, equality cutoff | FxHash (J1 only) |
| comemo | 0.5.1 / 2026-01-29 | memoize pure functions over tracked contexts | SipHash-1-3/128 (J2) |
| differential-dataflow / timely | 0.25.1 / 0.31.0 (2026-07) | incremental dataflow over change streams, including recursion | `fnv` dependency |
| dbsp | 0.363.0 / 2026-10-08 (near-daily releases) | IVM including transitive closure | XXH3-64 sharding |
| rustc-stable-hash | 0.1.2 / 2025-03-05 | stable `Hasher` wrapper (endianness, `usize` normalised); SipHash128 only | — |
| DICE | not published (git only) | design reference | FxHasher64 |
| adapton / anchors | 2019 / 2021 | dormant | — |

Sources: [crates.io salsa](https://crates.io/crates/salsa), [comemo](https://crates.io/crates/comemo), [differential-dataflow](https://crates.io/crates/differential-dataflow), [dbsp](https://crates.io/crates/dbsp), [rustc-stable-hash](https://crates.io/crates/rustc-stable-hash), [adapton](https://crates.io/crates/adapton).

Incremental dataflow: differential dataflow processed "only 67 differences" when a 24-hour window slid by 1 s on incremental SCC over a Twitter graph, "0.003% of the work done in a full prioritized re-evaluation" ([CIDR 2013](https://www.cidrdb.org/cidr2013/Papers/CIDR13_Paper111.pdf)). Limits: DBSP joins cost O(|DB|×|ΔDB|), and distinct, join and integration need O(|DB|) space ([VLDB 2023](https://www.vldb.org/pvldb/vol16/p1601-budiu.pdf)), so large deltas or tight memory favour periodic full recomputation.

---

## 5. Dependency resolution

### 5.1 Resolver architectures and their hashing (all J1 on interned ids)

| | libsolv 0.7.40 | resolvelib 1.2.1 (pip 26.2.1) | pubgrub 0.4.0 | resolvo 0.12.2 | Cargo resolver |
|---|---|---|---|---|---|
| Core | minisat-style SAT (watched literals, learnt rules) + optimisation policies | provider-driven backtracking + backjumping (with "optimistic backjumping") | CDCL over version-*set* terms, lazy clauses, implicit at-most-one | CDCL SAT, watched literals | DFS backtracking, newest version first |
| Learning | learnt rules | incompatibility info carried over between states | derived incompatibilities with a binary cause DAG | learnt clauses | global conflict trie, **never cleared** |
| Interning | pool `Id` | provider `identify()` key | `HashArena<P>` = `IndexSet<P, FxBuildHasher>` → `Id<P>` (u32) | user `Interner` (`NameId`, `SolvableId`) | leaked `InternedString`/`PackageId`, `ptr::eq` |
| Table hasher | shift-add `strhash` (`r += (r << 3) + c`), `relhash = name + 7*evr + 13*flags` | Python `dict` | FxHash (`Map`/`Set` aliases) | ahash 0.8.12 | Fx in `im_rc` persistent maps (O(1) backtracking snapshots) |
| Errors | problem/solution rules | causes list | `DerivationTree` → prose (`DefaultStringReporter`) | tree report | conflict listing |

Sources: [libsolv hash.h](https://github.com/openSUSE/libsolv/blob/master/src/hash.h), [libsolv-history.txt](https://github.com/openSUSE/libsolv/blob/master/doc/libsolv-history.txt), [resolvelib resolution.py](https://github.com/sarugaku/resolvelib/blob/main/src/resolvelib/resolvers/resolution.py), [pubgrub arena.rs](https://github.com/pubgrub-rs/pubgrub/blob/main/src/internal/arena.rs), [type_aliases.rs](https://github.com/pubgrub-rs/pubgrub/blob/main/src/type_aliases.rs), [resolvo](https://github.com/prefix-dev/resolvo), [cargo conflict_cache.rs](https://github.com/rust-lang/cargo/blob/master/src/resolver/conflict_cache.rs), [context.rs](https://github.com/rust-lang/cargo/blob/master/src/resolver/context.rs), [interning.rs](https://github.com/rust-lang/cargo/blob/master/src/util/interning.rs).

**None of these resolvers uses a bulk hash such as XXH3 for its tables.** Keys are tiny and the work is dominated by integer-id lookups.

**PubGrub** ([solver.md](https://github.com/dart-lang/pub/blob/master/doc/solver.md)): an incompatibility is "a set of terms that are not *all* allowed to be true"; clauses are added lazily, only "when those versions are candidates for selection"; the derivation graph "is a directed acyclic binary graph" whose proof becomes the error message. **pubgrub 0.4.0** [src] ([solver.rs](https://github.com/pubgrub-rs/pubgrub/blob/main/src/solver.rs), [provider.rs](https://github.com/pubgrub-rs/pubgrub/blob/main/src/provider.rs), [CHANGELOG](https://github.com/pubgrub-rs/pubgrub/blob/main/CHANGELOG.md)): published 2026-04-10, MPL-2.0, edition 2024, MSRV 1.92; `resolve<DP: DependencyProvider>(dp, package, version) -> Result<SelectedDependencies, PubGrubError<DP>>`; provider methods `prioritize`, `choose_version`, `get_dependencies`, `should_cancel`; `V` needs only `Ord`, so schema revisions, API levels or dates work as versions; custom reasons travel through `Dependencies::Unavailable(M)` / `External::Custom(P, VS, M)`.

**uv 0.12.24** [src] ([Cargo.toml](https://github.com/astral-sh/uv/blob/main/Cargo.toml), [resolver/mod.rs](https://github.com/astral-sh/uv/blob/main/crates/uv-resolver/src/resolver/mod.rs), [digest.rs](https://github.com/astral-sh/uv/blob/main/crates/uv-cache-key/src/digest.rs), [dirhash.rs](https://github.com/astral-sh/uv/blob/main/crates/uv-extract/src/dirhash.rs), [cache docs](https://docs.astral.sh/uv/concepts/cache/), [resolver internals](https://docs.astral.sh/uv/reference/internals/resolver/)) uses its fork `astral-pubgrub` 0.6.1, which exports `State`, `Id` and `Incompatibility`, and drives the loop itself per `ForkState`: forks split on disjoint markers, and forks with identical packages are merged. Its hashing maps cleanly onto the four jobs: J1 FxHash; J2 **SeaHash u64** cache keys ("stable across releases and platforms"); J3 **SHA-256** artifact hashes in `uv.lock` (1,019 `sha256:` entries in uv's own lock) and **BLAKE3** Merkle dirhashes of unpacked trees. Local-dependency caching is still keyed on **last-modified time**, with hidden inputs declared via `tool.uv.cache-keys`. Lock freshness is structural, not an input hash: "if you change the version constraints such that the existing locked version is still included, the lockfile will still be considered up-to-date" ([sync](https://docs.astral.sh/uv/concepts/projects/sync/#checking-the-lockfile)).

**Cargo.lock** stores "a sha256 checksum of the tarball … so mirror sources can be verified". V2 inlined checksums so that an update "only updates two lines". The default format is V4 ([encode.rs](https://github.com/rust-lang/cargo/blob/master/src/resolver/encode.rs)). The Rust project goal "pubgrub-in-cargo" (2025h1) exists ([goal](https://goals.rust-lang.org/2025h1/pubgrub-in-cargo.html)). There is no 2025h2 or 2026 goal, and cargo HEAD has no pubgrub flag. **The pubgrub README phrase "designated replacement for cargo's solver" is unverified as of 2026-10-08.**

**MVS** (Go modules, Bazel bzlmod) restricts constraints to minimums, so the build list is a per-module maximum over a graph traversal: linear time, "in the intersection of … 2-SAT, Horn-SAT, and Dual-Horn-SAT" ([research.swtch.com/vgo-mvs](https://research.swtch.com/vgo-mvs)), and deterministic without a lockfile ([go.dev/ref/mod](https://go.dev/ref/mod#minimal-version-selection)). Integrity is still cryptographic: `go.sum` `h1:` hashes are two-level Merkle SHA-256 over sorted `"<sha256(file)>  <name>\n"` lines ([dirhash.go](https://github.com/golang/mod/blob/master/sumdb/dirhash/hash.go)), verified against a Merkle transparent log ([checksum database](https://go.dev/ref/mod#checksum-database)). `MODULE.bazel.lock` records `registryFileHashes` of every remote registry file consulted, so "all remote inputs are hashed" for a reproducible resolution ([lockfile](https://bazel.build/external/lockfile)); the algorithm is SHA-256 *[unverified]*.

### 5.2 Executed harness: resolution → topological order → Merkle-fingerprinted lock

`$H/depres` uses pubgrub 0.4.0, xxhash-rust 0.8.19, sha2 0.10.9 and rustc-hash 2.1.3. It was re-run at write time with identical output [exec]:

| Step | Result |
|---|---|
| §1 resolve | Picked `features 1.0.0` over the newer 1.2.0, because 1.2.0 needs `schema 4.x`, which conflicts with `ingest 2.x → schema 3.x`. **No hand pin needed** |
| §2 unsatisfiable `app 2.0.0` | Two-step derivation ending "app 2.0.0 is forbidden" |
| §3 order | `logging -> schema -> theme -> ingest -> features -> report -> app` (Kahn, dependencies first) |
| §4 cycles | `resolve()` accepts `plugin-a ↔ plugin-b` (`ok = true`). Kahn's leftover set `["plugin-a","plugin-b","root"]` includes `root`, which is downstream of the cycle, not in it. Use Tarjan/Kosaraju for exact membership |
| §5 lock | SHA-256 for integrity + XXH3-128 Merkle fingerprint per node; deterministic rebuild identical |
| §6 diff after rebuilding `schema 3.1.0` (same version, new bytes) | Exactly its ancestors changed (`ingest`, `features`, `report`, `app`); `logging` and `theme` unchanged. The cause (sha256 changed) is reported separately from the propagated effect |
| §7 why | `app 1.0.0 --requires[>=1.0.0, <2.0.0]--> features 1.0.0 --requires[>=3.0.0, <4.0.0]--> schema` |
| §8 manifest key | `key(a) = key(b)` for reordered declarations (`f1d26aad02dea3f4d6bf43c22653ce22`); a changed constraint gives a different key |
| §9 3,001 packages × 4 versions | `P=String` 8.52–11.72 ms vs pre-interned `P=u32` 6.64–8.31 ms (≈1.2–1.7×) |
| §9 50 k × 24-B name lookups | SipHash 83–121, **Fx 32–48**, `Xxh3DefaultBuilder` 66–73, `Xxh64Builder` 55–57 ns (includes indirection and string compare) |
| §9 content hashing | 1 MiB: XXH3-128 18.4–18.8 vs SHA-256 1.25–1.27 GiB/s. 64 B: 14.2–21.1 vs 130–185 ns |

```rust,ignore
// excerpt of /home/claude/graph-hash-harness/depres/src/main.rs  (cargo run --release; prints §1–§9 above)
#[derive(Clone, PartialEq)]
struct LockEntry {
    version: String,
    sha256: String, // integrity: crosses trust boundary (download/cache share) -> cryptographic
    fp: u128,       // Merkle fingerprint: XXH3-128(self content || sorted dep fps) -> change detection
}

fn build_lock(g: &Resolved, order: &[String], rebuilds: &HashMap<&str, u32>) -> BTreeMap<String, LockEntry> {
    let mut lock: BTreeMap<String, LockEntry> = BTreeMap::new();
    for n in order {
        // deps-first order guarantees dep fps exist
        let (ver, deps) = &g[n];
        let bytes = artifact(n, ver, *rebuilds.get(n.as_str()).unwrap_or(&0));
        let sha = Sha256::digest(&bytes);
        let mut buf = Vec::with_capacity(128);
        // length-prefixed fields -> unambiguous encoding
        for field in [n.as_bytes(), ver.to_string().as_bytes(), &xxh3_128(&bytes).to_le_bytes()] {
            buf.extend_from_slice(&(field.len() as u32).to_le_bytes());
            buf.extend_from_slice(field);
        }
        for d in deps.keys() {
            // BTreeMap => sorted => order-independent
            buf.extend_from_slice(d.as_bytes());
            buf.push(0);
            buf.extend_from_slice(&lock[d].fp.to_le_bytes());
        }
        lock.insert(n.clone(), LockEntry {
            version: ver.to_string(),
            sha256: sha.iter().map(|b| format!("{b:02x}")).collect(),
            fp: xxh3_128(&buf),
        });
    }
    lock
}

fn diff_lock(a: &BTreeMap<String, LockEntry>, b: &BTreeMap<String, LockEntry>) -> Vec<String> {
    let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    let mut out = vec![];
    for k in keys {
        match (a.get(k), b.get(k)) {
            (Some(x), Some(y)) if x == y => {}
            (Some(x), Some(y)) if x.version != y.version => out.push(format!("~ {k} {} -> {}", x.version, y.version)),
            (Some(x), Some(y)) if x.sha256 != y.sha256 => out.push(format!("~ {k} content changed (sha256 {}.. -> {}..)", &x.sha256[..8], &y.sha256[..8])),
            (Some(_), Some(_)) => out.push(format!("~ {k} fingerprint changed (a dependency changed)")),
            (None, Some(y)) => out.push(format!("+ {k} {}", y.version)),
            (Some(x), None) => out.push(format!("- {k} {}", x.version)),
            (None, None) => unreachable!(),
        }
    }
    out
}
```

Interning helps on the **provider side** (cloning, hashing and comparing `String`s), because pubgrub already interns internally. At thousands of nodes, resolution takes milliseconds, and uv's docs say "The slowest part of resolution in uv is loading package and version metadata". **Interning is a second-order optimisation here. The fingerprinted lock is the first-order one** [inf].

### 5.3 Transferable kit for internal plugin, task and data-product graphs

1. **Per-node manifests with edge constraints**, generated from each module rather than a global hand-ordered list.
2. **Computed resolution**: MVS when your organisation controls compatibility (linear, no lockfile); pubgrub (Rust) or resolvelib (Python) when you need upper bounds, exclusions and readable "why not" explanations. In resolvelib, `identify()` is the interning point: return an `int` or a short interned `str`.
3. **A deterministic lock**: sorted and schema-versioned; per node the resolved version, SHA-256 for external bytes, and an XXH3-128 Merkle fingerprint.
4. **A memo key for re-resolution**: XXH3-128 of the sorted canonical manifest set, so repeated runs are O(1).
5. **Graph-derived explanations**: BFS "why" paths on success, `DerivationTree` on failure.
6. **Explicit cycle policy**: SCC condensation, or an error naming the SCC members (not Kahn leftovers).
7. **Storage in the graph DB**: node records `{name, version, sha256?, fp128}` plus `depends_on` edges carrying the constraint, so "changed since last run" is an indexed fingerprint query and "why" a shortest-path query (§7).
8. **Never use DB record ids or Fx values as lock identity**; use canonical names plus content fingerprints.

Caveats: Cargo-style global leaked interning suits a short-lived CLI but leaks memory in a long-running orchestrator, so scope the arena to one resolution as PubGrub does [inf]. No public case study was found of a non-package-manager system replacing hand-ordered plugin lists with PubGrub; the evidence is the mechanism plus the harness.

---

## 6. Graph algorithms where hashing matters

### 6.1 Merkle hashing over cycles: SCC condensation (J2)

Plain recursive Merkle hashing does not terminate on a cycle, and hashing node ids or DFS visit order breaks relabel invariance. Condense SCCs instead. petgraph's `tarjan_scc` returns SCCs in "postorder (reverse topological sort)", so with edges meaning "u depends on v" every dependency SCC is finished before its dependents ([tarjan_scc.rs](https://docs.rs/crate/petgraph/0.8.3/source/src/algo/scc/tarjan_scc.rs)). Hash each SCC as one unit over sorted, length-prefixed multisets of member labels, internal-edge pairs and child-SCC hashes. Unison does the same: a cycle member hashes to `#x.n` (x = cycle hash), and "a cycle has a canonical order determined by sorting all the members of the cycle by their individual hashes", with 512-bit SHA3 ([Unison hashes](https://www.unison-lang.org/docs/language-reference/hashes), [big idea](https://www.unison-lang.org/docs/the-big-idea/)).

**Measured** on 200,000 nodes and 407,355 edges, giving 190,384 SCCs, of which 2,621 are non-trivial (12,237 nodes, largest 53). Content hash is XXH3-128 over whitespace-normalised text [exec `$H/engines/results/scc_merkle.md`]:

| Operation | SCCs rehashed | Median ms | Speed-up vs full |
|---|---:|---:|---:|
| full recompute (Tarjan + condensation + hash), refined canon | 190,384 | 84.5 | 1× |
| … of which petgraph build + `tarjan_scc` | – | 15.5 | – |
| … of which per-node content hashing | – | 21.0 | – |
| content edit, no dependents | 1 | 0.010 | 8,381× |
| content edit, median cone | 73,579 | 14.1 | 6× |
| content edit, p90 cone | 167,252 | 31.9 | 3× |
| content edit, member of the largest SCC | 121,711 | 23.1 | 4× |
| whitespace-only edit (normalised hash unchanged) | 0 | 0.0022 | – |

The invariance checks passed: 0 failures over 200 random relabelings × 2 canonicalizations at n = 5,000, and the full 200 k graph relabelled identically. Every incremental result was asserted equal to a from-scratch recompute.

A content edit **cannot cut off early** in a Merkle scheme, because every ancestor's hash must change. The incremental cost is therefore exactly the ancestor cone. The median cone here is 39% of all SCCs, so typical speed-ups are 3–6×. **Below ~10⁵ nodes, or with infrequent edits, recompute instead of maintaining incremental state** [inf].

**The symmetric-member weakness.** Any relabel-invariant hash must give automorphic members equal labels, which is correct. The bug is giving *non-isomorphic* SCCs equal hashes [exec]:

| SCC pair (all members have identical content) | Isomorphic? | Sorted member hashes | WL-refined | Exact canon (k ≤ 8) | Identity in content |
|---|---|---|---|---|---|
| 6-cycle + chord 0→2 vs 6-cycle + chord 0→3 | no | **same hash** | different | different | n/a |
| circulant C7(1,2) vs C7(1,3) (both 2-in/2-out regular) | no | **same hash** | **same hash** | different | different |
| 6-cycle + chord 0→2 vs chord 3→5 (relabelled) | yes | same | same | same | n/a |

Fixes, ranked: (1) **put a stable identity (path, name, key) into each node's content**, so labels are unique and sorting is an exact canonical form at O(k log k), which is right for code and dependency graphs; (2) WL refinement inside the SCC, O(rounds · internal edges), which resolves irregular SCCs but not regular ones; (3) exact canonical labelling, exponential in the worst case (brute force for k ≤ 8 here; nauty/bliss not tested); (4) if identity must be content-only (dedup across renames), use the hash as a bucket key and confirm with an exact isomorphism check.

```rust
// harness: /home/claude/graph-hash-harness/engines (bin ex_b_scc_merkle)
//! Excerpt (b): order-independent Merkle hash of a CYCLIC dependency graph via SCC condensation.
use petgraph::{algo::tarjan_scc, graph::{DiGraph, NodeIndex}};
use xxhash_rust::xxh3::{xxh3_128, Xxh3Default};

fn put(h: &mut Xxh3Default, mut xs: Vec<u128>) { // feed a multiset: sort, length-prefix
    xs.sort_unstable();
    h.update(&(xs.len() as u64).to_le_bytes());
    for x in xs { h.update(&x.to_le_bytes()); }
}

/// Edge u→v = "u depends on v". Returns a hash per node. Node indices never enter any hash.
pub fn scc_merkle(content: &[&[u8]], g: &DiGraph<(), ()>) -> Vec<u128> {
    let sccs = tarjan_scc(g); // emitted dependencies-first (reverse topological order of the condensation)
    let mut comp = vec![0usize; g.node_count()];
    for (i, s) in sccs.iter().enumerate() { for v in s { comp[v.index()] = i; } }
    let (mut scc_h, mut node_h) = (vec![0u128; sccs.len()], vec![0u128; g.node_count()]);
    for (i, s) in sccs.iter().enumerate() {
        let k = s.len();
        let pos = |v: NodeIndex| s.iter().position(|&x| x == v).unwrap();
        let (mut succ, mut pred, mut ext) = (vec![vec![]; k], vec![vec![]; k], vec![vec![]; k]);
        for (a, &v) in s.iter().enumerate() {
            for w in g.neighbors(v) {
                if comp[w.index()] == i { let b = pos(w); succ[a].push(b); pred[b].push(a); }
                else { ext[a].push(scc_h[comp[w.index()]]); } // child SCC already hashed
            }
        }
        // member labels = content hash, then colour refinement (WL) inside the SCC so equal-content members
        // in different roles get different labels. k−1 rounds always suffice (scc_merkle.rs stops once stable).
        let mut lab: Vec<u128> = s.iter().map(|v| xxh3_128(content[v.index()])).collect();
        for _ in 1..k {
            lab = (0..k).map(|a| {
                let mut h = Xxh3Default::new(); h.update(&lab[a].to_le_bytes());
                put(&mut h, succ[a].iter().map(|&b| lab[b]).collect());
                put(&mut h, pred[a].iter().map(|&b| lab[b]).collect());
                put(&mut h, ext[a].clone());
                h.digest128()
            }).collect();
        }
        let pair = |x: u128, y: u128| xxh3_128(&[x.to_le_bytes(), y.to_le_bytes()].concat());
        let mut h = Xxh3Default::new(); h.update(b"scc/v1");
        put(&mut h, lab.clone());                                                             // members
        put(&mut h, (0..k).flat_map(|a| succ[a].iter().map(move |&b| (a, b))).map(|(a, b)| pair(lab[a], lab[b])).collect()); // internal edges
        put(&mut h, (0..k).flat_map(|a| ext[a].iter().map(move |&c| (a, c))).map(|(a, c)| pair(lab[a], c)).collect());       // edges to children
        scc_h[i] = h.digest128();
        for (a, v) in s.iter().enumerate() { node_h[v.index()] = pair(scc_h[i], lab[a]); }
    }
    node_h
}

fn graph(n: usize, edges: &[(usize, usize)]) -> DiGraph<(), ()> {
    let mut g = DiGraph::new(); for _ in 0..n { g.add_node(()); }
    for &(a, b) in edges { g.add_edge(NodeIndex::new(a), NodeIndex::new(b), ()); } g
}

fn main() {
    // a ↔ b cycle depending on c; d depends on the cycle
    let content: Vec<&[u8]> = vec![b"a", b"b", b"c", b"d"];
    let base = scc_merkle(&content, &graph(4, &[(0, 1), (1, 0), (1, 2), (3, 0)]));
    // same graph, relabelled (perm: old i → new p[i]) and inserted in a different order
    let p = [2, 0, 3, 1];
    let mut c2: Vec<&[u8]> = vec![b""; 4]; for i in 0..4 { c2[p[i]] = content[i]; }
    let perm = scc_merkle(&c2, &graph(4, &[(p[3], p[0]), (p[1], p[2]), (p[1], p[0]), (p[0], p[1])]));
    assert!((0..4).all(|i| base[i] == perm[p[i]]), "relabel invariance");
    // weakness: two non-isomorphic 2-regular circulants on 7 identical-content nodes are indistinguishable
    let circ = |s: [usize; 2]| graph(7, &(0..7).flat_map(|i| s.map(|d| (i, (i + d) % 7))).collect::<Vec<_>>());
    let same = vec![b"same".as_slice(); 7];
    assert_eq!(scc_merkle(&same, &circ([1, 2]))[0], scc_merkle(&same, &circ([1, 3]))[0]); // collision!
    // fix: put a stable identity (path/name) in each node's content ⇒ labels unique ⇒ exact
    let named: Vec<Vec<u8>> = (0..7).map(|i| format!("crate::f{i}").into_bytes()).collect();
    let nr: Vec<&[u8]> = named.iter().map(|v| v.as_slice()).collect();
    assert_ne!(scc_merkle(&nr, &circ([1, 2]))[0], scc_merkle(&nr, &circ([1, 3]))[0]);
    println!("ex_b_scc_merkle: all assertions passed");
}
```

**Incremental update with early stop**: re-hash only the SCC containing the change and its ancestor SCCs, in Tarjan order:

```rust,ignore
// excerpt of /home/claude/graph-hash-harness/engines/src/bin/scc_merkle.rs  (executed by `cargo run --release --bin scc_merkle`)
fn update_content(g: &mut G, m: &mut Merkle, v: usize, new_content: Vec<u8>, canon: Canon) -> usize {
    g.content[v] = new_content;
    let nc = content_hash(&g.content[v]);
    if nc == m.chash[v] { return 0; }                 // whitespace-only edits stop here
    m.chash[v] = nc;
    let mut heap = BinaryHeap::new();
    let mut queued = vec![false; m.members.len()];
    heap.push(Reverse(m.comp[v])); queued[m.comp[v] as usize] = true;
    let mut rehashed = 0; let mut nh = Vec::new();
    while let Some(Reverse(s)) = heap.pop() {
        let s = s as usize;
        nh.clear();
        let d = hash_scc(g, m, s, canon, &mut nh);
        rehashed += 1;
        for &(x, hx) in &nh { m.node_hash[x as usize] = hx; }
        if d == m.scc_hash[s] { continue; }            // SCC hash unchanged → stop propagating
        m.scc_hash[s] = d;
        for &p in &m.parents[s] { if !queued[p as usize] { queued[p as usize] = true; heap.push(Reverse(p)); } }
    }
    rehashed
}
```

A second, Unison-style implementation (`$H/algos/src/bin/merkle_scc.rs`: cycle-removed member hash → up to |SCC| refinement rounds → sort → cycle hash with internal references as canonical positions → member id = H(cycle, index)) on a 9-package graph with a 3-cycle `parser↔expr↔stmt` [exec]: relabel and insertion-order invariant, 0 unresolved ties; bumping `tls` changes exactly `["app","http","tls","web"]`, which a root-down Merkle diff finds in 8 node visits; a symmetric 2-cycle with identical bodies gives `ties = 1` (members are automorphic, so the cycle hash is unaffected, but which member gets index 0 is arbitrary).

### 6.2 Weisfeiler–Lehman hashing (J2/J4)

1-WL colour refinement with a fingerprint hash in place of the paper's relabelling dictionary yields an isomorphism-*invariant* graph fingerprint. The paper notes that "any other injective mapping will give equivalent results" and that runtime "scales only linearly in the number of edges" ([JMLR 2011](https://www.jmlr.org/papers/v12/shervashidze11a.html)). The 128-bit fingerprint matters because it makes WL **distributable**: no shared dictionary is needed, and labels are comparable across processes and languages.

**Measured** with XXH3-64 over CSR on n = 200,000 and m = 1,000,000, i.e. 2 M adjacency entries [exec `$H/engines/results/wl_hash.md`]:

| WL step variant | ms per iteration | M adjacency entries/s |
|---|---:|---:|
| sort-based, 1 thread | 29.4 | 68 |
| sort-based, rayon (2 threads) | 14.9 | 134 |
| commutative-sum aggregate (no sort), 1 thread | 13.4 | 149 |
| full graph hash h = 3, 1 thread | 109.7 | – |

Invariance held: 0 mismatches over 20 relabelings plus edge shuffles. The sum aggregator gave the same 170,320 distinct labels after one step, but its combiner is linear, so prefer the sorted multiset for anything persisted or adversarial.

**Rust vs Python** on a 20 k / 100 k graph with h = 3 [exec `wl_python.md`]:

| Implementation | Time | Graph hash |
|---|---|---|
| Rust | 8.64 ms | `d538d9855c782623` |
| networkx 3.6.1 `weisfeiler_lehman_graph_hash` (blake2b, string labels) | 221 ms | — |
| pure-Python xxh3 port | 266 ms | `d538d9855c782623` (bit-identical to Rust) |
| networkx with `_hash_label` patched to xxh3_128 | 182 ms (−18%) | — |

Hash calls alone take 11.6 ms with blake2b and 2.5 ms with xxh3, so Python overhead dominates. The comparison favours networkx, which runs one fewer refinement step on attribute-less graphs. A second run in the columnar harness agreed: 419 → 348 ms (−17%) [exec].

**Limits, measured.**

| Limit | Evidence |
|---|---|
| **1-WL is incomplete** | Collides at every h for 2×C3 vs C6 (`ex_c_wl`, h = 1..6), decalin vs bicyclopentyl, and the strongly regular rook 4×4 vs Shrikhande (both srg(16,6,2,2)) [exec `$H/algos/src/bin/wl_hash.rs`, `$H/engines/results/wl_networkx_collisions.txt`]. networkx collides on the same pairs, though its docs claim "strong guarantees that non-isomorphic graphs will get different hashes" ([docs](https://networkx.org/documentation/stable/reference/algorithms/generated/networkx.algorithms.graph_hashing.weisfeiler_lehman_graph_hash.html)) |
| **Regular graphs always tie** | 1-WL gives every vertex of a regular graph the same colour, so all same-size, same-degree regular graphs collide ([Sato survey](https://arxiv.org/pdf/2003.04078)). Dependency graphs are rarely regular, but replicated services and fan-out stages produce ties [inf] |
| **networkx bug: concatenation collisions** | Neighbour labels are joined with no separators (`node_labels[node] + "".join(sorted(label_list))`); node labels `(1, 1111)` and `(11, 111)` collide at iterations 1, 3 and 5 (both aggregate to `"11111"`) [exec `$H/algos/py/wl_probe2.py`]. The Rust version with fixed-width u128 labels does not collide |
| **networkx bug: directed + `edge_attr` drops direction** | `prefix = "s_" + "" if edge_attr is None else str(...)` parses as `("s_" + "") if … else …`, so the `s_`/`p_` markers vanish when `edge_attr` is given; out-star vs in-star and path vs out-fork collide [exec `$H/algos/py/wl_probe.py`, networkx 3.7]. Harness finding, not checked against the issue tracker |
| **Version-sensitive values** | Directed and attribute-less hashes changed in networkx 3.5 ([graph_hashing.py](https://github.com/networkx/networkx/blob/main/networkx/algorithms/graph_hashing.py), [#7806](https://github.com/networkx/networkx/issues/7806)) |

Correct dedup pipeline [inf]: a WL-128 fingerprint as the bucket key, then exact confirmation inside the bucket (VF2 via `petgraph::algo::is_isomorphic_matching` for a few candidates, or `nauty_pet::IntoCanon` → XXH3-128 over the canonical adjacency as the exact id). In Python, run the loop natively (Rust via PyO3); swapping the hash inside networkx is not enough.

```rust
// harness: /home/claude/graph-hash-harness/engines (bin ex_c_wl)
//! Excerpt (c): Weisfeiler–Lehman subtree graph hash with XXH3-64 over a CSR adjacency.
use xxhash_rust::xxh3::xxh3_64;

pub struct Csr { pub off: Vec<u32>, pub adj: Vec<u32> } // undirected: each edge stored in both rows

pub fn csr(n: usize, edges: &[(u32, u32)]) -> Csr {
    let mut off = vec![0u32; n + 1];
    for &(a, b) in edges { off[a as usize + 1] += 1; off[b as usize + 1] += 1; }
    for i in 0..n { off[i + 1] += off[i]; }
    let (mut pos, mut adj) = (off.clone(), vec![0u32; off[n] as usize]);
    for &(a, b) in edges {
        adj[pos[a as usize] as usize] = b; pos[a as usize] += 1;
        adj[pos[b as usize] as usize] = a; pos[b as usize] += 1;
    }
    Csr { off, adj }
}

fn as_bytes(v: &[u64]) -> &[u8] { unsafe { std::slice::from_raw_parts(v.as_ptr().cast(), v.len() * 8) } } // LE on x86/aarch64

/// h iterations; label' = xxh3(own ‖ sorted neighbour labels); graph hash folds each iteration's sorted label multiset.
pub fn wl_hash(g: &Csr, h: usize, init: impl Fn(usize) -> u64) -> u64 {
    let n = g.off.len() - 1;
    let mut lab: Vec<u64> = (0..n).map(init).collect();
    let (mut next, mut buf, mut per_iter) = (vec![0u64; n], Vec::new(), Vec::with_capacity(h));
    for _ in 0..h {
        for v in 0..n {
            buf.clear(); buf.push(lab[v]);
            buf.extend(g.adj[g.off[v] as usize..g.off[v + 1] as usize].iter().map(|&w| lab[w as usize]));
            buf[1..].sort_unstable(); // multiset ⇒ order-independent
            next[v] = xxh3_64(as_bytes(&buf));
        }
        std::mem::swap(&mut lab, &mut next);
        let mut s = lab.clone(); s.sort_unstable();
        per_iter.push(xxh3_64(as_bytes(&s)));
    }
    xxh3_64(as_bytes(&per_iter))
}

fn main() {
    let deg = |g: &Csr| { let o = g.off.clone(); move |v: usize| (o[v + 1] - o[v]) as u64 };
    // permutation invariance on a small graph
    let e = [(0, 1), (1, 2), (2, 3), (3, 0), (0, 2), (3, 4)];
    let p = [3u32, 0, 4, 1, 2];
    let pe: Vec<(u32, u32)> = e.iter().rev().map(|&(a, b)| (p[b as usize], p[a as usize])).collect();
    let (g, gp) = (csr(5, &e), csr(5, &pe));
    assert_eq!(wl_hash(&g, 3, deg(&g)), wl_hash(&gp, 3, deg(&gp)));
    // limitation: 1-WL cannot tell 2 disjoint triangles from a 6-cycle (both 2-regular on 6 nodes)
    let tri2 = csr(6, &[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3)]);
    let c6 = csr(6, &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 0)]);
    for h in 1..=6 { assert_eq!(wl_hash(&tri2, h, deg(&tri2)), wl_hash(&c6, h, deg(&c6))); }
    println!("ex_c_wl: all assertions passed (WL collision on 2×C3 vs C6 reproduced for h=1..6)");
}
```

### 6.3 Canonical labelling (exact identity)

Theory: McKay & Piperno, refinement–individualisation, nauty and Traces ([arXiv 1301.1493](https://arxiv.org/abs/1301.1493)). **`nauty-pet` 0.15.0** (2026-08-16), "Canonical graph labelling using nauty/Traces and petgraph" ([docs.rs](https://docs.rs/nauty-pet/latest/nauty_pet/)): traits `IntoCanon`, `IntoCanonNautySparse`, `TryIntoCanonTraces`, `IsIdentical`; automorphism groups; a `stable` feature for deterministic behaviour when weights compare equal; petgraph `>=0.7, <=0.8`; depends on `nauty-Traces-sys` 0.11; uses ahash internally; crate license Apache-2.0 (nauty's own license unverified). Also `canonical-form` 0.12.0 and `graph-canon` 0.1.4 (2023); the crates.io name `bliss-rs` is an unrelated audio library ([crates.io](https://crates.io/crates/canonical-form)). petgraph has pairwise VF2 (`is_isomorphic`, `is_isomorphic_matching`, `subgraph_isomorphisms_iter`) but no canonical form ([isomorphism.rs](https://docs.rs/crate/petgraph/0.8.3/source/src/algo/isomorphism.rs)). Not benchmarked: use canonical labelling only where an exact content id for an unlabelled structure is required.

### 6.4 Sketches (J4)

| Use | Crate (version) | Hash input | Source |
|---|---|---|---|
| Neighbourhood Jaccard, near-duplicate dependency sets, link prediction | probminhash 0.1.12 (ProbMinHash, SuperMinHash, SetSketch), gaoya 0.2.2 (LSH), datasketches 0.5.0 | Each neighbour id via XXH3-64 or `foldhash::quality`; LSH banding avoids O(n²) | ([superminhasher.rs](https://docs.rs/crate/probminhash/0.1.12/source/src/superminhasher.rs), [SetSketch](https://arxiv.org/abs/2101.00314)) |
| Membership pre-check before a DB round trip | xorf 0.13.0 `Xor8/16/32`, `BinaryFuse8/16/32` — "does not provide methods for hashing arbitrary types", immutable, no false negatives | XXH3-64 of the key | ([xorf lib.rs](https://docs.rs/crate/xorf/0.13.0/source/src/lib.rs)) |
| Growing Bloom sets | fastbloom 0.17.0 (default: randomly keyed **SipHash-1-3**, so not persistable as-is); growable-bloom-filter 2.1.1 (xxhash-rust) | `with_false_pos(fp)` | ([fastbloom hasher.rs](https://docs.rs/crate/fastbloom/0.17.0/source/src/hasher.rs)) |
| Per-node reach / neighbourhood function at scale | webgraph-algo 0.6.2 HyperBall | HLL counters, merged by register-wise max along edges, O(n log log n) memory; "a few hours … graphs with billions of nodes" | ([HyperANF](https://arxiv.org/abs/1011.5599), [webgraph-algo](https://docs.rs/webgraph-algo/latest/webgraph_algo/)) |
| Cardinality inside query engines | Polars streaming joins import `CardinalitySketch` and `F2Sketch`; DuckDB 2.0 preview uses HLL to decide pre-aggregation | engine-internal | ([hash_keys.rs](https://github.com/pola-rs/polars/blob/main/crates/polars-expr/src/hash_keys.rs), [DuckDB blog](https://duckdb.org/2026/08/25/how-duckdb-runs-recursive-ctes-faster)) |

Rule: seeds must be fixed and versioned whenever sketches are persisted or compared across processes. Use `xxh3_64_with_seed` (stable since XXH3 0.8.0). The only sketch property measured here is uniformity under truncation (§1.1); no sketch crate was benchmarked.

### 6.5 Pregel-style hash partitioning

"The default partitioning function is just hash(ID) mod N … but users can replace it" ([Pregel, SIGMOD 2010](https://kowshik.github.io/JPregel/pregel_paper.pdf)). DBSP shards records across workers with `Xxh3Default` through the `Hash` trait ([hash.rs](https://github.com/feldera/feldera/blob/main/crates/dbsp/src/hash.rs)). Rules [inf]: partition on a well-mixed hash (XXH3-64 or an Fx-mixed id, never a raw sequential id mod N); if shard assignment is persisted across upgrades, hash explicit bytes, not `Hash` impls; hash partitioning ignores locality, which is why Pregel lets users override it.

### 6.6 E-graphs and hash-consing (J1)

An e-graph is a hashcons (`memo: HashMap<ENode, Id>` with canonicalised children) plus a union-find. egg's contribution is **deferred rebuilding**: let unions accumulate, then re-canonicalise pending e-nodes in one pass, so that memo collisions found during the pass *are* the congruences. Under the traditional strategy, merging x with y₁…yₙ under f₁(x)…fₙ(x) "could require O(n²) hashcons updates. With deferred rebuilding… no more than O(n)". In aggregate, "congruence is 88× faster, and equality saturation is 21× faster" (geometric mean over 32 tests) ([egg paper](https://arxiv.org/abs/2004.03082)).

Both egg 0.11.0 and egglog 3.0.0 use FxHash ([egg util.rs](https://docs.rs/crate/egg/0.11.0/source/src/util.rs), [egglog util.rs](https://docs.rs/crate/egglog/3.0.0/source/src/util.rs)). Executed in `$H/algos/src/bin/hashcons_egg.rs` [exec]: a hand-rolled hashcons collapsed a depth-20 full binary tree (2,097,151 constructor calls) to 21 nodes; egg's `f(x)==f(y)` was false before `rebuild()` and true after (`classes=2`); equality saturation simplified `(+ 0 (* (+ a (* b 0)) 1))` to `a` (cost 1; 8 e-nodes; `Saturated`).

Keep plain rewriting when one confluent, terminating rewrite system already exists. Watch e-graph memory: an e-graph can blow up without limits [inf].

### 6.7 Datalog engines replacing hand-written fixed points

| Engine (version) | Join strategy | Hashing | Notes |
|---|---|---|---|
| datafrog 2.0.1 (2019; used by polonius-engine 0.13.0) | sorted `Vec` relations, galloping merge joins, leapjoins | **none** (sorting) | ([datafrog lib.rs](https://docs.rs/crate/datafrog/2.0.1/source/src/lib.rs)) |
| ascent 0.8.1 | hash indexes, lattices, rayon | `DashMap<_, _, BuildHasherDefault<FxHasher>>` | ([c_lat_index.rs](https://docs.rs/crate/ascent/0.8.1/source/src/c_lat_index.rs)) |
| crepe 0.2.0 | semi-naive evaluation, stratified negation | std `HashMap`; `run_with_hasher::<S>()` swaps it | ([crepe lib.rs](https://docs.rs/crate/crepe/0.2.0/source/src/lib.rs)) |
| differential-dataflow 0.25.1 | incremental, nested iteration | `fnv` | active; DDlog's repository was archived 2026-07-13 ([DDlog](https://github.com/vmware/differential-datalog)) |

Transitive closure on a 300-node chain with a back-edge cycle and a side branch [exec `$H/algos/src/bin/datalog_reach.rs`]: both the naive loop and datafrog produced **44,918 tuples in 299 rounds**; the naive loop did **2,710,454,832 join probes**; wall time (single runs, indicative) naive 2.38–2.40 s vs datafrog 9.8–11.1 ms (a write-time re-run gave 1.39 s vs 6.3 ms). A second datafrog program, `affected(x) :- depends_on(x, y), affected(y)`, found the 8 nodes affected by changing `1001`.

**Strawman caveat.** The naive loop re-joins all of `tc` with a nested-loop join every round, so the ratio overstates the gap against a careful hand-written worklist. The real win is **semi-naive evaluation** (join only each round's delta) plus engine-chosen indexes, not the hash function. Keep hand-written code for single-relation reachability (BFS/DFS with `FixedBitSet` is simpler and faster), non-monotone logic, or when macro-based Datalog compile times (ascent, crepe) hurt [inf]. Datalog wins once several mutually recursive relations, negation or many join patterns appear (borrow checking, points-to analysis, "affected-by-change" over several edge kinds). Fingerprints (`u128`) are `Ord`, so fingerprint-keyed relations work: put the join key first in the tuple.

```rust,ignore
// excerpt of /home/claude/graph-hash-harness/algos/src/bin/datalog_reach.rs
fn datafrog_closure(edges: &[(u32, u32)]) -> (Relation<(u32, u32)>, usize) {
    let mut it = Iteration::new();
    let edge: Relation<(u32, u32)> = edges.iter().copied().collect();          // (src, dst)
    let tc_by_dst = it.variable::<(u32, u32)>("tc_by_dst");                    // (mid, src): join key first
    tc_by_dst.extend(edges.iter().map(|&(a, b)| (b, a)));
    let mut rounds = 0;
    while it.changed() {
        rounds += 1;
        // tc(a, d) :- tc(a, b), edge(b, d).   Only the *delta* (recent tuples) is joined.
        tc_by_dst.from_join(&tc_by_dst, &edge, |&_b, &a, &d| (d, a));
    }
    let out = tc_by_dst.complete();
    (Relation::from_iter(out.iter().map(|&(d, a)| (a, d))), rounds)
}
```

### 6.8 Incremental topological order

`petgraph::acyclic::Acyclic<G>` maintains a topological order under edge insertion using Pearce–Kelly, with `FixedBitSet` discovered/finished sets ([acyclic.rs](https://docs.rs/crate/petgraph/0.8.3/source/src/acyclic.rs), [Pearce & Kelly](http://www.doc.ic.ac.uk/~phjk/Publications/DynamicTopoSortAlg-JEA-07.pdf)). It is worst-case linear per insertion but "fast in practice, particularly on sparse graphs". incremental-topo 0.3.1 implements the same algorithm with `FnvHashSet` ([lib.rs](https://docs.rs/crate/incremental-topo/0.3.1/source/src/lib.rs)). **No maintained Rust crate for dynamic SCC was found** ([crates.io search](https://crates.io/search?q=incremental%20scc)).

```rust
// harness: /home/claude/graph-hash-harness/algos (bin acyclic_incremental)
//! petgraph::acyclic::Acyclic keeps a topological order under edge insertion
//! (Pearce-Kelly) instead of re-running toposort / cycle detection per insert.
use petgraph::acyclic::Acyclic;
use petgraph::data::Build; // Acyclic::add_node comes from the Build trait
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;

fn main() {
    let mut dag: Acyclic<DiGraph<&str, ()>> = Acyclic::new();
    let [a, b, c, d] = ["fetch", "build", "test", "deploy"].map(|n| dag.add_node(n));
    for (x, y) in [(a, b), (b, c), (c, d)] {
        dag.try_add_edge(x, y, ()).unwrap();
    }
    let order: Vec<_> = dag.nodes_iter().map(|n| dag.inner()[n]).collect();
    println!("maintained topo order: {order:?}");
    match dag.try_add_edge(d, a, ()) {
        Err(e) => println!("deploy -> fetch rejected: {e:?}"),
        Ok(_) => println!("unexpectedly accepted"),
    }
    // Clunky alternative: mutate a plain graph, then re-run full toposort to detect the cycle (O(V+E) per insert).
    let mut g: DiGraph<&str, ()> = dag.inner().clone();
    g.add_edge(d, a, ());
    println!("plain DiGraph + toposort after insert: is_err = {}", toposort(&g, None).is_err());
}
```

The output, re-run at write time: `maintained topo order: ["fetch", "build", "test", "deploy"]`, then `deploy -> fetch rejected: Cycle(Cycle(NodeIndex(0)))`, then `is_err = true`. Keep batch construction (build everything, then one O(V+E) `toposort`) when edges arrive all at once.

---

## 7. Graph databases: SurrealDB 3.3 (Rust) and ArcadeDB 26.10 (Python)

Executed against SurrealDB **3.3.2** (crate `surrealdb =3.3.2`, `kv-mem`, dev build at opt-level 1; `$H/surreal`) and ArcadeDB **26.10.1** (local server from the release tarball, `-Xmx1g`, Python 3.13.16 + python-xxhash 4.0.1 over plain HTTP/JSON; `$H/arcade`). Timings are indicative only: shared 2-vCPU host, SurrealDB not in release mode, ArcadeDB over HTTP.

### 7.1 Side-by-side reference

| Concern | SurrealDB 3.3.2 from Rust | ArcadeDB 26.10.1 from Python |
|---|---|---|
| Release / license | 3.3.2 published 2026-10-08; 2.x still patched (2.7.0). **BSL 1.1**: "you may not use the Licensed Work as a Database Service"; Change Date 2030-01-01 → Apache 2.0 [src LICENSE] ([crates.io](https://crates.io/crates/surrealdb)) | 26.10.1 tagged 2026-10-05 (commit `f8cd5c0`); monthly releases; **Apache-2.0**; Java 21 or 25 (17 not compatible) ([tags](https://github.com/ArcadeData/arcadedb/tags), [requirements](https://docs.arcadedb.com/arcadedb/reference/requirements.html)) |
| Client API | 3.x is a breaking redesign. `#[derive(SurrealValue)]` replaces serde ("The derive does not read `#[serde(...)]` attributes"); `RecordId::new(table, key)`; `db.query(sql).bind((k, v))`, `.take::<T>(i)`; `db.clone().begin()` → `.commit()`/`.cancel()` ([Rust after 3.0](https://surrealdb.com/docs/sdk/rust/concepts/rust-after-30)) | Plain HTTP `/api/v1/command|query/{db}` with `:name` params; official `arcadedb-driver` 0.2.0 (released 2026-10-06; the driver is a month old, first released 2026-09-04; `timeout=None` means **no timeout**); psycopg 3 over the Postgres wire protocol (needs ≥26.10.1); neo4j driver over Bolt (Cypher only) ([HTTP API](https://docs.arcadedb.com/arcadedb/reference/http-api/http.html), [PyPI](https://pypi.org/project/arcadedb-driver/)) |
| Server-side xxHash | **none**; built-ins are `crypto::md5/sha1/sha256/sha512/blake3/joaat` [src] | **none**; `.hash('MD5'\|'SHA-256')` works and `.hash('XXH3')` fails [exec]; Cypher `util.md5/sha*` ([utility](https://docs.arcadedb.com/arcadedb/reference/extended-functions/utility.html)) |
| Integer type | i64. A raw u64 > i64::MAX becomes a **string** key in id position and a **parse error** in value position [exec] | LONG = signed 64-bit. Raw u64 ≥ 2⁶³ is rejected server-side over HTTP and psycopg, and client-side by the neo4j driver (`OverflowError`) [exec] |
| Node identity | `RecordId::new("func", xxh3_64(fq_name) as i64)`, point-looked-up by `RecordIdScan` with **no index** [exec]. Lossless: `(k as u64) == h` | `key128`: 32-hex STRING with `UNIQUE_HASH` on a common supertype (O(1) equality); optional `key64` LONG under an LSM `UNIQUE` index for ordering [exec] |
| 128-bit keys | `memo:<32-hex>` string, `memo:[hi as i64, lo as i64]` array (prefix-rangeable), or `memo:u'…'` UUID: all three passed get-or-compute [exec] | 32-hex STRING. **BINARY(16) is unusable** for indexed equality in 26.10.1: `WHERE k = :k` fails with "Invalid binary type for comparison" on indexed properties only [exec]. Two LONG columns under a composite index: untested |
| Internal ids | Record ids are your keys; KV layout `/*{ns}*{db}*{tb}*{id}`; signed ints are stored as `(v ^ i64::MIN).to_be_bytes()` (order-preserving) [src storekey] | **RIDs (`#bucket:pos`) are reused after delete**: `#61:2` was deleted and then reassigned [exec]. There is no per-record `@version` in SQL (MVCC is per page) [exec] |
| Idempotent node upsert | `UPSERT $id SET …` (checks unique indexes); `INSERT INTO t $rows ON DUPLICATE KEY UPDATE h = $input.h` (fires on an id **or** a unique-index conflict) [exec] | `UPDATE T SET … UPSERT WHERE key128 = :k`, atomic only when the WHERE is an equality on a UNIQUE index; **refused** otherwise ("Upsert must involve an index") [exec]; Cypher `MERGE (n:L {key128:$k})` [exec] |
| Edge dedup | (1) `LIGHTWEIGHT` relation (3.3.0+): the edge id **is** `[in, out]` and repeated RELATE returns the same record, but edges carry no data. (2) A deterministic edge id (`calls:[in,out]` or a seeded xxh3 of the tuple) + `INSERT RELATION … ON DUPLICATE KEY UPDATE`/`IGNORE`. (3) `UNIQUE(in, out)` index [exec] | `CREATE INDEX ON E (`@out`,`@in`) UNIQUE` + `CREATE EDGE … IF NOT EXISTS`, or a hashed `ekey` under `UNIQUE_HASH` on the edge supertype. **There is no UPSERT for edges** ([CREATE EDGE](https://docs.arcadedb.com/arcadedb/reference/sql/sql-create-edge.html)) [exec] |
| Change detection | (a) pre-write manifest diff `SELECT VALUE id FROM $m WHERE id.content_hash != h`; (b) `prev_hash` + an indexed `VALUE`-computed `dirty` bool (`IndexScan`), since field-vs-field predicates are `TableScan`; (c) `CHANGEFEED` + `SHOW CHANGES SINCE <versionstamp>`; (d) `LIVE SELECT` (works embedded) [exec] | (a) `UNWIND $pairs AS p MATCH (n {key128: p.k}) WHERE n.content_hash <> p.h`; (b) guarded `UPDATE … WHERE key128 = :k AND content_hash <> :h` (returns count 0 or 1); (c) `/ws` change feed, where **identical rewrites emit no events** [exec] |
| Traversal | `->e->t`, `<-`, `<->`; recursion `.{n}`, `.{1..4}`, `.{..}` with `+collect`, `+path`, `+shortest=<id>`, `+inclusive`; bounds 1–256 ([idioms](https://surrealdb.com/docs/surrealql/datamodel/idioms)) | SQL `TRAVERSE … [MAXDEPTH] [STRATEGY BREADTH_FIRST]` (**depth-first by default**), `MATCH {…}.out(){while:…}`, `shortestPath` (default direction is **BOTH**, as executed; the docs contradict themselves), `dijkstra`, `astar`, `bellmanFord`; OpenCypher; **71 algorithm procedures/functions** in the docs index ([graph algorithms](https://docs.arcadedb.com/arcadedb/reference/graph-algorithms/chapter.html)) |
| Whole-graph algorithms | **none built in** (no PageRank, SCC or toposort) [src] → pull the graph into Rust (§3) | `algo.topologicalSort` (Kahn; cycle members get `order = -1`), `cycleDetection` (Kosaraju), `scc`, `wcc`, `pagerank`, `louvain`, … They run over **every vertex in the database** and yield RIDs [exec] |
| Memo / cache table | `memo` table keyed by an XXH3-128 Merkle key; get-or-compute in one round trip; or a synchronous `DEFINE EVENT` that deletes dependent memos [exec] | `Memo` document keyed by `fp` (`UNIQUE_HASH`), where fp = XXH3-128 over the closure's sorted `(key128, content_hash)` pairs; in the same ACID database as the graph [exec] |
| Collision guard | `DEFINE FIELD name ON t TYPE string READONLY`, or `ASSERT $before = NONE OR $before = $value`: an id collision becomes a hard error [exec] | `key128 … (mandatory true, notnull true, readonly true)` + UNIQUE index; duplicate keys raise `DuplicatedKeyException` [exec] |
| Idempotent transport | — | `X-Request-Id` replay: the same id sent twice produced 2 × HTTP 200 and **1 row**. Not applied inside `/begin` sessions, NDJSON streams or `/api/v1/batch` [exec] |
| Build / ops cost | Cold build ≈ **1,085 s** with `-j 2`; `target/` 1.6 GB; even `kv-mem` compiles C (`aws-lc-sys`, `ring`, `lz4-sys`, `cmake`) [exec] | Bare `bin/server.sh` enables **unauthenticated remote JMX** on 9999/9998 (override `ARCADEDB_JMX`) and writes **heap dumps on OOM** (1.9 GB `.hprof` observed; `ARCADEDB_HEAP_DUMP_DIR=""`) [exec] |

### 7.2 Hashing integration specifics: the u64 boundary and other traps

**SurrealDB, executed with h = 16641971587827677869 (> i64::MAX).** Every route by which a raw u64 reaches SurrealDB either changes its type or loses information:

| Path | Result |
|---|---|
| `serde_json::json!({"h": h})` bound, then returned | `1.6641971587827677e+19`, type **float**, inexact. `impl SurrealValue for serde_json::Value` maps a number to Int only if `as_i64()` succeeds [src] |
| `SerdeWrapper(H { h })` | `"16641971587827677869"`, type **decimal** |
| `Value::from_t(h: u64)` | Int −1804772485881873747 (bit-cast); `into_t::<u64>()` then fails "out of range for type u64" |
| `#[derive(SurrealValue)] struct Node { id, h: u64 }` + `db.create(..).content(..)` | **The row is written** as an int, but **the call returns `Err`** ("Failed to deserialize field 'h' … out of range for type u64"). For h < 2⁶³ it round-trips. That makes the bug data-dependent: about half of all hashes fail |
| SurrealQL literal `RETURN 16641971587827677869` | parse error: "number cannot fit within a 64bit signed integer" |
| `CREATE t:16641971587827677869` | succeeds, but the key is the **string** `"16641971587827677869"` |
| `RecordId::parse_simple("t:1")` | always yields a **string** key `'1'` [src]; `k:10` and ``k:`10` `` are distinct records [exec] |

Rule: declare hash fields as `i64`, bit-cast at the boundary, never `u64` with the derive; use hex strings when ids must match python-xxhash or `xxhsum` byte for byte. Sort order [exec] ([record ids](https://surrealdb.com/docs/surrealql/datamodel/ids)): i64 keys come back in **signed** order; zero-padded hex string keys in **unsigned** order (``hx:`8000000000000000`..=`ffffffffffffffff` `` returns exactly the upper half of the hash space); array keys support prefix range scans (`c:[f:1, NONE]..=[f:1, ..]` plans as `RecordIdScan`); ranges are end-exclusive since 3.0, so use `..=`.

```rust,ignore
// excerpt of /home/claude/graph-hash-harness/surreal/src/main.rs  (cargo run; output in run2.txt)
use xxhash_rust::xxh3::{xxh3_64, xxh3_64_with_seed, xxh3_128};
/// Stable node identity: hash of the fully-qualified name (NOT the body), so the id survives edits.
fn node_key(fq_name: &str) -> i64 { xxh3_64(fq_name.as_bytes()) as i64 } // bit-cast, lossless: (k as u64) == h
/// Content fingerprint stored as a field. Bit-cast to i64 so it is a SurrealQL int.
fn content_hash(body: &str) -> i64 { xxh3_64(body.as_bytes()) as i64 }
/// Deterministic edge key: hash of (caller_key, callee_key) as fixed-width LE bytes.
fn edge_key(from: i64, to: i64) -> i64 {
    let mut buf = [0u8; 16];
    buf[..8].copy_from_slice(&from.to_le_bytes());
    buf[8..].copy_from_slice(&to.to_le_bytes());
    xxh3_64_with_seed(&buf, 0xED6E) as i64 // seed distinguishes the edge domain from the node domain
}
// Idempotent ingest in one transaction: nodes first (ENFORCED relation requires endpoints), keep prev_hash.
//   FOR $n IN $nodes { UPSERT $n.id SET name = $n.name, body = $n.body,
//       prev_hash = IF content_hash = NONE THEN $n.content_hash ELSE content_hash END,
//       content_hash = $n.content_hash RETURN NONE; }
//   INSERT RELATION INTO calls $edges ON DUPLICATE KEY UPDATE seen = (seen OR 0) + 1 RETURN NONE
// Schema: DEFINE FIELD dirty ON func TYPE bool VALUE prev_hash != NONE AND prev_hash != content_hash;
//         DEFINE INDEX func_dirty ON func FIELDS dirty;   -- WHERE dirty = true plans as IndexScan
//         DEFINE TABLE calls TYPE RELATION IN func OUT func ENFORCED SCHEMAFULL;
//         DEFINE INDEX calls_unique ON calls FIELDS in, out UNIQUE;
```

**ArcadeDB, executed:** `u64_to_i64` (two's complement) preserves canonical big-endian bytes (`struct.pack('>q', signed) == xxh3_64_digest()` over 10,000 hashes) but **not** unsigned sort order; offset binary (`u64_to_i64(u ^ 1<<63)`) preserves order but no longer equals the digest bytes. **Into a schemaless property, `2**64-1` was silently stored as DECIMAL**, so declare `CREATE PROPERTY T.h LONG` to make a missed conversion fail loudly. The HTTP envelope caps results at **20,000 rows** (`"limit": 20000, "truncated": bool`; the server appends `limit 20001` to SQL without a LIMIT), so always check `truncated`.

```python
# harness: /home/claude/graph-hash-harness/arcade/harness_http.py  (hashing helpers + identity schema; run against a local 26.10.1 server)
import struct, xxhash

def u64_to_i64(u: int) -> int:
    """Two's-complement reinterpretation so a u64 digest fits ArcadeDB LONG (signed 64-bit)."""
    return u - (1 << 64) if u >= (1 << 63) else u

def i64_to_u64(i: int) -> int:
    return i + (1 << 64) if i < 0 else i

def canon(*fields) -> bytes:
    """Length-prefixed canonical encoding (u32 LE length + UTF-8 bytes) so ('ab','c') != ('a','bc')."""
    out = bytearray()
    for f in fields:
        b = f if isinstance(f, bytes) else str(f).encode()
        out += struct.pack("<I", len(b)) + b
    return bytes(out)

def node_key(kind, qualname):                 # identity: what the node IS (not its content)
    d = canon(b"node-v1", kind, qualname)
    return xxhash.xxh3_128_hexdigest(d), u64_to_i64(xxhash.xxh3_64_intdigest(d))

def content_hash(body: str) -> int:           # what it CONTAINS
    return u64_to_i64(xxhash.xxh3_64_intdigest(body.encode()))

def edge_key(src_key128, label, dst_key128):
    return xxhash.xxh3_128_hexdigest(canon(b"edge-v1", src_key128, label, dst_key128))

DDL = [
    "CREATE VERTEX TYPE CodeNode IF NOT EXISTS",
    "CREATE PROPERTY CodeNode.key128 IF NOT EXISTS STRING (mandatory true, notnull true, readonly true)",
    "CREATE PROPERTY CodeNode.key64 IF NOT EXISTS LONG",
    "CREATE PROPERTY CodeNode.content_hash IF NOT EXISTS LONG",
    "CREATE PROPERTY CodeNode.hash_alg IF NOT EXISTS STRING",          # e.g. 'xxh3-v0.8/seed0/canon-v1'
    "CREATE INDEX IF NOT EXISTS ON CodeNode (key128) UNIQUE_HASH",      # identity, O(1) equality
    "CREATE INDEX IF NOT EXISTS ON CodeNode (key64) UNIQUE",            # optional, ordered LSM
    "CREATE VERTEX TYPE Function IF NOT EXISTS EXTENDS CodeNode",
    "CREATE EDGE TYPE Dep IF NOT EXISTS",
    "CREATE PROPERTY Dep.ekey IF NOT EXISTS STRING",
    "CREATE INDEX IF NOT EXISTS ON Dep (ekey) UNIQUE_HASH",
    "CREATE EDGE TYPE CALLS IF NOT EXISTS EXTENDS Dep",
    "CREATE INDEX IF NOT EXISTS ON CALLS (`@out`, `@in`) UNIQUE",
    "CREATE DOCUMENT TYPE Memo IF NOT EXISTS",
    "CREATE PROPERTY Memo.fp IF NOT EXISTS STRING",
    "CREATE INDEX IF NOT EXISTS ON Memo (fp) UNIQUE_HASH",
]
# Ingest = one sqlscript request: BEGIN; per node
#   UPDATE {kind} SET key128=:k, key64=:s, qualname=:q, content_hash=:c, hash_alg='...' UPSERT WHERE key128=:k;
# per edge
#   CREATE EDGE {label} FROM (SELECT FROM CodeNode WHERE key128=:a) TO (SELECT FROM CodeNode WHERE key128=:b)
#     IF NOT EXISTS SET ekey=:e;
# COMMIT RETRY 3;
```

### 7.3 Executed results (both databases)

| Measurement | SurrealDB 3.3.2 (kv-mem, opt-level 1) | ArcadeDB 26.10.1 (HTTP, `-Xmx1g`) |
|---|---|---|
| Idempotent re-ingest | after 3 ingests: func = 7, calls = 9, `seen` = 2 on each edge | 30 nodes / 39 edges: first ingest 554 ms, identical re-ingest 250 ms, counts identical; v2 with one body changed: 97 ms, only `util.f2`'s `content_hash` changed |
| Scale ingest | 5,000 nodes + 15,000 edges (`UNIQUE(in,out)` + deterministic ids, batches of 1,000): **6.60 s**; full idempotent re-ingest **4.13 s** (63%) | 20,000 nodes + 59,883 edges via OpenCypher `UNWIND … MERGE`: **1.23 s + 4.82 s**; identical re-run **0.50 s + 2.51 s** |
| Change detection | manifest diff over 5,000 ids: **43 ms → exactly the 50 changed**; writing only those: 25 ms | pairwise `UNWIND` diff over 20,000 pairs: **20 changed found in 0.33 s** |
| No-op rewrite | the event fired only when the hash actually changed | identical UPDATE and identical UPSERT: count 1, **0 change events**; real change and revert: 1 event each |
| Closure / reach | 3-hop distinct reach (27 nodes) 0.89 ms; `{..6+collect}` (1,030 nodes) **4.98 ms** | full closure 17,898 vertices: cold 930 ms; warm **292 ms (DFS default)**, **70 ms (`STRATEGY BREADTH_FIRST`)**; depth ≤ 6 (629 vertices): 4.0 ms warm BFS, 51.9 ms cold |
| Other algorithms | — | `algo.bfs` 215 ms; `algo.pagerank({relTypes:'NE'})` 193 ms; `algo.wcc('NE')` 989 ms (it returned 20,080 rows, every vertex in the DB, including 80 unrelated ones); `shortestPath` over 49 hops 284 ms (cold); `SELECT 1` HTTP floor 1.1 ms |
| Failure modes | unbounded `.{..+path}` on a graph with a self-loop: "Exceeded the idiom recursion limit of 256"; `+collect` and `+shortest` were fine | SQL `MATCH … while:(true) RETURN DISTINCT` and Cypher `[:NE*1..]` with `count(DISTINCT)` **OOM'd a 1 GB heap** on the 20k/60k graph (they enumerate paths, not vertices); bounded `while: ($depth < 6)` worked |

**Edge-dedup cost (SurrealDB).** 15,000 edges over 5,000 nodes, separate in-memory DB per case; pass 1 re-ingests identical edges. Cases ea–ec are in `$H/surreal/edgebench.txt`; ed–ee are in `edgebench_lw.txt`, because the first run's `RELATE $e.in->…` was a parse error and was re-run with parenthesised endpoints:

| Strategy | Pass 0 | Pass 1 (re-ingest) | Edges after | `{..6+collect}` |
|---|---|---|---|---|
| ea: xxh3 edge id, no unique index, `INSERT RELATION IGNORE` | 2.65 s | **0.58 s** | 15,000 | 39.7 ms |
| eb: random id + `UNIQUE(in,out)`, `IGNORE` | 6.43 s | 1.48 s | 15,000 | 30.4 ms |
| ec: xxh3 edge id + `UNIQUE(in,out)` | 6.53 s | **0.55 s** | 15,000 | 28.8 ms |
| ed: `LIGHTWEIGHT`, `FOR … RELATE ($e.in)->ed->($e.out)` | **0.88 s** | 0.89 s | 15,000 | 37.8 ms |
| ee: classic, `[in,out]` id via `RELATE OR UPDATE` | 1.82 s | 1.76 s | 15,000 | 32.7 ms |

The unique index is the dominant first-write cost (~2.4×, ea vs ec); deterministic ids make re-ingest cheap (a point-key existence check); LIGHTWEIGHT has the fastest first ingest, but RELATE is evaluated per edge, so its re-ingest costs as much as the first; traversal times are within noise.

**Concurrency (ArcadeDB)** [exec]: 6 threads × 19 `CREATE EDGE … IF NOT EXISTS` on the same 19 pairs gave **19 edges, 0 errors with `UNIQUE(@out,@in)`** and **22 edges (3 duplicates) without it** (single-threaded `IF NOT EXISTS` without the index did dedup). 4 threads × 3 rounds × 50 keys of `UPDATE … UPSERT WHERE k = :k` on `UNIQUE_HASH` gave exactly **50 rows, 0 errors**. The unique index, not `IF NOT EXISTS`, is what makes dedup atomic, as the docs say.

**Type-hierarchy traps (ArcadeDB)** [exec]: a `UNIQUE_HASH` index on the supertype `CodeNode(key128)` rejected a `Function` whose key equalled an existing `Module`'s (correct cross-subtype protection; the docs contradict each other on this, and the executed behaviour is coverage). **But `UPDATE CodeNode … UPSERT WHERE key128 = <new>` on the supertype created a plain `CodeNode`, not a subtype**, so always upsert against the concrete subtype.

**Change feed (ArcadeDB `/ws`).** Change events "only [work] on the server where changes are executed". A subscriber that falls more than 16 MB (`server.eventBusMaxPendingBytes`) behind is dropped ([HTTP API](https://docs.arcadedb.com/arcadedb/reference/http-api/http.html)). The feed is not a durable log.

**SurrealDB CHANGEFEED.** `SHOW CHANGES FOR TABLE artifact SINCE 0` returned 5 entries with increasing versionstamps. With `INCLUDE ORIGINAL`, updates carried reverse patches (`[{op:"replace", path:"/h", value:11}]`) and deletes carried `original` [exec]. It is one feed per database, so versionstamps are contiguous per table only if the table is alone in its database ([SHOW](https://surrealdb.com/docs/surrealql/statements/show)).

### 7.4 Merkle memo keys and event-driven invalidation

Two executed ways to make a memo table correct without hand-written invalidation:
1. **Event-driven.** A synchronous `DEFINE EVENT … WHEN $event = 'UPDATE' AND $before.content_hash != $after.content_hash THEN { DELETE memo WHERE deps CONTAINS $after.id OR of = $after.id }`. A rewrite with the same hash left the memo row in place; changing the leaf's hash deleted the dependent root memo **in the same transaction**. ASYNC events (3.0+) are at-least-once and run every 5 s by default ([DEFINE EVENT](https://surrealdb.com/docs/surrealql/statements/define/event)).
2. **Merkle key.** `key = XXH3-128(own_hash ‖ len ‖ sorted closure content_hashes)`. A changed leaf changes every ancestor's key, so lookups miss. **There is no invalidation code at all**; stale rows are garbage, not wrong answers.

The Merkle key on SurrealDB (the full program, including the event-driven variant and its schema, is `src/bin/merkle.rs`):

```rust,ignore
// excerpt of /home/claude/graph-hash-harness/surreal/src/bin/merkle.rs  (`cargo run --bin merkle`; output in merkle_tail.txt)
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::RecordId;
use xxhash_rust::xxh3::{xxh3_64, xxh3_128};

fn nk(s: &str) -> i64 { xxh3_64(s.as_bytes()) as i64 }
/// Merkle key: XXH3-128 over (own hash, sorted closure hashes) as fixed-width LE bytes.
async fn merkle(db: &Surreal<Db>, f: &str) -> surrealdb::Result<String> {
    let mut r = db.query("RETURN $f.content_hash; RETURN $f.{..+collect}(->calls->func).content_hash;")
        .bind(("f", RecordId::new("func", nk(f)))).await?;
    let own: Option<i64> = r.take(0)?;
    let mut deps: Vec<i64> = r.take(1)?;
    let own = own.expect("own");
    deps.sort_unstable();
    let mut buf = Vec::with_capacity(8 * (deps.len() + 2));
    buf.extend_from_slice(&own.to_le_bytes());
    buf.extend_from_slice(&(deps.len() as u64).to_le_bytes());
    for d in deps { buf.extend_from_slice(&d.to_le_bytes()); }
    Ok(format!("{:032x}", xxh3_128(&buf)))
}
// in main():
//   (b) Merkle key: no invalidation; changed closure => different key => miss
let k1 = merkle(&db, "main").await?;
db.query(format!("CREATE memo:`{k1}` SET of = func:{m}, deps = [], result = 'R2'")).await?.check()?;
let k_same = merkle(&db, "main").await?;
println!("merkle main k1={k1} recompute={k_same} hit={}", k1 == k_same);
db.query(format!("UPDATE func:{l} SET content_hash = 333")).await?.check()?;
let k2 = merkle(&db, "main").await?;
let mut rr = db.query("SELECT VALUE result FROM ONLY type::record('memo', $k)").bind(("k", k2.clone())).await?;
let hit: Option<String> = rr.take(0)?;
println!("after leaf change: k2={k2} differs={} lookup={hit:?}", k1 != k2);
```

Output, re-run at write time: `merkle main k1=996a52475c6004c7a102e0b25a412c72 recompute=996a52475c6004c7a102e0b25a412c72 hit=true`, then `after leaf change: k2=02a15c53c6e9972ad22d47ffe525272f differs=true lookup=None`. The earlier `merkle.txt` ends in a `take` error from a previous version of this source; `merkle_tail.txt` is the output of the current source.

The same pattern on ArcadeDB (closure of `core.f1`, 4 nodes: miss → UPSERT → hit, fp `c44c8c4c85cd1b2960c9d3f8f929e547`) [exec]:

```python
# harness: /home/claude/graph-hash-harness/arcade/harness_http.py  (memo keyed by the closure fingerprint; query/cmd/canon defined there)
def fingerprint(target):
    rows = query("""SELECT key128, content_hash FROM (TRAVERSE out('CALLS') FROM (SELECT FROM Function WHERE qualname = :t))
                    ORDER BY key128""", {"t": target})
    h = xxhash.xxh3_128()
    for r in rows: h.update(canon(r["key128"], str(r["content_hash"])))
    return h.hexdigest(), len(rows)

fp, n = fingerprint("core.f1")
hit = query("SELECT result FROM Memo WHERE fp = :fp", {"fp": fp})
cmd("UPDATE Memo SET fp = :fp, result = 'expensive-analysis-output' UPSERT WHERE fp = :fp", {"fp": fp})
```

Costs and counter-cases: each Merkle key costs one closure traversal per lookup, O(closure); on hot paths store a per-node `closure_hash` instead (own content + sorted children's closure hashes, recomputed bottom-up in topological order), so a changed leaf invalidates exactly its ancestors (§2.5, §5.2) [inf]. ArcadeDB `INCREMENTAL` materialized views currently do a *full* refresh after each committed source transaction ([materialized views](https://docs.arcadedb.com/arcadedb/how-to/data-modeling/materialized-views.html)). Events cannot reach caches outside the database. Memo rows accumulate and need GC.

### 7.5 Gotchas (each executed unless marked)

**SurrealDB**

| ID | Gotcha |
|---|---|
| S-G1 | **u64 is a foot-gun.** See §7.2. Use `i64` fields and bit-cast at the boundary |
| S-G2 | **The crate README is stale.** The README inside 3.3.2 still shows `surrealdb::sql::Thing` and serde derives (2.x API). The real API is `surrealdb::types::{RecordId, RecordIdKey, SurrealValue, Value}` [src]. docs.rs failed to build 2.7.0 and lists 3.3.0-beta.3 as its last successful build ([docs.rs](https://docs.rs/surrealdb/2.7.0/surrealdb/struct.RecordId.html)), so read the crate source |
| S-G3 | **2.x → 3.x is both a rewrite and a migration.** `type::thing` is removed ("did you maybe mean `type::record`"). "the 3.x binary cannot directly read 2.x data", so use `surreal v2 export --v3` + `import`. Ranges are end-exclusive, `LET` is required, `?` became `.?`, `MTREE` → HNSW ([migration](https://surrealdb.com/docs/build/migrating/from-old-surrealdb-versions/2x-to-3x.md)) |
| S-G4 | **Recursion limits.** Unbounded `+path` on a cycle errors at 256 levels; always bound it (`..4+path`) |
| S-G5 | **`FOR` loop endpoints.** `RELATE` endpoints that are field paths must be parenthesised: `RELATE ($e.in)->ed->($e.out)` |
| S-G6 | **Indexes and predicates.** Field-vs-field predicates (`WHERE prev_hash != content_hash`) are `TableScan`. Materialise a `VALUE` bool and index it. `WHERE content_hash = <const>` needs `DEFINE INDEX` to become an `IndexScan` |
| S-G7 | **3.3-specific changes.** Before 3.3, a re-RELATEd legacy edge could leave a stale reverse key, so traversals returned the edge **twice**; upgrade before relying on traversal counts. `mem://` no longer supports `VERSION` queries ([3.3 notes](https://surrealdb.com/releases/3.3)) |
| S-G8 | **LIGHTWEIGHT edges.** They cannot carry data, indexes, events, changefeeds or LIVE (`RELATE … SET w = 1` → "a LIGHTWEIGHT relation's edges cannot be given a data clause") |
| S-G9 | **Live queries are node-local.** On Community edition they only see changes made on the node holding the subscription ([LIVE](https://surrealdb.com/docs/surrealql/statements/live)) [doc] |
| S-G10 | **HNSW memory.** The HNSW graph is held fully in memory, "not bounded by" the cache setting ([DEFINE INDEX](https://surrealdb.com/docs/surrealql/statements/define/indexes)) [doc] |
| S-G11 | **Permissions issue.** Open issue #7466 (3.2.3): a traversal inside a `FOR select` PERMISSIONS clause bypasses the traversed table's permissions ([#7466](https://github.com/surrealdb/surrealdb/issues/7466)) [doc] |
| S-G12 | **Benchmarks are vendor-only.** The vendor crud-bench (Threadripper 9970X, May 2026) reports 3.x mixed CRUD 141k ops/s vs 107k for 2.x and wins over Neo4j, but Postgres leads on reads (327k vs 254k). It contains **no graph-traversal numbers** ([3.x by the numbers](https://surrealdb.com/blog/surrealdb-3-x-by-the-numbers.md)) [vendor] |

**ArcadeDB**

| ID | Gotcha |
|---|---|
| A-G1 | **RIDs are reused** after delete. Never persist them outside the database |
| A-G2 | **BINARY keys** cannot be equality-queried through their index in 26.10.1. The index can only be probed as ``SELECT FROM index:`Bin[k]` WHERE key = :k``. Use 32-hex STRING |
| A-G3 | **Raw u64** is rejected into LONG, and **silently becomes DECIMAL** in schemaless properties |
| A-G4 | **UPSERT on a supertype** creates the supertype |
| A-G5 | **`algo.*` scope.** Procedures span all vertices in the database and yield RIDs. `labels(node)` fails on them, and `id(n) = node` matched 0 rows. Run them in a database holding only the analysed graph, or post-filter |
| A-G6 | **Path enumeration OOMs.** Unbounded variable-length patterns with DISTINCT exhaust the heap; bound the depth, or use `TRAVERSE … BREADTH_FIRST` / `algo.bfs` |
| A-G7 | **TRAVERSE defaults to depth-first**, which was 4× slower than `STRATEGY BREADTH_FIRST` on full closure (292 vs 70 ms warm) |
| A-G8 | **The HTTP 20,000-row cap** applies; check `truncated`. Chunk `UNWIND` diffs (5,000 pairs per request was used) |
| A-G9 | **sqlscript results.** A `sqlscript` returns only the last statement's result (`[{"operation":"commit"}]`), so per-row change counts need separate statements. The query `WHERE key128 IN :ks AND content_hash NOT IN :hs` compares against the *set* of hashes, not per key: an anti-pattern |
| A-G10 | **`/api/v1/batch` is insert-only.** One duplicate key fails the whole request with 409, and the other new vertex was not inserted either |
| A-G11 | **JMX and heap dumps.** The bare `server.sh` enables unauthenticated remote JMX and writes heap dumps on OOM (1.9 GB observed); the server stayed `ready` afterwards |
| A-G12 | **Plugins must be listed.** Postgres and Bolt plugins only start when listed in `arcadedb.server.plugins`; the jars alone are not enough |
| A-G13 | **Docs drift.** The docs track `main`: several "Since 26.11.1" behaviours were unreleased on 2026-10-08. Session-expiry defaults conflict (5 s vs 30 s) [doc] |
| A-G14 | **No implicit `V`.** There is no implicit base `V` type (`FROM V` → "Type with name 'V' was not found"), unlike OrientDB |

### 7.6 Merkle-structured stores, CDC and the columnar alternative

**Content-addressed stores split the hashing work in two:** a fast hash for *shape* decisions, and a cryptographic hash for *addresses*.

| System | Shape hash | Address hash | Source |
|---|---|---|---|
| Dolt prolly trees | **XXH3-64 truncated to 32 bits** (`uint32(xxh3.HashSeed(key, salt))`) over the key only. Per-level salt = first 8 bytes of SHA-512(level). Chunks 512 B–16 KiB, Weibull-CDF boundary probability, target 4,096 | **SHA-512 truncated to 20 bytes** ("sha-512 is faster than sha-256 on 64 bit … 20 bytes is a good balance") | ([node_splitter.go](https://github.com/dolthub/dolt/blob/main/go/store/prolly/tree/node_splitter.go), [hash.go](https://github.com/dolthub/dolt/blob/main/go/store/hash/hash.go)) |
| AT Protocol MST | SHA-256, counting leading zeros in 2-bit chunks (fanout 4) | SHA-256 CIDv1 (dag-cbor) | ([repository spec](https://atproto.com/specs/repository)) |
| Git | — | SHA-1 over `type SP size NUL content`; SHA-256 repos since 2.29. Git 3.0, which makes SHA-256 the default, is unreleased (latest tag v2.56.0) | ([hash-function-transition](https://git-scm.com/docs/hash-function-transition), [BreakingChanges](https://github.com/git/git/blob/master/Documentation/BreakingChanges.adoc)) |
| IPFS/IPLD | — | Multihash: non-crypto functions "are not suitable for content addressing systems". `xxh3-64` (0xb3e3) and `xxh3-128` (0xb3e4) are **draft** multicodecs; `murmur3-x64-64` is permanent | ([multihash](https://github.com/multiformats/multihash), [table.csv](https://github.com/multiformats/multicodec/blob/master/table.csv)) |
| `merkle-search-tree` 0.8.0 (Rust) | — | SipHash-2-4 128-bit (keyed, non-cryptographic) for in-cluster anti-entropy | ([crates.io](https://crates.io/crates/merkle-search-tree)) |

History independence (the same rows produce the same tree regardless of write order) is what lets diff and merge skip identical subtrees: Dolt's diff "scales with the size of the differences, not the size of the tree" ([Dolt docs](https://www.dolthub.com/docs/architecture/storage-engine/prolly-tree/)). Merkle-CRDT sync broadcasts only the root CID and fetches only missing nodes, but history grows without bound and concurrent heads need a tie-breaker ([Merkle-CRDTs](https://arxiv.org/abs/2004.00107)).

**Executed: Merkle diff vs deep diff** (Python, 64×64×64 tree = 262,144 leaves, one leaf changed) [exec `$H/columnar/merkle_diff.out`]: Merkle diff **199 µs, 193 nodes visited**; deep-equality diff **158 ms, 266,305 nodes visited**; one-off Merkle build **1,062 ms**, about 7× one deep diff. A Merkle tree pays only when its digests are maintained incrementally and reused across many comparisons.

**Executed: FastCDC.** fastcdc 5.0.0, 256 MiB random data, min/avg/max = 4,096/16,384/65,536 → 13,407 chunks [exec `$H/columnar/cdc.out`]:

| Step | GiB/s |
|---|---|
| FastCDC v2020 boundary scan only | 1.72 |
| per-chunk xxhash-rust xxh3_128 | 5.16 |
| per-chunk twox-hash `XxHash3_128::oneshot` | 6.42 |
| per-chunk BLAKE3 (1 thread) | 2.76 |
| per-chunk SHA-256 | 1.19 |

After a 100-byte insert FastCDC re-stored **1 of 13,407** chunks and fixed 16 KiB chunking **10,924 of 16,385 (66.67%)**. With XXH3 the scan is the bottleneck: end-to-end ≈ 1.36 GiB/s vs ≈ 0.70 with SHA-256 (derived). `fastcdc` 5.0.0 requires **even** min/avg/max sizes for v2020 (odd sizes silently shifted boundaries before; [CHANGELOG](https://github.com/nlfiedler/fastcdc-rs/blob/master/CHANGELOG.md)), and `Chunk.hash` is the gear-hash state at the boundary, **not** a content fingerprint.

**Columnar edge tables vs native CSR.** Python, |V| = 200,000, |E| = 999,987, 2 threads [exec `$H/columnar/bench_columnar_graph.out`, `duckpgq_and_2hop.out`]:

| Workload | Polars 2.0.0 | DuckDB 1.5.6 | Native (rustworkx 0.18.1 / scipy CSR) |
|---|---|---|---|
| 2-hop path counts | 277.6 ms (lazy self-join + group_by) | 187.7 ms | scipy `A@(A@1)` **8.2 ms**; rustworkx Python loop 249 ms |
| distinct 2-hop pairs | 288.9 ms | 486.6 ms (1.5.4) | scipy `A@A` 115.6 ms |
| reachability, shallow (14 levels) | 205.1 ms (frontier loop) | **141.8 ms** (`WITH RECURSIVE … UNION`); 1,124 ms with `USING KEY` | rustworkx `descendants` 121.4; scipy BFS 16.7 |
| reachability, **5,000-deep chain** | **9,304.6 ms** | **1,031 ms** (UNION); **12,247 ms** (`USING KEY`) | rustworkx **0.4 ms**; scipy **0.1 ms** |
| weakly connected components | 1,410.5 ms (label propagation, 9 rounds) | — | rustworkx 19.0; scipy 41.0 |
| shortest hop counts to 100 targets | — | DuckPGQ `ANY SHORTEST` 3,795.9 ms (1.5.4) | rustworkx Dijkstra 205.8 ms |
| build | — | string→dense-id dictionary encoding via 2 hash joins: 166 ms | scipy CSR 47 ms; rustworkx `PyDiGraph` 374 ms |

Engine facts. **Polars** builds partitioned hashbrown maps keyed by `foldhash::quality` and uses `xxh3_64_with_seed` only for binary/string column hashing ([aliases.rs](https://github.com/pola-rs/polars/blob/main/crates/polars-utils/src/aliases.rs), [vector_hasher.rs](https://github.com/pola-rs/polars/blob/main/crates/polars-core/src/hashing/vector_hasher.rs)); it has no recursive CTEs (`SQLInterfaceError`) [exec]. **DuckDB** uses a murmur-style 64-bit finalizer for fixed-width types and packs a 16-bit salt into hash-table entry pointers ([hash.hpp](https://github.com/duckdb/duckdb/blob/main/src/include/duckdb/common/types/hash.hpp), [ht_entry.hpp](https://github.com/duckdb/duckdb/blob/main/src/include/duckdb/execution/ht_entry.hpp)). The **DuckDB 2.0 preview** claims 42.6× faster reachability (4.051 → 0.095 s on 1 M edges) by keeping `edges` on the build side [vendor] ([DuckDB blog](https://duckdb.org/2026/08/25/how-duckdb-runs-recursive-ctes-faster)); the latest release is still 1.5.6, so re-benchmark when 2.0 ships. **DuckPGQ** has **no binary for DuckDB 1.5.5/1.5.6** (HTTP 404; pin ≤1.5.4) [exec] and "only supports finding `ANY SHORTEST` path … which is non-deterministic" ([DuckPGQ docs](https://duckpgq.org/documentation/sql_pgq/)). **PostgreSQL 19 SQL/PGQ** was committed in March and **reverted on 2026-09-07** ([Command Prompt](https://www.commandprompt.com/blog/two-features-just-left-postgresql-19/)) [secondary].

The crossover [inf from exec]: **edge tables + hash joins win** for single-pass, set-at-a-time work on data already in the engine (2-hop counts, join-heavy reporting, dictionary-encoding string ids, shallow reachability), where building a native structure costs about as much as the whole query. **Sparse linear algebra beats both** for algebraic queries (SpMV 8 ms vs 188 ms). **Edge tables lose badly** on many dependent rounds with small frontiers: ~0.2–2.4 ms of fixed overhead per level in DuckDB and ~1.9 ms per level in Polars, against nanoseconds per edge in CSR. Dependency DAGs are typically deep and narrow, which is exactly the losing regime.

**Counter-case: in-process SQLite vs ArcadeDB over HTTP.** Same 20k/60k graph; SQLite 3.45.1 figures are single runs [exec `$H/arcade/run_rid_sqlite.log`, `run_traverse_variants.log`]:

| Query | SQLite `WITH RECURSIVE … UNION` | ArcadeDB TRAVERSE over HTTP |
|---|---|---|
| load 20k nodes + 60k edges | **0.16 s** | 1.23 + 4.82 s (UNWIND-MERGE), **≈ 38× slower** |
| full closure (17,898) | **128 ms** | 930 ms cold; 292 ms warm DFS; **70 ms warm BFS** |
| depth ≤ 6 (629) | **1.8 ms** | 19.5 ms cold; 4.0 ms warm BFS |
| depth ≤ 12 with (key, depth) | **77.8 ms** | 140.5 ms (single cold run) |

So at this scale, ArcadeDB's advantage is not raw latency. It is the declarative edge model, transactional upkeep and about 70 built-in algorithms. For a small graph that only needs reachability and already lives in an RDBMS, a recursive CTE is simpler and faster. The comparison is SQLite's best case: in-process, no network hop. PostgreSQL was not tested.

---

## 8. The displacement catalogue

The entries are ranked by strength of evidence:
- **Top tier:** deterministic counts from executed harnesses, plus multiple production systems.
- **Bottom tier:** evidence by mechanism, or only partly measured.

Each entry uses the same five headings: **Clunky pattern** (with its concrete symptom), **Replacement**, **Mechanism**, **Evidence**, and **When the old approach is still better**. Every entry has a real counter-case.

### 8.1 Dirty bits, mtimes and TTLs → content-addressed DAG with early cutoff

**Clunky pattern.** Make-style mtime staleness, `touch`-driven rebuilds, hand-written "if X changed, invalidate Y" rules, TTL-expired caches. Symptoms: a formatter run, `git checkout` or Docker layer copy rebuilds everything downstream; a comment edit re-runs the whole cone; reverts and branch switches redo finished work; TTLs serve stale results inside the window and needless misses after it.

**Replacement.** Constructive traces. **Task key** = XXH3-128(task id ‖ code version ‖ canonical args ‖ input *value* fingerprints); **output fingerprint** = XXH3-128(output bytes); the **store** keeps key → (output, fingerprint) for all versions; schedule in topological order and re-key only the frontier.

**Mechanism.** **Early cutoff**: an unchanged output fingerprint leaves dependents' keys unchanged, so they hit. **Constructive restore**: an older version's key is still in the store. **mtime immunity**: a touch changes no bytes, so no key changes.

**Evidence.**
- **Measured** (§4.4) [exec]: comment-only edit **232 → 1** execution (p90 cone 349 → 1); touch **232 → 0**; edit→revert **232 → 0**; lossy interior tasks 349 → 233. Every result matched a from-scratch build, and re-keying 10 k tasks costs ~1 ms.
- **Production systems:** Shake ("previous dependency graph … annotated with file content hashes"); Bazel CAS + action cache; rustc 128-bit result fingerprints with red-green cutoff; Nix CA derivations (experimental); Hamilton keys on dependency *data* versions; Cargo `checksum-freshness` (size + SHA-256/BLAKE3 instead of mtime); Turborepo git blob OIDs ([JFP](https://doi.org/10.1017/S0956796820000088), [REAPI](https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto), [rustc guide](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html), [Nix CA](https://github.com/NixOS/nix/blob/master/doc/manual/source/store/derivation/outputs/content-address.md), [cache_key.py](https://github.com/apache/hamilton/blob/main/hamilton/caching/cache_key.py), [cargo fingerprint](https://github.com/rust-lang/cargo/blob/master/src/compiler/fingerprint/mod.rs), [hash_object.rs](https://github.com/vercel/turborepo/blob/main/crates/turborepo-scm/src/hash_object.rs)).
- **TTL alternative:** content-keyed caches plus a namespace `salt` for poisoning recovery (REAPI), instead of expiry.

**Reference code:** `ex_a_ca_executor` and `build_hybrid` (§4.4).

**When the old approach is still better.** **Restat is enough without branching.** If you need only comment-style cutoff (no revert, no branch switching, no shared cache), **dirty+restat matched content addressing with zero hashing** (232 → 1) [exec]. **Tasks are as cheap as hashing.** With zero-work tasks, dirty-bit costs 0.3 ms per edit vs CA full 1.3 ms [exec]. **Cones are near-total.** On the deep, narrow DAG every sequential strategy cost ≈ naive (209–219 ms) [exec]. **`stat` beats reading bytes on huge, rarely touched trees.** Cargo keeps mtime as its default, and Bazel reads precomputed xattr digests. A hybrid uses mtime + size as a pre-filter and hashes on change [inf]. **External state is unhashable** (LLM model version, remote APIs, wall clock). LangGraph still ships `ttl` alongside XXH3-128 keys ([types.py](https://github.com/langchain-ai/langgraph/blob/main/libs/langgraph/langgraph/types.py)). **Undeclared inputs break every incremental strategy alike** (32 stale outputs, §9). **Inside one process, equality wins.** With the previous value in memory, Salsa/DICE-style value equality plus revision counters gives early cutoff without hashing (§4.3).

### 8.2 Re-deriving LLM labels and embeddings for code graphs → identity hash + Merkle content hash

**Clunky pattern.** Re-run LLM labelling, embedding and summarisation per changed file (mtime), per line range, or over the whole repo. Symptoms: a banner comment shifts every span, so line-keyed caches miss for every function below it; file-mtime re-indexing redoes the whole file yet misses callers in *other* files, so callee summaries go stale; full rebuilds cost thousands of LLM calls.

**Replacement.** **Identity** = XXH3-128 of the qualified path (`module::<SelfType as Trait>::name`), stable across edits. **Content** = XXH3-128 of the `syn` token stream (comments and whitespace are not tokens). Derived artifacts are keyed by content (label, embedding) or by an **SCC-aware deep Merkle hash** over the call graph (callee summaries).

**Mechanism.** Token-level hashing absorbs formatting, comments and span shifts. The deep hash changes iff the function or anything it transitively calls changes. Mutual recursion is handled by SCC condensation; unique (id, content) member labels make sorting an exact canonical form.

**Evidence.** Measured on petgraph 0.8.3: 64 files, 27,183 lines, 1,245 functions, 3,821 approximate call edges, 3 artifacts per function (3,735 total). Cells are **artifacts recomputed / left stale relative to deep semantics** [exec `$H/engines/results/code_reindex.md`]:

| Edit | Target (transitive callers) | Full rebuild | File-mtime | Shallow content hash | **Deep Merkle** | Interface-Merkle | Deep, doc-split keys |
|---|---|---:|---:|---:|---:|---:|---:|
| blank lines + `//` block at file top (shifts every span) | `node_weight` (118) | 3735 / 0 | 87 / 0 | 0 / 0 | **0 / 0** | 0 / 0 | 0 |
| `//` comment in body | `is_null` (443) | 3735 / 0 | 414 / 0 | 0 / 0 | **0 / 0** | 0 / 0 | 0 |
| `///` doc line added | `node_weight` (118) | 3735 / 0 | 87 / 106 | 3 / 118 | **121 / 0** | 3 / 118 | **1** |
| body change | `edge_endpoints` (6) | 3735 / 0 | 393 / 6 | 3 / 6 | **9 / 0** | 3 / 6 | 9 |
| body change | `node_weight` (118) | 3735 / 0 | 87 / 106 | 3 / 118 | **121 / 0** | 3 / 118 | 121 |
| body change | `is_null` (443) | 3735 / 0 | 414 / 344 | 3 / 443 | **446 / 0** | 3 / 443 | 446 |
| signature change (+param) | `node_weight` (118) | 3735 / 0 | 87 / 106 | 3 / 118 | **121 / 0** | 4 / 117 | 121 |

The index is cheap relative to what it protects: full index (parse + hash + link + Merkle) 156.6 ms; re-parse + hash of the largest file (83 KiB) 16.35 ms; link + Merkle 2.99 ms; Merkle alone 0.451 ms. LLM-derived artifacts dominate cost, so **artifact counts are the metric that matters**. No external production system was examined for this pattern; the evidence is the harness plus the BSàlC/rustc mechanism.

```rust
// harness: /home/claude/graph-hash-harness/engines (bin ex_d_fn_index)
//! Excerpt (d): function-level code graph with identity hash + comment-insensitive content hash (syn 3),
//! deep Merkle hash over the (cyclic) call graph.
use petgraph::{algo::tarjan_scc, graph::{DiGraph, NodeIndex}};
use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use rustc_hash::FxHashMap;
use syn::visit::Visit;
use xxhash_rust::xxh3::{xxh3_128, Xxh3Default};

/// Comments and whitespace are not tokens, so hashing the token stream ignores them for free.
/// (`///` docs ARE tokens — `#[doc = ".."]` — strip them if docs must not invalidate.)
fn hash_ts(h: &mut Xxh3Default, ts: TokenStream) {
    for tt in ts {
        match tt {
            TokenTree::Group(g) => { h.update(&[b'(', g.delimiter() as u8]); hash_ts(h, g.stream()); h.update(b")"); }
            other => { let s = other.to_string(); h.update(&(s.len() as u32).to_le_bytes()); h.update(s.as_bytes()); }
        }
    }
}

pub struct Fn { pub ident: String, pub id: u128, pub content: u128, pub calls: Vec<String> }

struct Calls(Vec<String>);
impl<'a> Visit<'a> for Calls {
    fn visit_expr_call(&mut self, c: &'a syn::ExprCall) {
        if let syn::Expr::Path(p) = &*c.func { self.0.push(p.path.segments.last().unwrap().ident.to_string()); }
        syn::visit::visit_expr_call(self, c);
    }
    fn visit_expr_method_call(&mut self, m: &'a syn::ExprMethodCall) { self.0.push(m.method.to_string()); syn::visit::visit_expr_method_call(self, m); }
}

pub fn index(module: &str, src: &str) -> Vec<Fn> {
    let file = syn::parse_file(src).expect("parse");
    file.items.iter().filter_map(|it| if let syn::Item::Fn(f) = it { Some(f) } else { None }).map(|f| {
        let ident = format!("{module}::{}", f.sig.ident);              // identity: stable across edits
        let mut h = Xxh3Default::new(); hash_ts(&mut h, f.to_token_stream());
        let mut c = Calls(vec![]); c.visit_block(&f.block);
        Fn { id: xxh3_128(ident.as_bytes()), ident, content: h.digest128(), calls: c.0 }
    }).collect()
}

/// deep(v) changes iff v or anything it (transitively) calls changes. Mutual recursion handled by SCCs;
/// member labels (id, content) are unique, so sorting them is an exact canonical form.
pub fn deep_hashes(fns: &[Fn]) -> Vec<u128> {
    let by_name: FxHashMap<&str, usize> = fns.iter().enumerate().map(|(i, f)| (f.ident.rsplit("::").next().unwrap(), i)).collect();
    let mut g = DiGraph::<(), ()>::new(); for _ in fns { g.add_node(()); }
    for (u, f) in fns.iter().enumerate() { for c in &f.calls { if let Some(&v) = by_name.get(c.as_str()) { g.update_edge(NodeIndex::new(u), NodeIndex::new(v), ()); } } }
    let sccs = tarjan_scc(&g); // callees first
    let mut comp = vec![0; fns.len()]; for (i, s) in sccs.iter().enumerate() { for v in s { comp[v.index()] = i; } }
    let (mut sh, mut deep) = (vec![0u128; sccs.len()], vec![0u128; fns.len()]);
    for (i, s) in sccs.iter().enumerate() {
        let mut mem: Vec<(u128, u128)> = s.iter().map(|v| (fns[v.index()].id, fns[v.index()].content)).collect(); mem.sort_unstable();
        let mut kids: Vec<u128> = s.iter().flat_map(|&v| g.neighbors(v)).filter(|w| comp[w.index()] != i).map(|w| sh[comp[w.index()]]).collect();
        kids.sort_unstable(); kids.dedup();
        let mut h = Xxh3Default::new();
        for (a, b) in mem { h.update(&a.to_le_bytes()); h.update(&b.to_le_bytes()); }
        for k in kids { h.update(&k.to_le_bytes()); }
        sh[i] = h.digest128();
        for v in s { deep[v.index()] = xxh3_128(&[sh[i].to_le_bytes(), fns[v.index()].id.to_le_bytes()].concat()); }
    }
    deep
}

fn main() {
    let v1 = "fn leaf(x: u32) -> u32 { x + 1 }\nfn mid(x: u32) -> u32 { leaf(x) * 2 }\nfn top() -> u32 { mid(3) }\nfn other() -> u32 { 7 }\n\
              fn ping(n: u32) -> u32 { if n == 0 { 0 } else { pong(n - 1) } }\nfn pong(n: u32) -> u32 { ping(n) }";
    let v2 = v1.replace("{ x + 1 }", "{\n    // a comment\n    x   +   1 }");
    let v3 = v1.replace("{ x + 1 }", "{ x + 2 }");
    let (a, b, c) = (index("m", v1), index("m", &v2), index("m", &v3));
    let (da, db, dc) = (deep_hashes(&a), deep_hashes(&b), deep_hashes(&c));
    fn changed(x: &[Fn], y: &[Fn]) -> Vec<String> { x.iter().zip(y).filter(|(p, q)| p.content != q.content).map(|(p, _)| p.ident.clone()).collect() }
    let dchanged = |x: &[u128], y: &[u128]| a.iter().zip(x.iter().zip(y)).filter(|(_, (p, q))| p != q).map(|(f, _)| f.ident.as_str()).collect::<Vec<_>>();
    assert!(changed(&a, &b).is_empty() && da == db);              // comment/whitespace edit: nothing invalidated
    assert_eq!(changed(&a, &c), ["m::leaf"]);                     // shallow: only leaf's own artifacts
    assert_eq!(dchanged(&da, &dc), ["m::leaf", "m::mid", "m::top"]); // deep: leaf + transitive callers, not other/ping/pong
    println!("ex_d_fn_index: all assertions passed");
}
```

**When the old approach is still better.** **The call graph is approximate.** Of 5,544 call sites, 1,332 (24%) were dropped as ambiguous (more than 3 same-name candidates) and 1,638 had no in-crate candidate. Missed edges mean possible staleness, so production use should take edges from rust-analyzer/SCIP or rustc name resolution [exec/inf]. **High-fan-in utilities cascade.** One edit to `is_null` invalidates 446 artifacts (443 caller summaries plus its own 3). An **interface firewall** key (own content + direct callees' signatures, like C headers or Java ijar) is cheap but leaves transitive summaries stale (443 here). The choice is a contract decision [exec]. **`///` docs are tokens and do invalidate.** Doc-split keys (docs feed only the label) reduce a doc edit to 1 artifact [exec]. **Tiny repos.** The index costs ~157 ms, so rebuilding everything can be simpler [inf]. **Unstable identity.** Ordinal disambiguation of cfg-gated duplicates breaks when a duplicate is inserted earlier in the file, and macro-generated functions are not expanded [exec gap].

### 8.3 DB-generated or app-allocated ids → content-derived keys

**Clunky pattern.** Sequences or auto ids, a `SELECT id WHERE name = ?` before every insert, or RIDs and record ids stored as external identity. Symptoms: an extra round trip per write; duplicate rows under concurrent producers; ids that differ between environments, so edges cannot be computed offline; ArcadeDB RIDs silently pointing at *different* records after a delete.

**Replacement.** **Node id** = XXH3 of canonical (kind, qualified name), never of content. SurrealDB: `RecordId::new(tb, xxh3_64(name) as i64)`, or `[hi, lo]` / 32-hex for 128-bit. ArcadeDB: `key128` 32-hex STRING with `UNIQUE_HASH` on the supertype. Upsert via `UPSERT`, `INSERT … ON DUPLICATE KEY UPDATE`, `UPDATE … UPSERT WHERE key128 = :k` or Cypher `MERGE`. Collision guard: a READONLY natural key or a unique index.

**Mechanism.** Every producer computes the same id with no coordination, so writes become idempotent, and the point lookup on the key needs no secondary index (SurrealDB `RecordIdScan`).

**Evidence** [exec]. SurrealDB: counts unchanged after 3 ingests; lossless `u64 → i64` round trip; the READONLY guard turned a colliding `UPSERT` into "Found changed value for field `name` … but field is readonly". ArcadeDB: concurrent UPSERT from 4 threads gave exactly 50 rows with 0 errors; re-ingest left counts unchanged at 30/39 and 20,000/59,883; the supertype unique index blocked cross-subtype key collisions; RID `#61:2` was reassigned after a delete.

```python
# harness: /home/claude/graph-hash-harness/arcade/harness_extra.py  (concurrent UPSERT on a UNIQUE_HASH key: 50 rows, 0 errors)
cmd("CREATE VERTEX TYPE Blob IF NOT EXISTS"); cmd("CREATE PROPERTY Blob.k IF NOT EXISTS STRING")
cmd("CREATE INDEX IF NOT EXISTS ON Blob (k) UNIQUE_HASH")
def worker(t, typ):
    s = requests.Session(); s.auth = AUTH
    for rep in range(3):
        for i in range(50):
            kk = xxhash.xxh3_128_hexdigest(f"blob{i}".encode())
            body = {"language": "sql", "command": f"UPDATE {typ} SET k = :k, w = :w UPSERT WHERE k = :k", "params": {"k": kk, "w": t}}
            r = s.post(f"{BASE}/command/{DB}", json=body)
            if r.status_code != 200: errs[typ].append(r.json().get("error"))
ths = [threading.Thread(target=worker, args=(t, "Blob")) for t in range(4)]
[x.start() for x in ths]; [x.join() for x in ths]
```

**When the old approach is still better.** **No stable natural name.** Renames then produce a new identity, plus delete + insert and edge churn. **Ids must be dense or ordered.** SurrealDB `ulid()` gives time order; dense ids are needed for array indexing (§3). **The 64-bit budget is thin.** Above ~10⁷–10⁸ entities, use 128-bit ids. **A single writer with a DB sequence** has no dedup problem to solve. **Content-as-identity is wrong.** Hashing the *body* into the id turns every edit into delete + insert and churns every incident edge; keep identity and content hashes separate [inf]. **ArcadeDB supertype UPSERT** creates the supertype [exec].

### 8.4 Timestamps, polling and version counters → content-hash change detection

**Clunky pattern.** `updated_at` columns polled with `WHERE updated_at > :last`, plus app-managed version counters. Symptoms: no-op rewrites look like changes and trigger downstream work; clock skew and same-millisecond writes are missed; every writer must remember to bump the counter (ArcadeDB has no per-record `@version` in SQL, so the counter would be entirely app-managed).

**Replacement.** Store `content_hash` (64-bit is fine for a per-record comparison) and detect change by (1) a **pre-write manifest diff** (send `(id, hash)` pairs, get back only changed ids), (2) a **guarded write** (`UPDATE … WHERE content_hash <> :h` returns 0 or 1), (3) an **indexed `VALUE`-computed `dirty` flag** (SurrealDB), or (4) a **change feed**: SurrealDB `CHANGEFEED` + `SHOW CHANGES SINCE <versionstamp>` (durable cursor) or `LIVE SELECT`; ArcadeDB `/ws`.

**Mechanism.** Equality of canonical-byte hashes replaces logical time, so no-op rewrites stop being events.

**Evidence** [exec]. SurrealDB: a manifest diff over 5 k ids found exactly the 50 changed in 43 ms; `dirty = true` plans as an `IndexScan`; the changefeed returned ordered versionstamps with reverse diffs; a live notification arrived on the embedded engine within 3 s. ArcadeDB: 20 changed found among 20 k pairs in 0.33 s; an identical UPDATE or UPSERT emitted **0 events**, a real change 1. Production precedent: Git's object model (name = hash of content) and Dolt's history-independent trees ([git](https://git-scm.com/docs/hash-function-transition), [Dolt](https://www.dolthub.com/blog/2024-11-26-history-independence/)).

```python
# harness: /home/claude/graph-hash-harness/arcade/harness_extra.py  (pairwise diff + guarded write; query/cmd from harness_http.py)
pairs = [{"k": node_key(kd, q)[0], "h": content_hash(b)} for kd, q, b in nodes2]
rows = query("UNWIND $pairs AS p MATCH (n:CodeNode {key128: p.k}) WHERE n.content_hash <> p.h RETURN n.qualname AS q ORDER BY q",
             {"pairs": pairs}, lang="opencypher")
# guarded write: count 0 if unchanged, 1 if changed (a plain UPSERT returns count 1 even when identical)
r_guard = cmd("UPDATE Function SET content_hash = :h WHERE key128 = :k AND content_hash <> :h", {"h": h, "k": k})
```

**When the old approach is still better.** **Audit, retention, time travel, replay after downtime.** These need wall-clock timestamps. SurrealDB `mem://` has no `VERSION` reads since 3.3 [doc]. **Causal ordering.** Hashes cannot say which event happened first. atproto keeps a monotonic `rev` next to the MST root ([atproto](https://atproto.com/specs/repository)), and Merkle clocks "cannot say which of the events happened first" for concurrent heads ([Merkle-CRDTs](https://ar5iv.labs.arxiv.org/html/2004.00107)). **Cross-node consumers.** SurrealDB Community live queries and ArcadeDB `/ws` events are node-local, and ArcadeDB drops subscribers that fall more than 16 MB behind [doc]. **Diffs need the full key set,** chunked under ArcadeDB's 20 k row cap. **Internal revision counters are not displaced.** Salsa's `Revision` and DICE's version numbers are the right tool in memory.

### 8.5 App-side edge dedup → deterministic edge ids + unique indexes

**Clunky pattern.** `SELECT … WHERE in = ? AND out = ?` then INSERT, in application code. Symptoms: a race window that produces duplicate edges under concurrent writers; two round trips per edge; traversals that return edges twice.

**Replacement.** SurrealDB: `LIGHTWEIGHT` relations for pure adjacency (deps, calls); otherwise `[in, out]` array ids, or a seeded xxh3 of a canonical `(in, out, discriminator)` tuple, with `INSERT RELATION … ON DUPLICATE KEY UPDATE`/`IGNORE`, optionally plus `UNIQUE(in, out)` as defence in depth. ArcadeDB: `UNIQUE(@out,@in)` + `CREATE EDGE … IF NOT EXISTS`; when parallel edges are real, a hashed `ekey` under `UNIQUE_HASH` on the edge supertype.

**Mechanism.** The edge identity is computable from its endpoints, and the unique index makes the existence check atomic.

**Evidence** [exec]. ArcadeDB with 6 concurrent writers: **22 edges (3 duplicates) without the index, 19 with it**. SurrealDB re-ingest of 15 k edges: 0.55 s with deterministic ids vs 1.48 s with random ids + a unique index (§7.3). LIGHTWEIGHT: three identical RELATEs returned the same `dep:[func:…, func:…]` and `count(dep)` = 1. ArcadeDB's docs say the unique index is what makes dedup atomic ([CREATE EDGE](https://docs.arcadedb.com/arcadedb/reference/sql/sql-create-edge.html)).

```python
# excerpt of /home/claude/graph-hash-harness/arcade/harness_extra.py  (edge dedup by content-derived ekey; node_key/edge_key/cmd/query from harness_http.py)
a, b = node_key("Module", "app")[0], node_key("Module", "net")[0]
ek = edge_key(a, "IMPORTS", b)        # xxh3_128_hexdigest(canon(b"edge-v1", src128, label, dst128)); UNIQUE_HASH on Dep(ekey)
for i in range(2):
    r = cmd("CREATE EDGE IMPORTS FROM (SELECT FROM CodeNode WHERE key128 = :a) TO (SELECT FROM CodeNode WHERE key128 = :b) IF NOT EXISTS SET ekey = :e",
            {"a": a, "b": b, "e": ek}, check=False)
# -> IMPORTS count rises by exactly 1; the edge is found again by ekey via the Dep[ekey] index
```

**When the old approach is still better.** **Parallel edges are legitimate** (two call sites with different lines). `UNIQUE(@out,@in)` and `[in, out]` forbid them; use a discriminator hash, at the cost of an extra index. **First loads of known-new data.** Plain batch inserts are fastest. ArcadeDB `/api/v1/batch` is insert-only and rejects the whole request on one duplicate. **First-ingest cost.** A unique index costs ~2.4× on first ingest in SurrealDB. Skip it if all writers use your deterministic ids. **LIGHTWEIGHT limits.** It cannot carry edge data, indexes, events or changefeeds.

### 8.6 String- or sparse-u64-keyed maps → interned dense u32 + CSR

**Clunky pattern.** `HashMap<String, Vec<String>>` adjacency, string-keyed visited sets, or graphs kept in `HashMap<u64, Vec<u64>>` keyed by DB record ids or fingerprints. Symptoms: every edge traversal hashes a key; memory is 3–14× larger; traversal is cache-hostile.

**Replacement.** Intern once (string-interner, lasso + Fx, `IndexMap<&str, u32, FxBuildHasher>`) or relabel once (`HashMap<u64, u32, Fx>`), then build CSR and traverse with `Vec<bool>`/`FixedBitSet`. Keep the fingerprint ↔ dense-id map at the persistence boundary.

**Mechanism.** One hash per distinct key instead of one per edge visit; contiguous neighbour slices.

**Evidence** [exec]. Sparse u64 ids: build **457 vs 989–1,816 ms**, BFS **104 vs 539–1,101 ms**, memory **70 vs 153 MiB** (§3.1). Dense `Vec` vs the best hash map on hits: 4.4× faster at 1 M keys, 4.6× at 16 K. pubgrub resolution with `u32` packages: 1.2–1.7× faster than `String`. pubgrub `HashArena`, Cargo `InternedString`, libsolv pool `Id`s, Salsa `#[salsa::interned]` and the Nx `u32` interner all intern.

Reference code: the Fx relabel + counting-sort CSR excerpt in §3.1. The `IndexMap<&str, u32, FxBuildHasher>` interner → `Vec<Vec<u32>>` + `FixedBitSet` DFS variant runs in `$H/algos` (bin `keys_and_order`, output `[0, 1, 2, 4]`) [exec].

**When the old approach is still better.** **Tiny or one-shot graphs,** and debug-first tooling where readability wins. **Deletion-heavy, open-ended node sets.** Use `StableGraph`, or `GraphMap` with a fast hasher. **Ids that must survive across processes.** Dense ids are not stable, so persist the fingerprint or the name. **Pure-Python pipelines.** For small keys, python-xxhash costs ~110 ns per call vs 79 ns for builtin `hash`. Move the loop to Rust instead of changing hashers [exec].

### 8.7 Deep equality and bespoke dedup → hash-consing

**Clunky pattern.** Recursive `==` on ASTs, plans or config trees; "canonical string" dedup; per-tree fingerprint walks into a `HashSet`. Symptoms: O(size) equality; duplicated subtrees in memory; rule-order-dependent simplifiers.

**Replacement.** A hash-consing interner (`FxHashMap<Node, Id>` whose keys hold child ids, commutative operations canonicalised), so equality is `Id == Id` and dedup is set-insertion of `u32`s. An e-graph (egg/egglog) when equalities are *discovered* during rewriting. For persistence, one XXH3-128 structural hash per *interned node*, not per tree.

**Mechanism.** A shallow, fixed-size key makes "already present?" an O(1) lookup. Creation order is topological, so bottom-up analyses become array passes.

**Evidence** [exec]. 19.85 M → 4,272 nodes; 476 MB → 0.38 MB; 19.8 M → 25 allocations; equality 27 µs → 1.18 ns; dedup 587 → 0.13 ms; build 1,448 → 173 ms; a depth-20 tree of 2,097,151 nodes → 21. egg paper: congruence 88×, equality saturation 21× ([arXiv 2004.03082](https://arxiv.org/abs/2004.03082)). Reference code: `ex_e_hashcons` (§3.3).

**When the old approach is still better.** **The producer already holds shared handles.** An `Rc` memo builds in 0.77 ms vs 173. It cannot find structurally equal terms built separately (10,652 vs 4,272 nodes), and its derived `PartialEq` is still a deep walk [exec]. **Little sharing.** Interning costs ~8.7 ns per constructed node [exec]. **Long-running processes.** The table only grows unless you add weak tables (hashconsing 1.8.0) or epoch rebuilds. **Wrong map hasher.** `Xxh3DefaultBuilder` made interning 4.4× slower than Fx [exec]. **Field-level diffs.** Human-readable diffs *within* changed nodes still need structural comparison.

### 8.8 Hand-pinned dependency lists and hand-ordered steps → solver + fingerprinted lock + topological order

**Clunky pattern.** `versions.yaml` pin tables for internal modules; hand-ordered startup, build or run lists; hand-written "why is X here" docs; imperative step-ordered scripts. Symptoms: pins drift; conflicts surface at runtime; order breaks when a dependency is added; no explanation when something cannot load.

**Replacement.** Per-node `depends_on` edges with constraints; a computed resolution (MVS, pubgrub, or resolvelib in Python) with locked versions as preferences; Kahn order with SCC condensation for cycles; a lock with SHA-256 integrity and XXH3-128 Merkle fingerprints; manifest-digest memo keys.

**Mechanism.** The solver finds the assignment and records why; the order is derived from the graph; fingerprints localise invalidation to ancestors.

**Evidence** [exec §5.2]. The resolver picked `features 1.0.0` automatically; the unsatisfiable request produced a derivation-tree explanation; a content change invalidated exactly 4 ancestors and left 2 siblings untouched; the manifest key is invariant to declaration order. Cargo, uv, Go and Bazel all derive order from the resolved graph and lock by hash ([cargo resolver](https://github.com/rust-lang/cargo/blob/master/src/resolver/mod.rs), [uv](https://docs.astral.sh/uv/reference/internals/resolver/), [go.dev/ref/mod](https://go.dev/ref/mod#minimal-version-selection), [bazel lockfile](https://bazel.build/external/lockfile)). Reference code: `build_lock` / `diff_lock` (§5.2).

**When the old approach is still better.** **Single-version monorepo** (one version rule): there is nothing to resolve. **Compatibility is guaranteed:** MVS is linear and needs no lockfile. **Fewer than ~10 stable steps,** or ordering that encodes side effects not modelled as edges [inf]. **Policy reasons** (legal, organisational) must still be written down, though `External::Custom(M)` can carry them. **Static declaration has its own wins.** It lets a topological scheduler detect cycles and missing keys before work starts, and remote execution needs the input Merkle tree *before* running. Bazel user rules cannot be monadic ([JFP §4.1, Table 1](https://doi.org/10.1017/S0956796820000088)). **Code structure is not always a DAG.** Hamilton's signature-derived DAG redirects loops and conditionals to Burr ([README](https://github.com/apache/hamilton/blob/main/README.md)). **Fabricate-style traced scripts** are the one model BSàlC cannot express.

### 8.9 Hand-written fixed-point loops → Datalog (semi-naive evaluation)

**Clunky pattern.** "Loop until nothing changes" over graph relations, with hand-maintained `HashMap<K, Vec<V>>` side tables and "changed" flags. Symptoms: every round re-joins everything; index choice is ad hoc; termination is fragile when several relations are mutually recursive.

**Replacement.** datafrog (sorted joins, no hashing) for small, hot analyses; ascent or crepe (hash indexes) for rich rule sets; differential-dataflow or DBSP for incremental maintenance over change streams.

**Mechanism.** Each round joins only the *delta* of new tuples; the engine owns the indexes.

**Evidence** [exec §6.7]. Same 44,918 tuples; the naive loop did 2.7×10⁹ probes; 2.4 s vs ~10 ms. **Strawman caveat:** the naive loop is a nested-loop full re-join, so the ratio overstates the gap against a careful worklist. Polonius uses datafrog ([polonius-engine](https://raw.githubusercontent.com/rust-lang/polonius/master/polonius-engine/Cargo.toml)); differential dataflow processed 0.003% of the full work on a sliding-window SCC ([CIDR 2013](https://www.cidrdb.org/cidr2013/Papers/CIDR13_Paper111.pdf)). Reference code: `datafrog_closure` (§6.7).

**When the old approach is still better.** Single-relation reachability: BFS + `FixedBitSet` is simpler and faster. Non-monotone logic. Macro-heavy compile times. **Incremental dataflow limits:** large deltas relative to the database, or tight memory (O(|DB|) arrangements per join or distinct) ([VLDB 2023](https://www.vldb.org/pvldb/vol16/p1601-budiu.pdf)). **A single confluent, terminating rewrite system** needs no e-graph.

### 8.10 Recursive SQL for deep traversal → native CSR (with counter-cases)

**Clunky pattern.** Relational modelling of graph-shaped data (join tables) with `WITH RECURSIVE` for closure, impact or dependency walks, or DataFrame join loops for label propagation and PageRank. Symptoms: seconds per query on deep, narrow graphs; every level pays a fixed engine overhead.

**Replacement.** Dictionary-encode string ids through a hash join (166 ms per 1 M edges), build CSR (scipy 47 ms; petgraph or rustworkx-core), and run native algorithms. Use the graph database's native traversal for neighbourhood and closure queries over live data (SurrealDB `.{..+collect}`, ArcadeDB `TRAVERSE … BREADTH_FIRST`).

**Mechanism.** Pointer-chasing costs nanoseconds per edge, against ~0.2–2.4 ms of fixed overhead per recursion level in DuckDB 1.5.6 and ~1.9 ms per level in Polars.

**Evidence** [exec §7.6]. 5,000-deep chain: **0.1–0.4 ms native vs 1.0–12.2 s DuckDB vs 9.3 s Polars**. WCC: 19–41 ms vs 1.41 s. SSSP: 0.21 s rustworkx vs 3.8 s DuckPGQ. SurrealDB `{..6+collect}` over 1,030 nodes in ~5 ms; ArcadeDB BFS closure 70 ms warm.

```python
# excerpt of /home/claude/graph-hash-harness/columnar/bench_columnar_graph.py  (the clunky loops vs the native call; con = duckdb, e/el = polars edge table, A = scipy CSR)
def duck_union():          # relational recursion: fixed per-level engine overhead
    return con.execute("""
        WITH RECURSIVE r(node) AS (SELECT 0::BIGINT UNION SELECT ee.dst FROM r JOIN ee ON ee.src = r.node)
        SELECT count(*) FROM r""").fetchone()[0]
def polars_frontier():     # one hash join + one anti-join per BFS level
    seen = pl.DataFrame({"node": [0]}, schema={"node": pl.Int64}); frontier = seen; levels = 0
    while frontier.height:
        nxt = (frontier.lazy().join(el, left_on="node", right_on="src").select(pl.col("dst").alias("node")).unique()
               .join(seen.lazy(), on="node", how="anti").collect())
        seen = pl.concat([seen, nxt]); frontier = nxt; levels += 1
    return seen.height, levels
def scipy_bfs():           # native CSR traversal
    return len(breadth_first_order(A, 0, directed=True, return_predecessors=False))
```

**When the old approach is still better.** **Shallow, set-at-a-time queries over data already in SQL.**
  - 14-level reachability: DuckDB 142 ms vs rustworkx 121 ms *before* rustworkx's 374 ms build.
  - 2-hop counts: 188 ms DuckDB vs a 374 ms build first.
- **In-process SQLite vs ArcadeDB over HTTP:** SQLite **loaded 38× faster** and **beat ArcadeDB's default (DFS) and cold closures** (128 ms vs 292/930 ms). Only warm BFS TRAVERSE (70 ms) beat it [exec].
- **Moving data out of SQL is the dominant cost,** or the data is larger than memory.
- **DuckDB 2.0's recursive-CTE rework** (vendor claim 42.6×) may narrow the gap.
- **Rich predicates** across many columns, feeding more SQL.
- **Sparse linear algebra** beats both edge tables and graph libraries for algebraic queries (SpMV 8 ms).

### 8.11 Structural diffs of large stores → Merkle/prolly trees + CDC

**Clunky pattern.** Deep-equality diffs of whole configs, lock trees or datasets; full rescans or full-state transfer for replica sync; fixed-size chunking or whole-file hashing for dedup. Symptoms: diff cost proportional to the data, not the change; a 100-byte insert invalidates two-thirds of the chunks.

**Replacement.** Per-node Merkle digests (XXH3-128 over `canon/v1`) with a top-down skip-equal-subtree diff; prolly or MST trees for KV stores; FastCDC v2020 boundaries + XXH3-128 chunk ids (trusted) or BLAKE3/SHA-256 (untrusted); root-digest exchange for sync.

**Mechanism.** Equal digests prune whole subtrees; content-defined boundaries resynchronise after insertions; history independence makes equal content produce equal trees.

**Evidence** [exec §7.6]. Merkle diff: 193 vs 266,305 nodes visited (199 µs vs 158 ms). After a 100-byte insert, CDC re-stored 1 vs 10,924 chunks. Production precedent: Dolt (diff "scales with the size of the differences"), Git trees, atproto MSTs, and Merkle-CRDT sync, which broadcasts only the root CID.

```python
# excerpt of /home/claude/graph-hash-harness/columnar/merkle_diff.py  (canon_hash from canon.py; run: python3 merkle_diff.py <dir containing canon.py>)
def merkle(tree):
    """Return (digest, children-map); leaves hashed via canon/v1, dirs hash sorted (name, child digest)."""
    if "body" in tree:
        return (canon_hash(tree), None)
    kids = {k: merkle(v) for k, v in tree.items()}
    h = xxhash.xxh3_128()
    h.update(b"dir/v1\x00" + struct.pack("<Q", len(kids)))
    for k in sorted(kids):
        kb = k.encode(); h.update(struct.pack("<Q", len(kb)) + kb + bytes.fromhex(kids[k][0]))
    return (h.hexdigest(), kids)

def merkle_diff(a, b, path=(), out=None, visited=None):
    out = [] if out is None else out
    visited[0] += 1
    if a[0] == b[0]: return out                      # equal digest: prune the whole subtree
    if a[1] is None or b[1] is None: out.append(path); return out
    for k in a[1].keys() | b[1].keys():
        if k not in a[1] or k not in b[1]: out.append(path + (k,)); continue
        merkle_diff(a[1][k], b[1][k], path + (k,), out, visited)
    return out
```

**When the old approach is still better.** **One-off comparisons.** The Merkle build cost 1,062 ms, about 7× one deep diff. **Data mutated in place with no hook to update digests.** **Semantic equality** (float tolerance, unordered lists). **CDC limits.** In-place fixed-width updates (pages, columnar blocks) never shift boundaries. When CPU is the bottleneck, the CDC scan (1.72 GiB/s) is 3–4× slower than the per-chunk hash. **Small datasets:** just send them. **Untrusted peers** require cryptographic digests (atproto mandates SHA-256).

---

## 9. Failure modes

| Failure | Mechanism | Measured or cited | Mitigation |
|---|---|---|---|
| **Collisions from width** | Birthday bound n²/2^(b+1) | XXH3-64: 2.7% at 10⁹ items, 27,105 pairs at 10¹²; 128-bit: 1.47×10⁻¹⁵ at 10¹² [exec] | 128-bit for keys, addresses and shared caches; 64-bit only for per-record change checks behind a natural-key guard |
| **Truncation** | Fx/foldhash outputs are structured on integer inputs | Fx: 0 collisions where 128 were expected; Fx misses 38.9 ns on `i << 40` [exec] | Truncate only XXH3 output; never truncate map-hasher output |
| **Adversarial inputs** | XXH3/XXH64 are not collision-resistant; the random-oracle model covers accidents only | REAPI labels MURMUR3 "not cryptographic"; multihash says non-crypto hashes "are not suitable for content addressing systems"; Turborepo's HMAC protects artifacts, not keys | SHA-256/BLAKE3 wherever another party can write keys or ids are public; std SipHash for attacker-chosen `HashMap` keys. foldhash is only "minimally DoS-resistant", and HashDoS resistance was not tested |
| **Trust boundaries in locks** | Integrity of downloaded bytes | Cargo.lock, uv.lock and go.sum all use SHA-256 ([encode.rs](https://github.com/rust-lang/cargo/blob/master/src/resolver/encode.rs), [uv.lock](https://github.com/astral-sh/uv/blob/main/uv.lock), [go.sum](https://go.dev/ref/mod#go-sum-files)) | Keep SHA-256 for integrity; add XXH3-128 only for internal change detection. SHA-256 costs little here (1.26 GiB/s with SHA-NI) |
| **Canonicalization drift** | The encoding differs between languages, libraries or versions | JCS vs `json.dumps` ordering; cbor2 integer-key order; serde `HashMap` order; NFC 15.1 vs 17.0 (20 code points); DuckDB `hashfuncs` half-swap; networkx 3.5 WL change; pickle keys [exec] | `canon/v1`-style explicit encoder; persist a scheme tag; one shared Rust core via PyO3; pin the Unicode policy |
| **Encoding/conversion drift at the DB** | Signed columns; JSON number types | serde_json → float, SerdeWrapper → decimal, derive u64 → error on ~50% of hashes (SurrealDB); schemaless → DECIMAL (ArcadeDB) [exec] | `i64` fields bit-cast at the boundary; declared LONG properties; or hex strings |
| **Nondeterministic tasks** | Output fingerprints differ every run | Not simulated. **Expected:** early cutoff never fires and the store grows without bound [inf] | Canonicalize outputs before fingerprinting (sort, strip timestamps), or exclude those tasks from caching (`AlwaysUnequal`/`no_hash` analogues) |
| **Undeclared inputs** | Ambient state that is not part of the key | **32 stale outputs under dirty-bit, restat and CA alike; only naive re-execution was correct** [exec]. uv needs `tool.uv.cache-keys` for setuptools-scm ([uv cache](https://docs.astral.sh/uv/concepts/cache/)) | Model every ambient read (env vars, config, git tags, model versions, clock) as an input node whose value hash enters the key. rustc's `eval_always` concedes the same thing |
| **Cache growth / GC** | Constructive traces keep every version | 10,001 entries after a cold build + one edit [exec]; DBSP arrangements O(\|DB\|) | LRU or epoch eviction; REAPI `salt` to disown poisoned namespaces; TTL as GC only, never as correctness |
| **Cycles** | Merkle recursion does not terminate; Kahn's leftover set over-reports | Kahn leftovers included the downstream `root` [exec] | `tarjan_scc` condensation (§6.1); report SCC members |
| **Symmetric structure** | Relabel-invariant hashes cannot separate WL-equivalent members | C7(1,2) vs C7(1,3) collide even with WL refinement [exec] | Put identity into content; exact canonical labelling; bucket + confirm |
| **Opaque ids hurt debugging** | Hashes in logs, keys and URLs carry no meaning | — | Store the natural key next to the hash (READONLY name / `qualname`); log `name@hash8`; keep "why" explanations from the graph (§5) |
| **Platform-dependent hashers** | Fx `K` differs between 32-bit and 64-bit; foldhash/ahash/std are seeded per process | pubgrub #373; petgraph `dijkstra` order changes per run [exec] | Never persist J1 hash values; `IndexMap`/`BTreeMap` or sorting for anything emitted |
| **twox 32-bit streaming bug** | twox-hash XXH3 streaming is wrong past 4 GiB on 32-bit targets (crate reference G7) | the earlier crate reference [exec] | Use xxhash-rust, or one-shot calls, on 32-bit |
| **Native-build XXH64 regression** | twox-hash XXH64 at 4.84 GiB/s under `target-cpu=native` on this CPU | quiet re-run [exec] | Benchmark on the target CPU; prefer xxhash-rust or XXH3 |
| **DB traversal blow-ups** | Path enumeration | ArcadeDB OOM at 1 GB; SurrealDB unbounded `+path` errors at 256 [exec] | Bound depth; use vertex-set traversals (`+collect`, BFS) |

---

## 10. Applying this to the user's codebase

The codebase was not shared. Below are an audit procedure and a staged migration plan that an agent can run against the Rust (SurrealDB) and Python (ArcadeDB) repositories.

### 10.1 Audit checklist

**Inventory every graph.**
1. **Data graphs.** Every SurrealDB `TYPE RELATION` table and record link; every ArcadeDB vertex/edge type; every in-memory adjacency structure.
   - Search Rust for `HashMap<.*Vec<`, `petgraph::`, `GraphMap`, `DiGraph`.
   - Search Python for `defaultdict(list)`, `networkx`, `rustworkx`.
2. **Execution graphs.** Task runners, pipeline definitions (YAML DAGs, Dagster/Prefect/Hamilton/LangGraph definitions), job queues with prerequisites, build scripts.
3. **Dependency graphs.** Plugin registries, `versions.yaml`/pin tables, startup orders, data-product lineage, schema migration chains.

**For each graph, record the following.**

| Question | What to look for | Maps to |
|---|---|---|
| Node identity scheme? | DB auto ids, RIDs, `rand()`/ULIDs, sequences, `SELECT id WHERE name=?` before insert; ids derived from content (wrong) vs name (right) | §8.3 |
| Edge identity / dedup? | SELECT-then-INSERT; missing `UNIQUE(in,out)`/`(@out,@in)`; random edge ids | §8.5 |
| Hashers in use? | `Xxh3DefaultBuilder`/`BuildHasherDefault<XxHash3_64>`/`xxhash64::State` as map hashers; `nohash` on raw ids; lasso `Rodeo::new()`; `GraphMap` default `S`; persisted Fx/foldhash/ahash/`hash()` values; Python `hash()` persisted | §2, §3.4 |
| Fingerprint encodings? | `#[derive(Hash)]` into a persisted hash; `json.dumps(sort_keys=True)`; pickle; serde over `HashMap`; `str` passed to python-xxhash; missing scheme tags | §1.2 |
| Widths? | XXH3-64/XXH64 used as memo, cache or dedup keys at >10⁷ entries or shared across machines | §1.1 |
| u64 at the DB boundary? | `u64` fields with `#[derive(SurrealValue)]`; `serde_json` u64 binds; raw u64 SurrealQL literals; ArcadeDB schemaless hash properties | §7.2 |
| Hot algorithms? | BFS/DFS/toposort/SCC/closure/PageRank in tight loops; visited sets as `HashSet`; `petgraph::Graph` traversal; recursive SQL or DataFrame join loops for deep walks; networkx loops on large graphs | §3, §8.6, §8.10 |
| Hand-declared invalidation? | mtimes, `updated_at` polling, TTLs as correctness, manual "invalidate X when Y" code, hand-bumped `code_version`/version counters | §8.1, §8.4 |
| Hand-declared ordering or dependencies? | Ordered step lists, pinned versions, hand-maintained dependency lists | §8.8 |
| Hand-written fixed points? | `while changed:` loops over relations | §8.9 |
| Deep equality or bespoke dedup? | Recursive `==` on trees; "canonical string" keys; per-tree fingerprint walks | §8.7 |
| Derived-artifact caches (LLM)? | Keys by file path + mtime or line range; full rebuilds | §8.2 |
| Ambient inputs? | env vars, config files, model versions, git state, clock, network reads inside cached tasks | §9 |
| Output determinism? | `HashMap` iteration reaching output, goldens, locks or cache keys | §3.4 |
| Trust boundaries? | Caches written by other teams, tenants or CI forks; downloaded artifacts; public ids | §0.1 J3 |

### 10.2 Staged migration order

| Stage | Change | Why this order | Exit check |
|---|---|---|---|
| 1 | **Identity and DB schema.** Content-derived node keys from canonical (kind, qualified name). SurrealDB: `i64` bit-cast ids + READONLY name guard; 128-bit `[hi,lo]` or 32-hex for memos. ArcadeDB: `key128` STRING + `UNIQUE_HASH` on the supertype, upserts on concrete subtypes. Deterministic edge ids / `LIGHTWEIGHT` / `UNIQUE(@out,@in)`; `X-Request-Id` on ArcadeDB writes; `hash_alg` tag column | Fixes correctness under concurrency first (duplicate edges, RID reuse); everything later depends on stable ids | Re-ingest twice → counts unchanged; concurrent ingest test → no duplicates |
| 2 | **Shared canonical encoder + fingerprint library.** One Rust crate (`canon/v1` + XXH3-128 + Merkle helpers), exposed to Python via PyO3/maturin abi3; parity tests against python-xxhash; fixed NFC policy | Removes Rust↔Python drift before fingerprints are persisted | 27/27-style vector parity in CI on both sides |
| 3 | **Change detection without timestamps.** `content_hash` columns, manifest diffs or guarded updates, changefeed/`/ws` consumers for in-node push. Keep `updated_at` only for audit | Cheap, high value; removes polling and no-op churn | Identical re-ingest → 0 changed rows, 0 events |
| 4 | **In-memory representation.** Relabel DB ids to dense `u32`, build CSR, use `FixedBitSet`/epoch visited sets; Fx/foldhash for remaining maps; `IndexMap`/sort for emitted output; fix lasso and `GraphMap` hashers | Largest measured speed-ups (3–7×) for algorithm-heavy code; independent of the DB stages | Golden outputs stable across runs; BFS/closure timings |
| 5 | **Content-addressed execution.** XXH3-128 task keys over value fingerprints, frontier-only re-keying, a store with GC, all ambient inputs as explicit nodes; restat-style cutoff where revert/branch/shared caching is not needed | Needs stages 1–2 for stable keys; the undeclared-input audit is the precondition | Touch → 0 executions; revert → 0; ambient change → correct re-run |
| 6 | **Merkle memo keys and derived artifacts.** Per-node closure hashes (SCC-aware), memo tables keyed by them, deep Merkle keys for LLM artifacts with doc-split keys and an interface firewall where fan-in is high | Builds on 1–5 | Comment edit → 0 artifacts; body edit → exactly the callers' cone |
| 7 | **Resolution and locks.** Per-node manifests, MVS or pubgrub/resolvelib, a lock with SHA-256 + XXH3-128 fps stored as a graph in the DB, a manifest-digest memo, an SCC cycle policy | Replaces pin tables and ordered lists once identity and fingerprints are stable | Reordered manifests → same key; content change → ancestor-only diff |
| 8 | **Algorithm engines where they replace hand-written code.** Datalog for multi-relation fixed points; DB `algo.*` (in a dedicated database) or Rust petgraph/rustworkx-core for whole-graph analytics; WL + exact confirmation for graph dedup; sketches only at scale | Highest effort, narrowest wins; adopt per use case | Same results as the existing code on golden data |

Keep the old approach where §8's counter-cases apply:
- in-process value equality (Salsa-style);
- timestamps for audit;
- recursive SQL for shallow queries over data already in SQL;
- MVS or nothing for single-version monorepos;
- cryptographic hashes at trust boundaries.

---

## Appendix A — Verification: what was executed, where, and how to re-run it

**Machine.** One KVM guest: 2 vCPU, Intel Xeon @ 2.10 GHz (family 6, model 207), L2 4 MiB total, L3 260 MiB, ~8 GiB RAM, Linux 6.18. CPU flags include AVX2, AVX-512F/BW/VL, AES-NI, SHA-NI and VAES.

**Toolchain.** rustc/cargo 1.97.0 (2d8144b78 2026-07-07), `--release` (opt-level 3, no LTO), baseline `x86-64` target; native builds used `RUSTFLAGS="-C target-cpu=native"` with `CARGO_TARGET_DIR=target-native`. Python 3.13.16; OpenJDK 21.0.12.1.

**CPU-label discrepancy.** The columnar notes' header names a "2.80 GHz" CPU, copied from the earlier crate reference. `lscpu` on the harness host reports **2.10 GHz**, and every results file written by the Rust harnesses records the 2.10 GHz CPU.

**Noise.** Other agents were compiling on the same 2 vCPUs during many runs; each results file records its load average (0.06–3). Figures are medians of 7 interleaved repetitions with rotated variant order, except BFS builds (5), key patterns (5), collisions (5 seeds), depres (min of 5 per run, 4 runs), columnar (best of 3; the DuckDB and Polars rows on the 5,000-deep chain are single runs) and SQLite (single runs). A full BFS replicate moved individual numbers by 1–11% for visited sets and up to ~14% for builds and library traversals. **Treat differences under ~30% as parity**; deterministic counts are the primary evidence. Under contention the rayon speed-up in `dag_exec` vanished (217.5 vs 210.2 ms, against 1.8× when quiet), so re-run every parallel number on a quiet machine with more cores.

### A.1 Quiet-machine re-runs

`$H/primitives/results/c_fingerprint.default.quiet.md` (load 0.06) and `c_fingerprint.native.quiet.md` (load 0.82, compile-time features `[sse4.2,avx2,avx512f,aes]`) reproduce the noisy-run conclusions. They also **confirm the twox-hash XXH64 regression under `target-cpu=native` on this host**:

| Build | twox XXH64 at 1 MiB | xxhash-rust XXH64 at 1 MiB | twox XXH64 at 16 KiB |
|---|---|---|---|
| default | 10.1 GiB/s | 10.1 GiB/s | 1,463 ns |
| native | **4.84 GiB/s** | 10.3 GiB/s | **3,121 ns** |

The earlier crate reference reported XXH64 at ~11–12 GiB/s "under every build" on a different host, which was given as a 2.80 GHz Xeon. The regression is therefore CPU-dependent. That reference has since added a correction note to the same effect. Other quiet-run figures used in this report:

| Measurement | Value |
|---|---|
| XXH3-64 at 1 MiB, xxhash-rust | 17.9 GiB/s (default) → 42.3 GiB/s (native) |
| XXH3-64 at 1 MiB, twox-hash | 27.2 → 29.2 GiB/s |
| hand LE encoder → XXH3-64 | 25.9 ns/node |
| serde_json sorted keys → XXH3-64 | 858 ns/node |

### A.2 Executed checks by harness

All commands run from the harness directory. `$H` = `/home/claude/graph-hash-harness`.

| Harness | Command(s) | Output | Key verified results |
|---|---|---|---|
| `$H/primitives` (8 bins + lib; `Cargo.lock`) | `cargo run --release --bin {a_hashmap_keys,b_bfs,c_fingerprint,d_interning,e_collisions,f_db_integers,g_merkle,h_key_patterns}`; `scripts/run_all.sh`; `FLAVOR=native scripts/run_all.sh`; `cargo run --release --bin f_db_integers -- --json > vec.json && python3 scripts/check_python.py vec.json` | `results/*.{default,native,replicate,quiet}.md`, `python_check.md`, `vectors.json` | §2 tables; collision obs/exp 0.98–1.03; 1 M u64↔i64 round trips; xxhash-rust == twox-hash == python-xxhash on 6 vectors |
| `$H/engines` | `./run_all.sh`; `cargo run --release --bin dag_exec -- --reps 7` (plus `--coarse 0.3`, `--work-us 0`, `--out-bytes 65536`, `--layers 1000 --window 2`); `scc_merkle -- --reps 7 --perms 200`; `wl_hash -- --reps 7` then `python3 -I py/wl_compare.py results/wl_small.edges 20000 3 5`; `code_reindex -- --reps 7`; `hash_cons -- --reps 7` (and `--depth 6`); `ex_a_ca_executor`, `ex_b_scc_merkle`, `ex_c_wl`, `ex_d_fn_index`, `ex_e_hashcons` | `results/*.md`, `results/excerpts.txt`, `wl_networkx_collisions.txt` | §4.4, §6.1, §6.2, §8.2, §3.3. All 5 excerpts print "all assertions passed" (**re-run at report-writing time: pass**) |
| `$H/algos` | `cargo run --release --bin {keys_and_order,petgraph_determinism,wl_hash,merkle_scc,acyclic_incremental,hashcons_egg,datalog_reach}`; `python py/wl_probe.py`, `py/wl_probe2.py` (networkx 3.7 venv) | `outputs/*.txt` | §3.4, §6.1, §6.2, §6.6–§6.8. **Re-run at report-writing time:** `acyclic_incremental`, `datalog_reach` (44,918 tuples; wall 1.39 s vs 6.3 ms this time), `merkle_scc`, `wl_hash`, `keys_and_order`: same results |
| `$H/depres` | `cargo run --release` (pubgrub =0.4.0, xxhash-rust =0.8.19, sha2 0.10.9, rustc-hash 2.1.3) | stdout (4 runs in the notes) | §5.2. **Re-run at report-writing time: §1–§8 identical** |
| `$H/surreal` | `cargo run` (dev, opt-level 1; `surrealdb =3.3.2`, `kv-mem`); `cargo run --bin {extra,edgebench,u64derive,guard,merkle}`; `cargo run --bin edgebench -- lw` | `run2.txt`, `extra.txt`, `edgebench.txt`, `edgebench_lw.txt`, `u64derive.txt`, `guard.txt`, `merkle_tail.txt` | §7. `merkle.txt` ends in a `take` error from an earlier source version, while `merkle_tail.txt` is the current source. **The current `merkle` binary was re-run at report-writing time: hit=true, then lookup=None after the leaf change.** The ea–ec rows come from `edgebench.txt`; the ed/ee rows from `edgebench_lw.txt`, because `edgebench.txt` shows a parse error for unparenthesised `RELATE $e.in->…` |
| `$H/arcade` | Server: `cd srv/arcadedb-26.10.1 && ARCADEDB_OPTS_MEMORY="-Xms256m -Xmx1g" ARCADEDB_JMX=" " bin/server.sh` with root password, localhost binds and `arcadedb.server.plugins=Postgres:…,Bolt:…`. Then `python3 harness_http.py`, `harness_extra.py`, `harness_protocols.py`, `harness_driver.py`, `harness_ws.py` | `run_*.log` | §7. The server was **not** re-run at report-writing time. Results come from the logs (`run_http_1.log`, `run_extra_{1,2,3}.log`, `run_ws_2.log`, `run_rid_sqlite.log`, `run_traverse_variants.log`, `run_subtype_unique.log`) |
| `$H/columnar` | `python3 bench_columnar_graph.py`, `duckpgq_and_2hop.py`, `build_costs.py`, `using_key_variants.py`, `merkle_diff.py <dir>`, `probe_jcs.py`, `probe_cbor.py`, `nfc_compare.py`, `pyo3_parity.py`, `py_hash_perf.py`, `py_bulk.py`, `stable_col_hash.py`; Rust: `cdc-rs`, `canon-rs <vectors.json>`, `hm`, `nfcdiff`; `canon-py` via `python3 -m maturin build --release` | `*.out` next to each `*.py`, `cdc.out` | §1.2–§1.3, §7.6. **Re-checked at report-writing time:** python-xxhash 4.0.1 rejects `str`; `canon_hash("café") == canon_hash("café")`; XXH3-128(`node:42`) = `1ea4757a…c5a6` |

### A.3 Conflicts between notes and how they were resolved

| Conflict | Resolution |
|---|---|
| twox XXH64 under native flags: ~11–12 GiB/s in the earlier reference vs ~4.8 GiB/s here | Quiet re-run confirms ~4.84 GiB/s on this CPU. Reported as CPU-dependent |
| SurrealDB Merkle demo: error (`merkle.txt`) vs success (`merkle_tail.txt`) | The current source was re-run and the success is confirmed |
| SurrealDB LIGHTWEIGHT/`RELATE OR UPDATE` benchmark: parse error vs timings | `edgebench_lw.txt` (parenthesised endpoints, as the docs require) is authoritative |
| ArcadeDB `shortestPath` default direction: docs say both "OUT" and "BOTH" | Executed default is BOTH |
| ArcadeDB supertype unique index coverage: two doc statements disagree | Executed: the supertype index covers subtypes |
| ArcadeDB UPSERT error text differs from docs | Behaviour as executed ("Upsert must involve an index …") |
| ArcadeDB HTTP session expiry: 5 s vs 30 s in the docs | Unresolved [doc]. Set it explicitly |
| networkx version: engines/columnar used 3.6.1; algos probes used 3.7 | WL source on main matches 3.6.1. Collisions were reproduced on both |
| sha2 version: 0.11.0 (primitives, cdc) vs 0.10.9 (depres) | Both used SHA-NI and gave the same ~1.2–1.3 GiB/s |
| bincode 2 `encode_to_vec` in the quiet run: encode + hash (145 ns) < encode (154 ns) | Noise in an allocation-dominated row. Not used for any rule |
| datafrog wall times varied (naive 1.39–2.40 s) | Counts identical. Wall times labelled indicative |

### A.4 Not executed (claims kept as [doc]/[src]/[unverified])

Sketch crates (probminhash, xorf, fastbloom, HyperBall); timed nauty-pet/VF2 (exact SCC canonicalization was brute force for k ≤ 8); Salsa, DICE and comemo; nondeterministic tasks, remote/shared caches and structural (edge-changing) incremental Merkle updates; SurrealDB RocksDB/SurrealKV, release builds, HNSW and multi-node live queries; ArcadeDB HA clusters, `arcadedb-embedded`, gRPC and `/ws` `conflictMode:update`; PostgreSQL recursive CTEs; DuckDB 2.0 (the 42.6× figure is a vendor claim); aarch64, multi-threaded hash-table workloads, Zipf-skewed lookups, power-law graphs and HashDoS resistance.

---

## Appendix B — Versions (as of 2026-10-08)

| Component | Version used / current | Notes |
|---|---|---|
| xxhash-rust | 0.8.19 | `xxh3`, `xxh64`; BSL-1.0 |
| twox-hash | 2.1.5 | `default-features = false`, `xxhash3_64`, `xxhash3_128`, `xxhash64`, `std` |
| python-xxhash | 4.0.1 (2026-08-17), libxxhash 0.8.3 | rejects `str`; `data=` keyword |
| rustc-hash (Fx) | 2.1.3 | `K` differs on 32-bit |
| foldhash | 0.2.0 | hashbrown 0.17 default |
| hashbrown | 0.17.1 | top-7-bit tag |
| ahash / gxhash / rapidhash | 0.8.12 / 3.5.0 / 4.5.1 | gxhash needs AES at compile time |
| nohash-hasher | 0.2.0 | 2020 |
| indexmap | 2.14.2 | |
| petgraph | 0.8.3 (master pre-release restructuring, still 0.8.3) | `GraphMap` default `S = RandomState` |
| fixedbitset / roaring | 0.5.7 / 0.11.5 | |
| lasso / string-interner / internment / hashconsing | 0.7.3 / 0.20.0 / 0.8.6 / 1.8.0 | |
| blake3 / sha2 / siphasher | 1.8.7 / 0.11.0 (0.10.9 in depres) / 1.0.4 | |
| bincode | **2.0.1** used; **3.0.0 is a `compile_error!` tombstone** | |
| postcard / serde / serde_json | 1.1.3 / 1.0.229 / 1.0.151 | |
| syn / proc-macro2 / quote | 3.0.6 / 1.0.107 (`span-locations`) / 1.0.47 | `ItemImpl::trait_` shape changed vs 2.0.119 |
| rayon | 1.12.0 | |
| pubgrub / version-ranges | 0.4.0 (2026-04-10) / 0.1.3 | MSRV 1.92 |
| astral-pubgrub | 0.6.1 (2026-08-07) | uv fork |
| resolvo / libsolv / resolvelib | 0.12.2 / 0.7.40 / 1.2.1 (pip 26.2.1 vendors it) | |
| salsa / comemo | 0.28.5 / 0.5.1 | |
| differential-dataflow / timely / dbsp | 0.25.1 / 0.31.0 / 0.363.0 | |
| rustc-stable-hash | 0.1.2 | |
| egg / egglog | 0.11.0 / 3.0.0 | |
| datafrog / ascent / crepe | 2.0.1 / 0.8.1 / 0.2.0 | DDlog repo archived 2026-07-13 |
| rustworkx / rustworkx-core | 0.18.1 | |
| nauty-pet / nauty-Traces-sys | 0.15.0 / 0.11.0 | |
| graph / graph_builder / pathfinding / webgraph-algo | 0.3.2 / 0.4.2 / 4.16.0 / 0.6.2 | |
| incremental-topo | 0.3.1 | |
| probminhash / xorf / fastbloom / growable-bloom-filter | 0.1.12 / 0.13.0 / 0.17.0 / 2.1.1 | |
| fastcdc | 5.0.0 (2026-08-22) | even sizes required for v2020 |
| pyo3 / maturin | 0.29.3 / 1.15.0 | abi3-py310 |
| unicode-normalization | 0.1.25 (Unicode 17.0) | Python 3.13 = Unicode 15.1 |
| serde_jcs / rfc8785 (py) / cbor2 (py) | 0.2.0 / 0.1.4 / 6.1.5 | |
| **SurrealDB** crate / server | **3.3.2** (2026-10-08), `kv-mem`; 2.7.0 still patched | BSL 1.1 |
| **ArcadeDB** | **26.10.1** (tag 2026-10-05) | Apache-2.0; Java 21/25 |
| arcadedb-driver / psycopg / neo4j (py) | 0.2.0 / 3.3.6 / 6.4.0 | |
| SQLite | 3.45.1 | in-process baseline |
| Polars (py) / polars crate / polars-hash | 2.0.0 / 0.55.2 / 0.9.3 | |
| DuckDB / hashfuncs / DuckPGQ | 1.5.6 (1.5.4 for DuckPGQ) / 2025120401 (`0dec806`) / 0.3.1 (`f386a6c`) | no DuckPGQ for 1.5.5/1.5.6 |
| networkx | 3.6.1 used; 3.7 latest (2026-09-21) | WL changed in 3.5 |
| scipy / numpy | 1.18.1 / 2.5.3 | |
| Python orchestration | dagster 1.13.25, apache-hamilton 1.90.0 (main unreleased → XXH3-128), prefect 3.8.8, langgraph 1.2.14, apache-airflow 3.3.2 | |
| Build / CI tools (read, not run) | Bazel tag 9.3.0, Nix 2.35.2, Turborepo 2.11.7, Nx 23.3.0, uv 0.12.24, cargo 0.102.0, Git v2.56.0 (3.0 unreleased) | |

---

## Appendix C — Sources

All external sources cited above, grouped by topic. Local evidence lives under `/home/claude/graph-hash-harness/` (Appendix A).

**Theory and papers**
- Build Systems à la Carte: [JFP 2020](https://doi.org/10.1017/S0956796820000088), [ICFP 2018 PDF](https://www.microsoft.com/en-us/research/wp-content/uploads/2018/03/build-systems.pdf); [Ninja manual](https://ninja-build.org/manual.html)
- Differential dataflow: [CIDR 2013](https://www.cidrdb.org/cidr2013/Papers/CIDR13_Paper111.pdf); DBSP: [VLDB 2023](https://www.vldb.org/pvldb/vol16/p1601-budiu.pdf)
- WL graph kernels: [JMLR 2011](https://www.jmlr.org/papers/v12/shervashidze11a.html); GNN expressiveness survey: [Sato](https://arxiv.org/pdf/2003.04078); nauty/Traces: [arXiv 1301.1493](https://arxiv.org/abs/1301.1493)
- egg: [arXiv 2004.03082](https://arxiv.org/abs/2004.03082); HyperANF: [arXiv 1011.5599](https://arxiv.org/abs/1011.5599); SetSketch: [arXiv 2101.00314](https://arxiv.org/abs/2101.00314)
- Merkle-CRDTs: [arXiv 2004.00107](https://arxiv.org/abs/2004.00107), [ar5iv](https://ar5iv.labs.arxiv.org/html/2004.00107)
- Pregel: [paper](https://kowshik.github.io/JPregel/pregel_paper.pdf); Pearce–Kelly: [JEA PDF](http://www.doc.ic.ac.uk/~phjk/Publications/DynamicTopoSortAlg-JEA-07.pdf)
- MVS: [research.swtch.com/vgo-mvs](https://research.swtch.com/vgo-mvs); PubGrub spec: [solver.md](https://github.com/dart-lang/pub/blob/master/doc/solver.md)
- [GraphBLAS](https://graphblas.org/)

**Build, orchestration and incremental systems**
- Bazel: [remote_execution.proto](https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto), [BlazeServerStartupOptions.java](https://github.com/bazelbuild/bazel/blob/master/src/main/java/com/google/devtools/build/lib/runtime/BlazeServerStartupOptions.java), [bzlmod lockfile](https://bazel.build/external/lockfile)
- Buck2 DICE: [key.rs](https://github.com/facebook/buck2/blob/main/dice/dice/src/api/key.rs), [incrementality PDF](https://github.com/facebook/buck2/blob/main/dice/dice/docs/DiceIncrementalityAlgorithms.pdf), [cas_digest.rs](https://github.com/facebook/buck2/blob/main/app/buck2_common/src/cas_digest.rs)
- Nix: [store-path.md](https://github.com/NixOS/nix/blob/master/doc/manual/source/protocols/store-path.md), [content-address.md](https://github.com/NixOS/nix/blob/master/doc/manual/source/store/derivation/outputs/content-address.md), [experimental-features.cc](https://github.com/NixOS/nix/blob/master/src/libutil/experimental-features.cc)
- rustc: [dev guide](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html), [fingerprint.rs](https://github.com/rust-lang/rust/blob/main/compiler/rustc_data_structures/src/fingerprint.rs)
- Cargo: [fingerprint/mod.rs](https://github.com/rust-lang/cargo/blob/master/src/compiler/fingerprint/mod.rs), [dep_info.rs](https://github.com/rust-lang/cargo/blob/master/src/compiler/fingerprint/dep_info.rs), [hasher.rs](https://github.com/rust-lang/cargo/blob/master/src/util/hasher.rs)
- Salsa: [backdate.rs](https://github.com/salsa-rs/salsa/blob/master/src/function/backdate.rs), [interned.rs](https://github.com/salsa-rs/salsa/blob/master/src/interned.rs), [durability.rs](https://github.com/salsa-rs/salsa/blob/master/src/durability.rs)
- comemo: [hash.rs](https://github.com/typst/comemo/blob/main/src/hash.rs), [tree.rs](https://github.com/typst/comemo/blob/main/src/tree.rs)
- Turborepo: [traits.rs](https://github.com/vercel/turborepo/blob/main/crates/turborepo-hash/src/traits.rs), [lib.rs](https://github.com/vercel/turborepo/blob/main/crates/turborepo-hash/src/lib.rs), [hash_object.rs](https://github.com/vercel/turborepo/blob/main/crates/turborepo-scm/src/hash_object.rs), [signature_authentication.rs](https://github.com/vercel/turborepo/blob/main/crates/turborepo-cache/src/signature_authentication.rs)
- Nx: [hasher.rs](https://github.com/nrwl/nx/blob/master/packages/nx/src/native/hasher.rs), [task_hasher.rs](https://github.com/nrwl/nx/blob/master/packages/nx/src/native/tasks/task_hasher.rs)
- DBSP/Feldera: [hash.rs](https://github.com/feldera/feldera/blob/main/crates/dbsp/src/hash.rs), [filter.rs](https://github.com/feldera/feldera/blob/main/crates/dbsp/src/storage/file/filter.rs)
- Dagster: [data_version.py](https://github.com/dagster-io/dagster/blob/master/python_modules/dagster/dagster/_core/definitions/data_version.py)
- Hamilton: [README](https://github.com/apache/hamilton/blob/main/README.md), [cache_key.py](https://github.com/apache/hamilton/blob/main/hamilton/caching/cache_key.py), [fingerprinting.py](https://github.com/apache/hamilton/blob/main/hamilton/caching/fingerprinting.py), [graph_types.py](https://github.com/apache/hamilton/blob/main/hamilton/graph_types.py)
- Prefect: [cache_policies.py](https://github.com/PrefectHQ/prefect/blob/main/src/prefect/cache_policies.py), [hashing.py](https://github.com/PrefectHQ/prefect/blob/main/src/prefect/utilities/hashing.py)
- Flyte: [caching docs](https://www.union.ai/docs/v2/flyte/user-guide/task-configuration/caching/)
- LangGraph: [_algo.py](https://github.com/langchain-ai/langgraph/blob/main/libs/langgraph/langgraph/pregel/_algo.py), [types.py](https://github.com/langchain-ai/langgraph/blob/main/libs/langgraph/langgraph/types.py), [_cache.py](https://github.com/langchain-ai/langgraph/blob/main/libs/langgraph/langgraph/_internal/_cache.py)
- Airflow: [PyPI](https://pypi.org/project/apache-airflow/)
- Crates: [salsa](https://crates.io/crates/salsa), [comemo](https://crates.io/crates/comemo), [differential-dataflow](https://crates.io/crates/differential-dataflow), [dbsp](https://crates.io/crates/dbsp), [rustc-stable-hash](https://crates.io/crates/rustc-stable-hash), [adapton](https://crates.io/crates/adapton)

**Dependency resolution**
- pubgrub: [solver.rs](https://github.com/pubgrub-rs/pubgrub/blob/main/src/solver.rs), [provider.rs](https://github.com/pubgrub-rs/pubgrub/blob/main/src/provider.rs), [arena.rs](https://github.com/pubgrub-rs/pubgrub/blob/main/src/internal/arena.rs), [type_aliases.rs](https://github.com/pubgrub-rs/pubgrub/blob/main/src/type_aliases.rs), [CHANGELOG](https://github.com/pubgrub-rs/pubgrub/blob/main/CHANGELOG.md), [issue #373](https://github.com/pubgrub-rs/pubgrub/issues/373)
- uv: [Cargo.toml](https://github.com/astral-sh/uv/blob/main/Cargo.toml), [resolver/mod.rs](https://github.com/astral-sh/uv/blob/main/crates/uv-resolver/src/resolver/mod.rs), [digest.rs](https://github.com/astral-sh/uv/blob/main/crates/uv-cache-key/src/digest.rs), [dirhash.rs](https://github.com/astral-sh/uv/blob/main/crates/uv-extract/src/dirhash.rs), [uv.lock](https://github.com/astral-sh/uv/blob/main/uv.lock), [resolver internals](https://docs.astral.sh/uv/reference/internals/resolver/), [cache](https://docs.astral.sh/uv/concepts/cache/), [sync](https://docs.astral.sh/uv/concepts/projects/sync/#checking-the-lockfile)
- Cargo resolver: [mod.rs](https://github.com/rust-lang/cargo/blob/master/src/resolver/mod.rs), [conflict_cache.rs](https://github.com/rust-lang/cargo/blob/master/src/resolver/conflict_cache.rs), [context.rs](https://github.com/rust-lang/cargo/blob/master/src/resolver/context.rs), [encode.rs](https://github.com/rust-lang/cargo/blob/master/src/resolver/encode.rs), [interning.rs](https://github.com/rust-lang/cargo/blob/master/src/util/interning.rs), [pubgrub-in-cargo goal](https://goals.rust-lang.org/2025h1/pubgrub-in-cargo.html)
- libsolv: [hash.h](https://github.com/openSUSE/libsolv/blob/master/src/hash.h), [history](https://github.com/openSUSE/libsolv/blob/master/doc/libsolv-history.txt)
- resolvelib: [resolution.py](https://github.com/sarugaku/resolvelib/blob/main/src/resolvelib/resolvers/resolution.py); resolvo: [repo](https://github.com/prefix-dev/resolvo)
- Go: [MVS](https://go.dev/ref/mod#minimal-version-selection), [go.sum](https://go.dev/ref/mod#go-sum-files), [checksum database](https://go.dev/ref/mod#checksum-database), [dirhash.go](https://github.com/golang/mod/blob/master/sumdb/dirhash/hash.go)
- Polonius: [polonius-engine Cargo.toml](https://raw.githubusercontent.com/rust-lang/polonius/master/polonius-engine/Cargo.toml)

**Hashers, collections and graph crates**
- [foldhash README](https://github.com/orlp/foldhash); rustc-hash: [lib.rs (docs.rs)](https://docs.rs/crate/rustc-hash/2.1.3/source/src/lib.rs), [lib.rs (GitHub)](https://github.com/rust-lang/rustc-hash/blob/master/src/lib.rs); [hashbrown tag.rs](https://docs.rs/crate/hashbrown/0.17.1/source/src/control/tag.rs)
- petgraph 0.8.3: [graph_impl/mod.rs](https://docs.rs/crate/petgraph/0.8.3/source/src/graph_impl/mod.rs), [stable_graph](https://docs.rs/crate/petgraph/0.8.3/source/src/graph_impl/stable_graph/mod.rs), [graphmap.rs](https://docs.rs/crate/petgraph/0.8.3/source/src/graphmap.rs), [csr.rs](https://docs.rs/crate/petgraph/0.8.3/source/src/csr.rs), [algo/](https://docs.rs/crate/petgraph/0.8.3/source/src/algo/), [tarjan_scc.rs](https://docs.rs/crate/petgraph/0.8.3/source/src/algo/scc/tarjan_scc.rs), [isomorphism.rs](https://docs.rs/crate/petgraph/0.8.3/source/src/algo/isomorphism.rs), [acyclic.rs](https://docs.rs/crate/petgraph/0.8.3/source/src/acyclic.rs)
- [rustworkx-core dictmap.rs](https://docs.rs/crate/rustworkx-core/0.18.1/source/src/dictmap.rs), [graph](https://docs.rs/crate/graph/0.3.2/source/src/lib.rs), [pathfinding](https://docs.rs/crate/pathfinding/4.16.0/source/src/lib.rs), [webgraph-algo](https://docs.rs/webgraph-algo/latest/webgraph_algo/), [incremental-topo](https://docs.rs/crate/incremental-topo/0.3.1/source/src/lib.rs), [incremental SCC search](https://crates.io/search?q=incremental%20scc)
- [lasso](https://docs.rs/crate/lasso/0.7.3/source/src/lib.rs), [internment arc.rs](https://docs.rs/crate/internment/0.8.6/source/src/arc.rs), [hashconsing](https://docs.rs/crate/hashconsing/1.8.0/source/src/lib.rs)
- [egg util.rs](https://docs.rs/crate/egg/0.11.0/source/src/util.rs), [egglog util.rs](https://docs.rs/crate/egglog/3.0.0/source/src/util.rs)
- [datafrog](https://docs.rs/crate/datafrog/2.0.1/source/src/lib.rs), [ascent](https://docs.rs/crate/ascent/0.8.1/source/src/c_lat_index.rs), [crepe](https://docs.rs/crate/crepe/0.2.0/source/src/lib.rs), [DDlog](https://github.com/vmware/differential-datalog)
- [nauty-pet](https://docs.rs/nauty-pet/latest/nauty_pet/), [canonical-form](https://crates.io/crates/canonical-form)
- [probminhash](https://docs.rs/crate/probminhash/0.1.12/source/src/superminhasher.rs), [xorf](https://docs.rs/crate/xorf/0.13.0/source/src/lib.rs), [fastbloom](https://docs.rs/crate/fastbloom/0.17.0/source/src/hasher.rs)
- [blake3 lib.rs](https://docs.rs/crate/blake3/1.8.7/source/src/lib.rs), [bincode 3.0.0](https://crates.io/crates/bincode/3.0.0), [python-xxhash CHANGELOG](https://github.com/ifduyue/python-xxhash/blob/master/CHANGELOG.rst)
- networkx WL: [graph_hashing.py](https://github.com/networkx/networkx/blob/main/networkx/algorithms/graph_hashing.py), [docs](https://networkx.org/documentation/stable/reference/algorithms/generated/networkx.algorithms.graph_hashing.weisfeiler_lehman_graph_hash.html), [#7806](https://github.com/networkx/networkx/issues/7806)
- Unison: [hashes](https://www.unison-lang.org/docs/language-reference/hashes), [the big idea](https://www.unison-lang.org/docs/the-big-idea/)

**Canonical encodings**
- [RFC 8785 (JCS)](https://www.rfc-editor.org/rfc/rfc8785), [RFC 8949 (CBOR)](https://www.rfc-editor.org/rfc/rfc8949), [DRISL](https://dasl.ing/drisl.html), [protobuf: not canonical](https://protobuf.dev/programming-guides/serialization-not-canonical/), [postcard wire format](https://postcard.jamesmunns.com/wire-format)

**Graph databases**
- SurrealDB:
  - Releases and crates: [crates.io](https://crates.io/crates/surrealdb), [3.3 release notes](https://surrealdb.com/releases/3.3), [3.x by the numbers](https://surrealdb.com/blog/surrealdb-3-x-by-the-numbers.md)
  - SDK and migration: [Rust after 3.0](https://surrealdb.com/docs/sdk/rust/concepts/rust-after-30), [2.x→3.x migration](https://surrealdb.com/docs/build/migrating/from-old-surrealdb-versions/2x-to-3x.md)
  - SurrealQL reference: [record ids](https://surrealdb.com/docs/surrealql/datamodel/ids), [idioms](https://surrealdb.com/docs/surrealql/datamodel/idioms), [DEFINE EVENT](https://surrealdb.com/docs/surrealql/statements/define/event), [DEFINE INDEX](https://surrealdb.com/docs/surrealql/statements/define/indexes), [LIVE](https://surrealdb.com/docs/surrealql/statements/live), [SHOW](https://surrealdb.com/docs/surrealql/statements/show)
  - Issues: [#7466](https://github.com/surrealdb/surrealdb/issues/7466)
- ArcadeDB:
  - Releases and drivers: [tags](https://github.com/ArcadeData/arcadedb/tags), [requirements](https://docs.arcadedb.com/arcadedb/reference/requirements.html), [arcadedb-driver](https://pypi.org/project/arcadedb-driver/)
  - Reference: [HTTP API](https://docs.arcadedb.com/arcadedb/reference/http-api/http.html), [CREATE EDGE](https://docs.arcadedb.com/arcadedb/reference/sql/sql-create-edge.html), [graph algorithms](https://docs.arcadedb.com/arcadedb/reference/graph-algorithms/chapter.html), [utility functions](https://docs.arcadedb.com/arcadedb/reference/extended-functions/utility.html), [materialized views](https://docs.arcadedb.com/arcadedb/how-to/data-modeling/materialized-views.html)

**Merkle-structured stores, CDC and columnar engines**
- Dolt: [node_splitter.go](https://github.com/dolthub/dolt/blob/main/go/store/prolly/tree/node_splitter.go), [hash.go](https://github.com/dolthub/dolt/blob/main/go/store/hash/hash.go), [prolly-tree docs](https://www.dolthub.com/docs/architecture/storage-engine/prolly-tree/), [history independence](https://www.dolthub.com/blog/2024-11-26-history-independence/)
- atproto: [repository spec](https://atproto.com/specs/repository)
- Git: [hash-function-transition](https://git-scm.com/docs/hash-function-transition), [BreakingChanges](https://github.com/git/git/blob/master/Documentation/BreakingChanges.adoc)
- Multiformats: [multihash](https://github.com/multiformats/multihash), [multicodec table](https://github.com/multiformats/multicodec/blob/master/table.csv); [merkle-search-tree](https://crates.io/crates/merkle-search-tree)
- FastCDC: [fastcdc-rs CHANGELOG](https://github.com/nlfiedler/fastcdc-rs/blob/master/CHANGELOG.md)
- Polars: [aliases.rs](https://github.com/pola-rs/polars/blob/main/crates/polars-utils/src/aliases.rs), [vector_hasher.rs](https://github.com/pola-rs/polars/blob/main/crates/polars-core/src/hashing/vector_hasher.rs), [hash_keys.rs](https://github.com/pola-rs/polars/blob/main/crates/polars-expr/src/hash_keys.rs)
- DuckDB: [hash.hpp](https://github.com/duckdb/duckdb/blob/main/src/include/duckdb/common/types/hash.hpp), [ht_entry.hpp](https://github.com/duckdb/duckdb/blob/main/src/include/duckdb/execution/ht_entry.hpp), [recursive CTE blog (2.0 preview)](https://duckdb.org/2026/08/25/how-duckdb-runs-recursive-ctes-faster); [DuckPGQ SQL/PGQ docs](https://duckpgq.org/documentation/sql_pgq/)
- PostgreSQL 19 SQL/PGQ revert: [Command Prompt](https://www.commandprompt.com/blog/two-features-just-left-postgresql-19/)
