# A6: dependency fit of SurrealDB 3.3 in the library-context workspace

Date: 2026-10-05. Static resolution only. Nothing was compiled, and the repository was not modified.
Scratch root: `/tmp/claude-1000/-home-paul-library-context/3307aa74-dd84-437f-93be-1d92a29d0cce/scratchpad/a6-deps/`.

## Method

- **Baseline.** I copied `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `crates/` (with `tests/` excluded), `third_party/` and the manifests and sources of the two Python-binding crates into `a6-deps/ws/`. The copy uses a minimal `.cargo/config.toml` that keeps `feature-unification = "workspace"`.
- **Baseline check.** `cargo metadata --locked --offline` returned rc=0, with 857 lock entries.
- **Variants.** Each variant adds one member, `crates/surreal-probe`, and is resolved with `cargo metadata`. That command extends the existing lock minimally.

| Variant | dir | `surrealdb` requirement | rc | Lock entries added | New crates in the Linux normal+build closure | Probe closure |
|---|---|---|---|---|---|---|
| remote | v-remote | `"3.3"`, defaults (`protocol-ws`, `rustls`, `parse`) | 0 | 72 | 55 | 242 |
| SurrealKV | v-skv | `["kv-surrealkv","protocol-ws"]` | 0 | 207 | 153 | 412 |
| full brief | v-full | `["kv-rocksdb","kv-surrealkv","protocol-ws"]` | 0 | 220 | 166 | 428 |
| slim | v-slim | `default-features=false`, `["kv-surrealkv","protocol-ws"]` | 0 | 204 | 152 | 411 |
| +gql | v-gql | v-full plus `surrealdb-core = {version="3.3", default-features=false, features=["gql"]}` | 0 | 221 | 167 | 429 |

How the counts were made:
- The closure counts come from `cargo tree --offline --workspace -e normal,build --target x86_64-unknown-linux-gnu`. They are unique (name, version) pairs, so dev-dependencies are excluded.
- Lock entries also cover every platform: windows-*, wasm-* and objc2.
- The baseline workspace closure has 681 crates. The `set-*.txt` and `probe-*.txt` files hold the full lists.
- Resolution "locked to the highest Rust 1.98.1 compatible versions", which is the workspace's `rust-version`.
- No existing lock entry changed version. The only change was "Adding" new entries, with no "Updating" or "Downgrading" lines.

## Resolution and the single-version families

- **Result.** All variants resolve with no conflicts.
- **Family check.** `uv run --no-sync python scripts/check_family.py <variant>/Cargo.lock` (run from the repo, read-only) printed `family ok` for ws, v-remote, v-skv, v-full and v-slim. The check covers Arrow, DataFusion, object_store, petgraph, pyrefly, blake3, the Ruff/ty sources and salsa 0.28.5.
- **Family crates that SurrealDB itself depends on:**
  - `object_store` (from `surrealdb-core`, `^0.13.2`) unifies to the locked 0.13.2.
  - `blake3` (from `surrealdb-datastore` and `surrealdb-runtime`, `^1.8.4`) unifies to the exact 1.8.6 pin.
  - It does not depend on Arrow, DataFusion, petgraph, salsa or Ruff.
- **Feature-unification side effect on a family crate.** This changes features only, not the version:
  - `surrealdb-core` declares `[target.'cfg(not(target_family="wasm"))'.dependencies.object_store] features = ["aws","gcp","azure","tls-webpki-roots"]` with default features on. The upstream comment says these "pull an HTTP/TLS stack".
  - The workspace's object_store moves from `fs, tokio, walkdir` to the full cloud set: aws, azure, gcp, reqwest, quick-xml, ring, md-5, rand, serde and others. Under workspace feature unification, the DataFusion stack would compile against that wider object_store.
- **Other shared crates.**
  - `reqwest 0.12.28` gains `rustls-tls-*` and `http2` features.
  - `uuid` gains `serde` and `v7`.
  - tokio, serde, serde_json, chrono and rustls are unchanged.

## Version duplicates introduced

These are on Linux, in the normal+build closure, for v-full. None of them is a family crate.

| Crate | New | Already there |
|---|---|---|
| hmac | 0.12.1 | 0.13.0 |
| md-5 | 0.10.6 | 0.11.0 |
| pem | 3.0.6 | 4.0.0 |
| lz4_flex | 0.12.2 | 0.14.0 |
| quick-xml | 0.39.4 | 0.41.0 |
| shlex | 1.3.0 | 2.0.1 |
| untrusted | 0.7.1 | 0.9.0 |
| phf / phf_shared / phf_codegen / phf_generator | 0.13 and 0.14 | 0.11.3, 0.12.1 and 0.13.1 |
| webpki-roots | 0.26.11 and 1.0.9 (both new) | — |
| rstar | 0.8 to 0.13 (six versions, via geo; lock-wide, including other targets) | — |
| heapless / hash32 | multiple | — |

- `cargo tree -d --depth 0` lists 87 duplicated names in the baseline and 101 in v-full.
- The remote variant adds only hmac 0.12 and phf_generator 0.13.

## New crates and heavy native dependencies (v-full)

- **SurrealDB's own crates (26).** These are `surrealdb`, `-core`, `-types`, `-types-derive`, `-syn`, `-sql`, `-expr`, `-catalog`, `-datastore`, `-idx`, `-runtime`, `-kvs`, `-kvs-any`, `-kvs-rocksdb`, `-kvs-surrealkv`, `-engine-api`, `-engine-local`, `-iam`, `-rpc`, `-protocol`, `-cnf`, `-common`, `-collections`, `-observe`, `-parse-common`, `-strand` and `-keyspace-macro`, plus `surrealkv 0.21.4` and `vart`.
- **RocksDB (only with `kv-rocksdb`).** These are the extra 13 crates in v-full over v-skv:
  - `surrealdb-librocksdb-sys 0.18.3+11.0.0-4` (RocksDB 11 in C++, static build) and `surrealdb-rocksdb 0.24.0-surreal.5`.
  - Feature `bindgen-runtime`, which brings `bindgen 0.72`, `clang-sys` and `libloading`.
  - `libz-sys`, `lz4-sys` and `bzip2-sys`; `zstd-sys` is already in the graph.
  - Upstream documentation says it needs a C++ compiler and libclang. This host has `/usr/lib/llvm-18/lib/libclang.so.1`, `/usr/bin/c++` and the repository's clang 23. I did not verify that bindgen finds the `.so.1`.
- **aws-lc-sys 0.45 (C/CMake).**
  - It enters through `jsonwebtoken 10.4` with `aws_lc_rs` (from `surrealdb-core`/`-expr`), and through the SDK `rustls` feature (`rustls/aws_lc_rs` on native targets).
  - aws-lc-sys is in today's lock only through the hakari workspace-hack (`hyper-rustls`). Without the hack it is new.
  - It is present even with `default-features = false` (v-slim), because jsonwebtoken pulls it in.
- **Other weight that comes in for any `kv-*` (core) build:**
  - lindera 6.2 with lindera-dictionary, with no embedded dictionaries unless `cjk` is set;
  - diskann 0.56 with ndarray 0.17 (vector index);
  - geo 0.32 with i_overlay and rstar;
  - html5ever and ammonia;
  - rkyv 0.8;
  - argon2, bcrypt, scrypt, rsa, ed25519-dalek, p256 and p384;
  - sysinfo 0.37;
  - fst, roaring, tokio-tungstenite 0.28 and surrealdb-syn.
- **Absent in all variants (checked):** wasmtime (Surrealism), rquickjs (scripting), tonic/prost (grpc), axum, async-graphql, `surrealdb-gql`, mimalloc/jemallocator (`allocator`), reqwest 0.13 (`http`/`jwks`), tikv and surrealml.

## Feature-flag options

These come from the SDK and core registry `Cargo.toml` at 3.3.0, and the skill's `surrealdb.features-and-builds` brief.
- **Defaults.** The SDK defaults are `protocol-ws`, `rustls` and `parse`. A remote client with no core adds 55 crates.
- **Any `kv-*`.** Any `kv-*` feature brings in `surrealdb-engine-local` and the whole core.
- **Slimming has little effect.** `default-features = false` saves only `webpki-roots 0.26` (412 → 411 crates). The core dominates.
- **SurrealKV versus RocksDB.** `kv-surrealkv` (pure Rust) is 13 crates and one C++ build lighter than `kv-rocksdb`. `kv-mem` alone is lighter still, and is not measured here.
- **Forwarding features.** `scripting`, `http`, `cjk`, `ml`, `jwks`, `allocator`, `arbitrary` and `allocation-tracking` forward to the engine unconditionally. Each brings in the core even without a `kv-*` feature, so leave them off unless they are needed.
- **GQL and GraphQL are off in embedded builds.**
  - `surrealdb-engine-local` depends on `surrealdb-core` with `default-features = false`.
  - The core's defaults (`kv-mem`, `graphql`, `gql`) are therefore off, and the ISO GQL parser (`surrealdb-gql`) is not compiled.
  - The SDK has no feature that forwards `gql`. Adding a direct `surrealdb-core` dependency with `features=["gql"]` turns it on through unification: v-gql adds exactly `surrealdb-gql 3.3.0`.
  - The skill also records that an SDK-embedded engine cannot run `eval::gql`, and that the SDK has no GQL method.
- **TLS.** `rustls` uses aws-lc-rs on native targets. `native-tls` is the OpenSSL alternative. The workspace's sqlx uses rustls-ring, and ring stays in the graph.

## MSRV and fit with the nightly toolchain

- The `surrealdb` and `surrealdb-core` 3.3.0 manifests declare no `rust-version`. The skill's probe SX001 measured that the default SDK compiles on 1.95.0 and fails on 1.94.0 with E0658.
- `surrealkv` declares 1.86 and `surrealdb-librocksdb-sys` declares 1.85.0.
- The workspace minimum is 1.98.1 and the toolchain is nightly-2026-09-29 (1.101.0-nightly). The resolver accepted every crate as compatible with 1.98.1.
- The skill's probes ran on stable 1.99.0 (2026-09-28). No compile was run on this nightly, so nightly-specific lints or breakage are not_run.
- The workspace sets `unsafe_code = "forbid"` and Clippy warn lints. These apply only to workspace crates, not to dependencies.

## cargo-deny (facts only; licences are not a criterion)

Command: `cargo deny --offline --log-level error check bans sources licenses`, run with the repository's `deny.toml` copied into scratch.

| Check | ws (baseline) | v-full |
|---|---|---|
| Overall | rc=0 | rc=4 |
| bans | pass | pass (`multiple-versions = "allow"`) |
| sources | pass | pass: every SurrealDB crate is from crates.io; no git, no new registry |
| licences | pass | fail |

The licence failures in v-full:
- **26 `unlicensed`.** These are the `surrealdb*` crates that use `license-file = "LICENSE"`. The file is Business Source License 1.1 (Licensor SurrealDB Ltd., with an Additional Use Grant that forbids use "as a Database Service").
- **2 `rejected`.** `surrealdb-protocol 0.13.1` is `BUSL-1.1`, and `ext-sort 0.1.6` is `Unlicense`. Neither is on the allow list.

To admit these, `deny.toml` would need `BUSL-1.1` and `Unlicense` on the allow list (or as exceptions), plus a `[[licenses.clarify]]` (or exception) entry for the license-file crates. That would be a proposal to widen `deny.toml`, following the user's working preference.

Advisories were not run.

## The other side of the ledger: removing PostgreSQL and DataFusion

Method:
- I removed `sqlx`, `pgpq`, `sea-query`, `pgvector`, `datafusion`, `datafusion-table-providers-postgres`, `tokio-postgres` and `testcontainers-modules` from every crate manifest.
- I emptied `lctx-workspace-hack`'s dependencies in both the comparison baseline (`ws-nohack`) and the removal variants, because the hack crate would otherwise keep those crates' features pinned.
- Arrow stays, since `lctx-model` uses `serde_arrow` and `arrow-*`.

| Set | Linux normal+build closure |
|---|---|
| ws-nohack | 669 |
| nopg (removal only) | 511, i.e. 158 crates leave |
| nopg-full (removal plus SurrealDB v-full) | 719, i.e. +174 / −124 against ws-nohack, a net +50 |

What leaves (see `removed-pgdf.txt`):
- DataFusion: all 30 `datafusion-*` 55.1.0 crates, plus sqlparser 0.62.
- Arrow: the arrow umbrella and its arith, cast, csv, ipc, json, ord and string crates. arrow-array, -buffer, -data, -row, -schema and -select remain.
- Parquet, compression and hashing: parquet 59.3, brotli, zstd 0.13, lz4_flex 0.14, flatbuffers, twox-hash, snap.
- PostgreSQL: sqlx and sqlx-core/-postgres/-macros, tokio-postgres, postgres-protocol, postgres-types, postgres-native-tls, bb8 and bb8-postgres, pgpq, pgvector, sea-query and its derive, rust_decimal, bigdecimal.
- TLS and DNS: native-tls, openssl, openssl-sys, openssl-macros, tokio-native-tls, rustls, rustls-webpki, ring, hickory-*.
- Other: moka.

Under SurrealDB, 34 of those 158 come back. They are object_store 0.13.2, rustls, rustls-webpki, ring, rustls-native-certs, rust_decimal, flatbuffers, snap, twox-hash, geo-types, logos, time and stringprep, among others.

OpenSSL leaves the graph completely. The C and C++ builds trade as follows:
- **Gone:** openssl-sys, which links the system OpenSSL.
- **Added:** aws-lc-sys and, with `kv-rocksdb`, the RocksDB C++ build together with libz-sys, lz4-sys and bzip2-sys.
- **Stays:** zstd-sys.

## Stability, and pinning under `docs/pins.md`

What upstream says:
- `surrealdb-core/src/lib.rs` (3.3.0): "This crate is **SurrealDB internal API**. It does not adhere to SemVer and its API is free to change and break code even between patch versions… a stable interface… [is] the Rust SDK."
- The core README says it "should not be used outside of SurrealDB itself".

How the crates depend on each other:
- The SDK and `surrealdb-engine-local` depend on every internal crate with a caret requirement (`version = "3.3.0"`), not an exact one.
- So `surrealdb = "3.3"` (a caret) lets `just upgrade` move the SDK and all internal crates together to the newest 3.x.

How the pins policy applies:
- **The SDK alone.** Through the SDK alone, the default float policy plus the lockfile applies. The SDK is presented as the stable interface, and a caret requirement needs no row.
- **Reaching the core directly.** If the code reaches `surrealdb-core` directly, for example `kvs::Datastore` or the `gql` feature-unification trick, that is a dependency-specific, overt reason under the pins rules: upstream disclaims SemVer even across patch releases.
- **What an exact pin would look like.** An exact `=3.3.0` on `surrealdb-core` (and, to keep one coherent family, on `surrealdb`) would then be justified, with a `docs/pins.md` row. Its reason would be "core is not SemVer; type-sharing internal family must resolve to one release". Revisit it at a deliberate SurrealDB upgrade.
- **Adding it to `check_family.py`.** It may be worth adding `surrealdb(-.+)?` to `check_family.py`'s FAMILY pattern. The 26 internal crates must resolve to one version together. Cargo already guarantees this within one semver-compatible range, but not across a mixed 3.x/4.x graph.
- **The skill's pin.** The skill indexes exactly 3.3.0, and pins that exist so a library skill's claims transfer are also allowed.

## Files written (scratch only)

- `a6-dependency-fit.md` (this file)
- `a6-deps/`: copies `ws`, `ws-nohack`, `v-remote`, `v-skv`, `v-full`, `v-slim`, `v-gql`, `nopg` and `nopg-full`; outputs `meta-*.json`, `meta-*.err`, `set-*.txt`, `probe-*.txt`, `new-*.txt`, `removed-pgdf.txt`, `dupnames-*.txt`, `deny-ws.log`, `deny-v-full.log`, `adding-full.txt`, `base-metadata.json` and `base-Cargo.lock`

## Uncertainties

- Nothing was compiled: SurrealDB on nightly-2026-09-29, RocksDB/bindgen against `libclang.so.1`, aws-lc-sys under mold/clang, and build time and disk are all not_run.
- The counts are (name, version) pairs for x86_64 Linux normal and build edges, and include proc-macros and build-only crates. Dev-only crates and other targets are excluded.
- The removal ledger assumes the hakari hack is regenerated. Arrow's retained crates depend on `lctx-model` keeping serde_arrow.
- `cargo deny` advisories were not checked.
- Context7 (`/surrealdb/surrealdb`) returned only general backend notes. The feature facts come from the registry manifests at 3.3.0 and from the shared `neo4j-surrealdb` skill corpus (`surrealdb/content/corpus`), which are authoritative for this version.
