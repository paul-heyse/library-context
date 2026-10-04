# N1: new Rust libraries for generic mechanisms

Supporting evidence for the
[library-leverage review](../../reviews/design_review_library-leverage_2026-10-04.md). Lane N1
(library-research). Baseline `948b2a88`. HEAD at check time was `b0256115`, which changes only a
snapshot under `crates/` (`git diff --name-only 948b2a88 HEAD -- crates`), so every location cited
here matches the baseline.

**Method.**
- Static reading of the cited source, the M1–M4 inventories and lanes L1/L4.
- crates.io API metadata for every candidate, fetched 2026-10-04 (the latest stable version, the
  date it was published, recent downloads and declared features).
- `Cargo.lock` and `cargo tree -i` to establish which crates are already compiled.
- The local RustSec advisory database (last commit 2026-09-23).
- Context7 for facet.
- docs.rs for ciborium.
- The pg_query.rs changelog.
- Local registry sources for arrow-schema 56.2.0/57.3.1/59.3.0, hashbrown 0.16.1/0.17.1,
  indexmap 2.14.2, memory-stats 1.2.0 and serde_json 1.0.151.

No probe was run and nothing was built. Library behaviour is labelled `Documented` (stated in
docs or source) or `Observed (source)` (read in the source at the stated version). Fit judgements
are `Proposed`.

L3 owns reasoning and graph crates, typed-index crates and heap-size crates (get-size2, deepsize).
This lane only cross-references them.

**Corrections to the brief.**
- **proptest is not pinned.** It is absent from `Cargo.lock` and from every `Cargo.toml`.
- **trybuild is not pinned.**
- **insta is pinned** at `=1.48.0`. The latest release is 1.49.0, published 2026-10-03.

## 1. Candidate table

Column key:
- **Lock** says whether the crate is already compiled. "yes (X)" means it is compiled for the
  target through X. "lock only" means it appears in `Cargo.lock` but `cargo tree -i` shows no
  compiled path. "no" means it would be a new crate.
- **Served** lists the inventory IDs. A candidate that serves none is listed only to rule it out.

| Crate · latest · date | Served | Displaces / enables | Does not preserve | Burden | Lock |
|---|---|---|---|---|---|
| **Own derive extension** (`lctx-model-macros`, syn/quote) | M2-04, §2 of M2, M2-07, M2-16, M4-09 | A `PortSet` derive with a generic-method visitor. It replaces the 66 exported lists and their 408 local re-expansions, and gives the 11-relation publication set one named type. `DomainCode` would also emit `JsonSchema`, `label()` and `ALL`. | Nothing by construction. It keeps the codebook discipline (explicit i16 codes, append-only) and one declaration. | No new dependency. The work is the proc-macro code plus its tests (currently none in `lctx-model-macros`). | n/a |
| facet 0.46.5 · 2026-05-26 | M2-04 (in principle) | Runtime `Shape` reflection: field names, offsets and extension attributes. | Shapes carry vtables only for auto-detected std traits (Context7, facet docs: "VTable with auto-detected trait implementations"). Calling `Record::decode`/`NAME` per field still needs a hand registry. Port lists become runtime data rather than types. | Pre-1.0 with 118 releases since 2025-04; a heavy proc-macro; MSRV 1.90. | no |
| frunk 0.5.0 · 2026-07-04 | M2-04 | `LabelledGeneric`/HList plus `Poly` for type-level iteration over fields. | Nothing semantic. The problem is ergonomics and diagnostics: the HList types surface in every signature and error. | A new crate and its derive. Slow type-checking for 30-field HLists (Proposed, not measured). | no |
| bevy_reflect 0.19.1 · 2026-08-13 | — | Runtime reflection with a type registry. | Same gap as facet, plus the Bevy ecosystem churn. | Large dependency set. Ruled out. | no |
| strum 0.28.0 · 2026-02-22 | M2-16 (non-codebook enums only) | `VariantArray`/`EnumIter`, `IntoStaticStr` and `EnumString` for `Stage`, `Tool`, `FailureKind` and the other 11 `ALL` arrays. | For `DomainCode` enums strum would create a second label/discriminant source, which is the b0 reason and still holds. For wire names hashed into identities (`Tool::name`), every name must be explicit (`#[strum(serialize=…)]`). | Near zero: `strum` with `derive` is already unified in `lctx-workspace-hack/Cargo.toml:107`. | yes (jsonschema dev + workspace-hack) |
| enum-map 3.1.0 · 2026-07-21 | — | Arrays indexed by enum. | No per-variant table consumer was found in M1/M2. | MSRV 1.95. Ruled out for now. | no |
| derive_more 2.1.1 · 2025-12-22 | M2-13 (Display labels) | `Display`/`From`/`Deref` derives. | `Display` text becomes corpus/identity text. It would need explicit format attributes per variant, which is what an own `label()` gives directly. | Compiled via pyrefly_build. | yes (pyrefly_build) |
| schemars 1.2.2 derive (pinned) | M2-07 | L1 covers the sum derive. This lane adds that the 76-type `code_schema!` list is best emitted by `DomainCode` itself, as an integer `enum` plus `x-` labels. | The exposure set (which types reach the wire) still needs a declaration. | Pinned. | yes |
| multi_index_map 0.15.1 · 2026-01-18 | M2-01 | A derive that generates a container with hashed or ordered, unique or non-unique indexes over a slab. `insert` fails on a unique conflict. | It is per-struct (fields of one row type). Our secondary indexes are cross-relation (`Frame→Id`, `Id→Vec<Id>`). It has no allocation hook; charging stays an external estimate. `iter()` follows slab order, so deterministic order needs an ordered index. Conflict means a duplicate key, not "same key, different payload". | Proc-macro per relation; 0.x; 55k recent downloads. | no |
| indexmap 2.14.2 · 2026-09-05 | M2-01, M1-19 | Insertion-ordered map with `try_reserve` (`map.rs:401`) and `sort_keys`. | Order is insertion order, not `Id` order. It needs `sort_keys` before every observable iteration, otherwise determinism depends on input order. | Already unified (with serde). | yes |
| hashbrown 0.17.1 · 2026-05-09 (`HashTable`; std raw-entry is gone) | M1-16, M1-19, M2-01 | `try_reserve` plus `allocation_size()` (`map.rs:1131,2006`). This gives exact, fallible capacity charging instead of `NODE_ALLOWANCE` estimates, and `try_insert` gives conflict detection. | Iteration order is arbitrary, so output must be sorted. Five hashbrown versions are already in the lock (0.12, 0.14, 0.15, 0.16, 0.17), so a direct dependency picks one. | Low. | yes (several) |
| imbl 7.0.2 · 2026-09-09 (im is archived: RUSTSEC-2026-0248) | M1 Vocabulary deep copy (`summary_production.rs:1617-1632`) | Structural sharing for branch-and-copy state. | Byte charging of shared nodes is undefined: who pays for a shared node? No measurement fires the b0 trigger. | A new crate. | no |
| crepe 0.2.0 · 2025-12-14 | M2-02 (semijoins), M1-08 | Compile-time Datalog to Rust. | Hash-set relations, no budget, and output order follows the hash set. L3 owns ascent, which dominates crepe (lattices, aggregates). | — | no |
| egglog 3.0.0 / differential-dataflow 0.25.1 | — | E-graphs; incremental dataflow on timely. | No rewrite consumer; no continuous insert/delete view (b0 triggers unfired). The timely runtime is not pure. | Large. Ruled out. | no |
| polars | — | DataFrames. | Violates ADR-0085 purity: it would be a second compute engine beside DataFusion. It also brings its own Arrow (polars-arrow) outside the Arrow 59 family. | Ruled out. | no |
| hex 0.4.3 · 2021-03-03 (stable, finished) | M2-06 | `hex::encode`/`decode` for `Id::hex`, `ContentHash::hex` and the cursor codec (`cursor.rs:62-91`). | Nothing. Current decode accepts uppercase (`is_ascii_hexdigit`) and so does `hex::decode`, so accepted tokens are unchanged. `encode` emits lowercase, as now. | Zero: it is a normal dependency of datafusion-functions and in workspace-hack (`:63`). | yes |
| const-hex 1.19.3 / faster-hex 1.0.0 | M2-06 | SIMD hex. | Speed is not a need for 16- to 32-byte values. | faster-hex 0.6.1 is compiled via pyrefly (an old major). | 0.6.1 via pyrefly |
| blake3 `Hash::to_hex`/`from_hex` (pinned) | M2-06 (`ContentHash` only) | A built-in fixed-size hex. | `Id` (16 bytes) is not a `blake3::Hash`. | Pinned. | yes |
| serde_json_canonicalizer 0.3.2 · 2026-02-03 | M2-05 (JSON bytes in `selection_digest`, `wire_identity`, `EmbeddingSpec::canonical_json`) | RFC 8785 (JCS): sorted keys and ECMAScript number formatting, independent of the `preserve_order` feature and of the serde_json float formatter. | Strings and numbers follow RFC 8785. That is a byte change for any current identity carrying floats or maps, so it is a rebuild under ADR-0078. | No features; small; MIT. | no |
| serde_jcs 0.2.0 · 2026-03-25 / json-canon 0.1.3 · 2023-05-13 | M2-05 | RFC 8785 alternatives. | serde_jcs had a 5-year gap between its two releases; json-canon is stale. | — | no |
| ciborium 0.2.2 · 2024-01-24 | M2-05 | CBOR via serde. | The serializer does not sort map keys. Canonical ordering exists only through `value::CanonicalValue` (docs.rs), which means a detour through `Value`. Enum variants serialize by name. | Last release 2024. | no |
| dcbor 0.25.2 · 2026-03-16 | M2-05 | Deterministic CBOR (dCBOR draft): numeric reduction, sorted maps, NFC. | It is not serde-native (own traits), so it is a second encoding authority beside `Key`. | BSD-2-Clause-Patent; 0.x with 60 releases. | no |
| serde_cbor 0.11.2 | — | — | Unmaintained (RUSTSEC-2021-0127). | Ruled out. | no |
| bcs 0.2.1 · 2026-04-25 | M2-05 | Canonical serde binary: maps sorted by key bytes. | No floats at all, so `FiniteF64` fails. Enums encode by position. | Small. | no |
| borsh 1.8.1 · 2026-08-26 | M2-05 | A specified binary format with its own derive, sorted `HashMap`, explicit enum discriminants (`use_discriminant`). | It is a second derive and a second encoding authority beside `Key`/postcard. | Lock only (an optional dependency of something; `cargo tree -i borsh` shows no path). | lock only |
| nutype 0.8.0 · 2026-09-20 | M2-15 (`Text<MIN,MAX>`), M1-20 | A newtype with sanitize/validate and serde. | Its JsonSchema support is the `schemars08` feature only, but the repo pins schemars 1.2.2, so the custom `x-minUtf8Bytes` schema stays hand-written. Const-generic newtypes are not supported (Proposed, from the feature set and docs; not compiled). | Proc-macro. | no |
| garde 0.23.0 · 2026-05-23 | M2-15, ADR-0073's dual limit | `length(chars, …)` and `length(bytes, …)` modes match ADR-0073's "500 scalars + byte budget". Also contexts and custom validators. | Of the 90 row validators, 12 are cross-field "status implies none" rules (`custom` fns either way). Range checks are already one attribute in the own derive (`#[model(validate=…)]`). | Proc-macro. | no |
| validator 0.21.0 | M2-15 | — | Dominated by garde (no contexts, weaker length modes). | — | no |
| bon 3.10.2 · 2026-10-02 / typed-builder 0.23.2 | M2 `MethodParameters` (10 `None`s at `catalog/build.rs:765-775`) | Compile-time-checked builders. | Nothing, but `Default` plus struct-update syntax already removes the `None`s, and no required-field consumer was found. | Proc-macro compile cost. | no |
| ordered-float 5.5.0 / decorum 0.4.0 | M1-20 | `NotNan`/total-order floats. | Neither is finite-only with one zero representation in bits. `NotNan` admits ±inf and keeps the −0.0 bits. Neither supplies the `Key` encoding. decorum is stale (2024-11). | — | no |
| figment 0.10.19 · 2024-05-17 / config 0.15.27 · 2026-09-30 | M4-12 | Layered configuration (file, env, profiles). | The current input is one protected 0600 JSON file with no layers. Neither performs the mode check, the `verify-full` rule or the "no secret in Debug" rule. figment has had no release for 28 months. | — | no |
| minijinja 2.24.0 · 2026-08-12 (3.0.0-alpha.2 exists) | M2-13 | Runtime templates. `fuel` (bounded instructions), `preserve_order`, no I/O by default. | Engine output (whitespace, filters, number formatting) becomes part of every content hash, so an engine upgrade forces a rebuild. It does not fix the Debug renderings, which are the real stability defect. | Pure; moderate size. | no |
| askama 0.16.1 · 2026-09-04 | M2-13 | Compile-time typed templates. | Same rendering-identity coupling (smaller, because filters are code). Does not fix Debug. | Proc-macro and template build step. | no |
| tera 2.4.0 / maud 0.27.0 | — | — | tera is dominated by minijinja (same author family; minijinja is the successor design). maud is HTML-only. | — | no |
| unicode-ident 1.0.26 · 2026-09-17 | M3-15 (`is_dotted`, `tokens`) | UAX #31 XID predicates. | Python identifiers are PEP 3131 (XID plus NFKC). `ruff_python_stdlib::identifiers::is_identifier` (pinned, L2/M3) is the Python authority. | Compiled for proc-macro2. | yes |
| serde-saphyr 1.3.0 · 2026-09-16 / yaml-rust2 0.13.0 | M3-15 (`title()`) | Real YAML for frontmatter, which `markdown` 1.0.0 already isolates as a `Yaml` node with `constructs.frontmatter`. serde-saphyr advertises parse budgets (depth/alias). | One field (`title`) is consumed. | New crate. serde_yml is unsound and archived (RUSTSEC-2025-0068); serde_yaml is deprecated. | no |
| aho-corasick 1.1.5 | M3-15 | Multi-pattern substring search. | The recognizer tokenizes on identifier boundaries and looks up a map (`document_parser.rs:158-247`). Substring automata would match across boundaries, which is worse. | Compiled. | yes |
| memchr 2.8.3 / textwrap 0.16.4 | M2-12 | memchr: newline search in `windows`. textwrap: display-width wrapping. | textwrap wraps by display width, not byte budget, so it does not fit byte-exact windows. memchr is a micro-optimization only. | memchr compiled. | memchr yes |
| rust-lapper 1.3.0 (2.0.0-beta.1) / iset 0.3.3 / coitrees 0.4.0 / intervaltree 0.2.7 | M2-10 | Overlap queries. | All answer overlap, not innermost containment. No work budget: rust-lapper's hidden scan (back off by `max_len`, then scan) is not observable, so `visited_nodes` cannot be counted. No fallible or charged allocation. coitrees and intervaltree are stale. | — | no |
| pg_query 6.2.1 · 2026-09-30 | M4-04, M1/M4 view-text normalization (`serving_shape.rs:20-50`) | A libpg_query parse, `fingerprint` and normalize for comparing `pg_get_viewdef`/CHECK text structurally instead of stripping whitespace and quotes. | It is the **PostgreSQL 17** parser (libpg_query 17-6.2.5, changelog); the store is PG18. It is not a schema differ: the finding taxonomy, shadow-in-rollback and ACL inspection stay ours. | A C build (cc + bindgen), prost. | no |
| sqlparser 0.62.0 (pinned via DataFusion; 0.63.0 latest) | M4-04 view-text comparison | Parse both view texts and compare ASTs, with no new crate. | Coverage of PG `pg_get_viewdef` output (casts, `ARRAY[]`) is unverified. | Pinned. | yes |
| pgdiff 0.1.0, pgcrate 0.7.0, dibs 0.1.1, rust-pgdatadiff 0.1.9 | M4-04 | Migration-oriented diff or data diff against live databases. | None runs inside a rolled-back transaction against a shadow schema, and none reports ACLs or our finding kinds. All are young or low-use (recent downloads: 1, 11, 214, 221). | — | no |
| tokio-postgres `error::SqlState` (pinned, 0.7.18) / sqlx `DatabaseError::kind()` (pinned) | M4-13 | Named SQLSTATE constants, generated from PostgreSQL's errcodes. sqlx's `ErrorKind` gives `UniqueViolation`/`ForeignKeyViolation`/`NotNullViolation`/`CheckViolation`. | There is no class-to-retry policy; the three divergent mappings must still be merged into one own function. `lctx-postgres` does not depend on tokio-postgres today. | tokio-postgres is compiled (cpg-core). | yes |
| sqlstate 0.1.0 · 2021 | M4-13 | — | Abandoned (8 recent downloads). | Ruled out. | no |
| backon 1.6.0 · 2025-10-18 | M4-14, `locks.rs:35-52` | A `Retryable` combinator with `.when(predicate)`, a max-times cap, jittered exponential or constant backoff and `tokio-sleep`. | The semantic part (committed-winner readback before retry, `cache.rs:103-150`) stays ours. There are two call sites. | Small new crate. backoff is unmaintained (RUSTSEC-2025-0012). tokio-retry 0.3.2 and tokio-retry2 0.9.1 are alternatives. | no |
| memory-stats 1.2.0 · 2024-06-26 | M4-11 (`stage_runtime.rs:18` VmRSS sampler, 20 ms) | Cross-platform RSS. | On Linux it reads and sums all of `/proc/self/smaps` per call (`linux.rs`, unless `always_use_statm`), far heavier than one `/proc/self/status` line at 50 Hz. Enabling `always_use_statm` would feature-unify into pyrefly_util's use. It has no peak (VmHWM). | Already compiled via pyrefly_util. | yes |
| sysinfo 0.39.6 | M4-11 | System-wide metrics. | Heavy, and refresh-oriented. | MSRV 1.95. | no |
| proptest 1.11.0 · 2026-03-24 | M2-10, M2-06, M2-05 framing, M1-17 | Differential properties: attachment index against `attachment_oracle.rs::scalar`; cursor round trip; `framed_part` injectivity. | n/a (a test). | New dev-dependency. | **no** |
| arbitrary 1.4.2 / bolero 0.13.7 · 2026-10-03 | same | bolero runs one harness under proptest-like random, libFuzzer or AFL engines (and Kani). | — | bolero needs nightly sanitizers for fuzz engines (the toolchain is nightly anyway). | no |
| trybuild 1.0.121 · 2026-09-08 | Own derive errors (DomainCode duplicate or missing code, a future PortSet) | Snapshot of compile-error diagnostics. The 18 existing `compile_fail` doctests only assert failure, not which error. | stderr snapshots change with nightly bumps (pinned `nightly-2026-09-29`), so they churn on every toolchain move. | Dev-only. | no |
| cargo-mutants 27.1.0 · 2026-06-02 | Process tool (`attachment.rs` has 0 unit tests; `charged.rs`/`rows.rs` none) | Finds untested decision points. | Not a dependency. An optional audit, not a gate. | A tool install only. | n/a |

## 2. Per-cluster findings

### 2.1 Stage-port and relation-list repetition (M2-04, M2 §2, M2-07, M2-16, M4-09)

**What the lists encode** (M2 §2):
- per field: name, `Record` type, transport, epoch, profile membership and an optional validator;
- per stage: unions of these sets.

The consumers are generic over `R: Record`. They need `R::NAME`, `Rows<R>::decode`,
`RelationUse::stored::<R>()`, `ValidationInput::of::<R>` and an async `load::<R>` in cpg-core.
Generating them therefore means **calling generic code once per field type**.

**Library options:**
- **facet / bevy_reflect** provide runtime field metadata. Their vtables cover only std traits
  (`Documented`, Context7 facet "Generated code"), so a per-type registry of
  `fn(&mut dyn Any, &RecordBatch)` would be needed anyway. That trades compile-time types for
  runtime lookups, the opposite of DESIGN §15.2's "generated forms derive from one declaration".
- **frunk** HLists plus `Poly` can iterate types, at a large ergonomic cost.

**Own option, a generic-method visitor:**
- `#[derive(PortSet)]` on `struct BindingData { #[port(epoch = facts)] calls: Rows<Call>, … }`
  emits `impl PortSet { fn each<V: PortVisitor>(v: &mut V) }`, where `trait PortVisitor` has
  `fn port<R: Record>(&mut self, field: &'static str, spec: PortSpec)`, plus `new`, `visit`,
  `matches`, `validation_inputs` and `stage_inputs`.
- cpg-core implements `PortVisitor` once each for load, write and declare. Async load works as a
  visitor that collects futures, or as an `async fn` in the trait on the pinned nightly.
- A tuple impl `(A, B): PortSet` composes sets. The 11-relation analysis publication set becomes
  one type (`AnalysisPublication`) referenced in the 16+ sites that now restate it.
- Writer-declares and stage-outputs then come from the same type, so the runtime equality check
  (`stages.rs:2129-2152`, `:2234`) is satisfied by construction and stays as a defensive check.

**`DomainCode` extensions:**
- `JsonSchema`, an integer enum carrying label extensions. This removes the 76-type
  `code_schema!` hand list, except for the exposure declaration.
- `label() -> &'static str`, taken from the existing `codes()` names.
- `const ALL`.

**strum** fits only non-codebook enums: the 11 `ALL` arrays and `name`/`from_name`. It is
already compiled, and the b0 "split codebook form" reason does not apply to those enums. Wire
names that feed `wire_identity` must be spelled explicitly.

**Shortlist:**
1. Extend `lctx-model-macros` with `PortSet` and the `DomainCode` additions. **The library
   alternatives add nothing that the own derive lacks.**
2. strum `VariantArray`/`IntoStaticStr` for non-codebook enums, as an optional convenience.

**Build-own burden:** roughly a new derive of the size of `Domain` (`lib.rs:211-504`, about 300
lines), plus trybuild or `compile_fail` cases for its errors.

### 2.2 In-memory relational containers and joins (M2-01, M2-02, M1-17)

**What no candidate provides:**
- No candidate gives all four of: byte charging before growth, conflict on same key with a
  different payload, `Id`-ordered deterministic iteration, and purity.
- multi_index_map and indexmap order by insertion. hashbrown is unordered.
- Charging stays an external estimate everywhere, except hashbrown's
  `try_reserve` + `allocation_size()`. That pair would make capacity charging exact and fallible
  for hash indexes (M1-19 `batching.rs` already hand-estimates ×2 for HashMap overhead). It costs
  a sort on output.
- **Datalog crates.** crepe (and ascent, L3) give joins but no budget and hash-order output.
  differential-dataflow, egglog and polars are out: none has a consumer, and polars is impure and
  brings a second Arrow.

**What would remove most of the repetition is own:**
- `Rows::need(id) -> Result<&R, ModelError>` replaces 52 copies of `need`.
- `Rows::index_by(&mut StateCharge, key_fn) -> ChargedMap<K, Vec<Id<R>>>`, plus a `semijoin`
  helper with ambiguity reporting, removes the O(n·m) `iter().any` scans
  (`documentary_templates.rs:46-65`).

**Shortlist:**
1. Extend `Rows`/`ChargedMap` with `need`, `index_by` and `unique_by`.
2. Optionally, a hashbrown-backed `ChargedHashIndex` for transient join indexes where exact
   capacity charging matters. hashbrown 0.17 is already compiled.
3. M1-17 stays on arrow-row/arrow-select; L1 covers it.

### 2.3 Canonical encoding and hashing (M2-05, M2-06)

**Observed (source), Arrow `Field` Debug changed between majors:**
- arrow-schema 56.2.0: `#[derive(Debug)]` on `Field` (`field.rs:47`).
- arrow-schema 57.3.1 and 59.3.0: a hand `impl Debug` that omits `name` when it is `"item"`,
  `nullable` when false, and empty `metadata` (`59.3.0/src/field.rs:63-100`, comment "Keep it
  short when debug-formatting `DataType::List`").

So `serving/mappings.rs:119-169`, which hashes `format!("{field:?}")`, already produced
different identity bytes for unchanged schemas across one Arrow upgrade.

`metadata` is a `HashMap`, so its Debug order would also vary per process. No field sets
metadata today (`grep with_metadata|set_metadata` over lctx-model and the macros crate: none),
so the hash is currently deterministic within a build.

`model.rs:311` (`format!("{sum:?}")`) hashes the Debug output of model-owned types. That is stable
only until a field is renamed, or until `str` Debug escaping changes with the toolchain's Unicode
tables.

**No library supplies a stable canonical Arrow-schema encoding.**
- IPC flatbuffers are not canonical (metadata order, builder layout).
- serde on `DataType` encodes variants by name or index and is not a contract.

The fix is own: encode the model's own declarations (`Scalar`, nullability, codes, sum arms)
through `KeySink`/`Key`. They already lower to Arrow, so hash them before lowering.

**JSON bytes:**
- `selection_digest`, `wire_identity` (schema JSON) and `EmbeddingSpec::canonical_json` hash
  `serde_json` output.
- serde_json 1.0.151 now formats floats with `zmij` (a dependency in its manifest) rather than
  `ryu`. That shows the float formatter is an implementation detail, not a contract.
- Map order depends on `preserve_order` feature unification.
- **serde_json_canonicalizer 0.3.2 (RFC 8785)** is the one library that turns these into a
  specified canonical form, and it is small. Adopting it is a byte change, which means a rebuild
  under ADR-0078.
- The cross-language need is absent: Python obtains the canonical spec from Rust through
  `lctx_semantics.canonical_embedding_spec` (`python/lctx_semantics/src/lib.rs:8`).

**Binary formats:**
- **postcard** is already adopted. Its 1.0 wire format is documented as stable. With
  `BTreeMap`-only types it is canonical by construction; no gap was found (L1).
- CBOR (ciborium needs a `CanonicalValue` detour; dcbor is a second authority), bcs (no floats)
  and borsh (a second derive) add no property that `KeySink` plus postcard lacks.

**Hex:** `hex` 0.4.3 is already compiled and behaviour-identical for the cursor. blake3's
`to_hex` covers `ContentHash` only.

**Shortlist:**
1. Own: replace the Debug-hashed parts with `Key` encodings of model declarations (M2-05).
2. `hex` for M2-06.
3. serde_json_canonicalizer, if the JSON-byte identities should be specified rather than de
   facto.

### 2.4 Checked scalars, builders and validation (M2-15, M1-20, M4-12)

Forward-plan §7 trigger: "nutype/garde/bon: a repeated checked scalar, contextual form or
required-field builder consumer".

**Checked scalars:**
- The evidence is one const-generic `Text<MIN,MAX>` with 6 instantiations, one `FiniteF64`, and
  hand version-string checks.
- nutype cannot carry the schemars-1 schema (its feature is `schemars08`) and does not express
  const generics. FiniteF64's single-zero, finite, Key-encoded contract fits neither
  ordered-float nor decorum.
- **The trigger is not met for nutype.**

**garde:**
- garde is the best fit if ADR-0073's "500 Unicode scalars plus byte budget" migration is done.
  Its length modes cover both.
- The remaining validators are cross-field and already sit in `#[model(validate=…)]`.
- That alone does not justify a second validation derive. An own `Text` variant with a scalar
  bound is about 10 lines.

**Builders:** `MethodParameters` needs `Default`, not bon. **The bon trigger is not met.**

**Config:** M4-12 has no layering consumer. Its duplication (three option builders, two
`verify-full` checks) is own consolidation. **figment/config are not indicated.**

### 2.5 Text and rendering (M2-12, M2-13, M3-15, M3-07)

**Rendering:**
- The stability defect in M2-13 is `{:?}` of domain enums and whole records feeding
  content-hashed corpus text. For example, `retrieval/build.rs:228` renders
  `format!("Literal {:?}", need(…)?)` over a whole record. **Templating does not fix this**:
  templates would still need a stable label per value.
- The fix is own: `DomainCode::label()` (§2.1) and explicit `Display`.
- If the forward-plan "brief-builder restructuring" trigger fires:
  - askama (typed, compile-time) suits code-authored briefs;
  - minijinja (with `fuel`) suits authored, data-driven templates.
- Either one makes the engine version part of rendered identity.

**M3-15:**
- `ruff_python_stdlib::identifiers::is_identifier` (pinned; L2) rather than unicode-ident for
  Python identifiers.
- The YAML frontmatter is already isolated by `markdown` 1.0.0. A YAML crate (serde-saphyr; not
  serde_yml, RUSTSEC-2025-0068) is warranted only if more than `title` is consumed.
- aho-corasick does not fit the boundary-token lookup.

**M3-07:** Ruff comment ranges are L2's.

**M2-12:** memchr would be a micro-optimization; textwrap does not fit byte windows.

**Shortlist:** own labels. No template engine without the trigger.

### 2.6 Interval and containment index (M2-10)

None of rust-lapper, iset, coitrees or intervaltree supports any of: innermost-containment
semantics, an observable work count for `visited_nodes`, or charged/fallible allocation.

The current implicit augmented tree (`attachment.rs:37-130`) is about 130 lines and already
returns `Ambiguous`/`BudgetExceeded`.

**Shortlist:**
1. Keep it.
2. Add a proptest differential against the existing scalar oracle
   (`cpg-extract/tests/attachment_oracle.rs`); the module has 0 unit tests.

### 2.7 Store-side mechanisms (M4-03/04/10/11/12/13/14)

**Schema diff (M4-04):**
- No Rust crate diffs a shadow schema inside a rolled-back transaction with ACLs and our finding
  taxonomy. The candidates are migration or data-diff tools with negligible use.
- The narrower gain is structural comparison of view and CHECK text instead of whitespace/quote
  stripping:
  - sqlparser 0.62 (pinned, no new crate) compares ASTs; its coverage of `pg_get_viewdef`
    output is unverified;
  - pg_query 6.2.1 is exact but uses the PG17 grammar and needs a C build.

**SQLSTATE (M4-13):**
- `tokio_postgres::error::SqlState` gives named constants (already compiled).
- sqlx `ErrorKind` gives four classes.
- The real gap is three divergent mappings, so merge them into one own function.

**Retry (M4-14):**
- backon 1.6.0 is the maintained choice (backoff is unmaintained, RUSTSEC-2025-0012).
- There are two sites (`cache.rs` admit, the `locks.rs` patience loop), and the committed-winner
  readback stays ours. Adoption is optional and low value.

**Locks (M4-10):** L1 records that `PgAdvisoryLock` does not fit. Nothing new.

**RSS (M4-11):**
- The 2026-09-23 reason ("no consumer") no longer holds: `stage_runtime.rs` has a sampler.
- A fit reason now replaces it. memory-stats reads all of smaps per call on Linux and has no
  peak. The bespoke VmRSS read is lighter and exact for its use.
- **Keep the own sampler; the rejection holds on fit.**

**Config (M4-12):** see §2.4.

### 2.8 Testing support

- **proptest** (not pinned) has three concrete differential consumers:
  - attachment index against the scalar oracle;
  - cursor round trip and refusal;
  - `framed_part`/KeySink injectivity, with a known-answer vector. M2 found no v3 known-answer
    vector, and the `ids__*.snap` snapshots have no owning test file.
- **bolero** is the upgrade path if fuzz engines are wanted.
- **trybuild**: its F-trigger fires only once the own derive grows (§2.1). Nightly stderr churn is
  the cost.
- **cargo-mutants** is an optional audit of `attachment.rs`, `charged.rs` and `rows.rs`, never a
  gate.
- insta 1.48.0 → 1.49.0 is a minor upgrade (finding only).

## 3. Uncertainties and absences

**Not compiled or probed:**
- the facet, frunk and `PortSet` visitor designs;
- nutype const-generic support (inferred from docs and features);
- sqlparser on `pg_get_viewdef` output;
- whether serde_json's switch to `zmij` changed any float text relative to `ryu` for values in
  current identities (not checked; the identity JSON may contain no floats);
- whether `EmbeddingSpec` contains map-typed fields affected by `preserve_order`.

**Search coverage:**
- crates.io for every named candidate plus pgdiff, dibs, pgcrate, rust-pgdatadiff, sqlstate,
  enum-iterator, num_enum, variant_count, veil, stable-hash and deterministic-hash (none changes a
  conclusion);
- web search "Rust crate PostgreSQL schema diff catalog compare 2026";
- the local RustSec database, as of 2026-09-23.

**Absence claims:**
- "No Rust PostgreSQL shadow-schema differ" and "no stable canonical Arrow-schema encoding" rest
  on that coverage. An empty search does not prove absence.
- Heap-size crates, typed indexes, ascent/datafrog and graph crates were deliberately left to L3.

**Already-compiled status** comes from `Cargo.lock` plus `cargo tree -i` (normal, build and dev
edges) at HEAD `b0256115`, and `lctx-workspace-hack/Cargo.toml`.
