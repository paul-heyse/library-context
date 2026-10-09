# xxHash in Rust: `twox-hash` and `xxhash-rust` — agent reference

**Versions covered:** `twox-hash` 2.1.5 (2026-10-04, commit `0222275`) and `xxhash-rust` 0.8.19 (2026-09-28, commit `3135fb1`). Both implement the xxHash C reference **0.8.x** algorithm family (XXH32, XXH64, XXH3-64, XXH3-128).

**Provenance:** API listings are transcribed from source at those commits. Behavioural claims in §5–§8 were executed in a harness on rustc 1.97.0 (x86_64 Linux, 2-vCPU Xeon with AVX2 + AVX-512F), with build checks on `i686-unknown-linux-musl`, `thumbv7em-none-eabihf`, `thumbv7neon-unknown-linux-gnueabihf`, `aarch64-unknown-linux-gnu` and `wasm32-unknown-unknown` (± `simd128`). Appendix A lists exactly what was run. Anything labelled *upstream claim* was not reproduced.

---

## 0. Selection rules

Both crates produce **bit-identical output** for every algorithm, seed and secret tested (lengths 0–600 plus 1 KiB–100 KiB, four seeds, custom secrets, streaming at 18 chunk sizes). Pick on API shape and deployment constraints, and mixing them across a producer/consumer boundary is safe.

| Requirement | Pick | Why |
|---|---|---|
| Portable x86_64 binary, long inputs, max throughput | `twox-hash` | Runtime AVX2→SSE2 dispatch. `xxhash-rust` is compile-time only and stays on SSE2 in a generic build (~1.4–1.8× slower on bulk XXH3 here). |
| Binary built with `-Ctarget-cpu=native`/`x86-64-v4` on AVX-512 hardware | `xxhash-rust` | Only crate with an AVX-512 path (~1.7× faster than twox on bulk XXH3 here). |
| `const`/`static` hashes, compile-time match tables | `xxhash-rust` (`const_xxh*`) | `twox-hash` has no `const fn` hashing. |
| wasm32 with SIMD | `xxhash-rust` + `-Ctarget-feature=+simd128` | `twox-hash` is scalar on wasm. |
| `no_std` + no allocator, streaming XXH3 | either | `xxhash-rust`: `Xxh3Default`/`Xxh3`. `twox-hash`: `xxhash3_64::RawHasher<S>` + `SecretBuffer` (scalar without `std`). |
| Custom secrets without panics | `twox-hash` | Returns `Result`. `xxhash-rust` panics or returns `Option`. |
| Checkpoint/resume a stream (serde) | `twox-hash` `serialize` | XXH32/XXH64 state only. |
| `std::io::Write` sink (`io::copy`) | `xxhash-rust` `std` | `twox-hash` has no `io::Write` impls. |
| Random-seeded `BuildHasher` out of the box | `twox-hash` `xxhash64::RandomState` | `xxhash-rust` has none; seed it yourself. |
| Zero dependencies, minimal compile | `xxhash-rust` | No deps; every feature off by default. `twox-hash` defaults pull `rand` 0.10 (+`chacha20`, `getrandom`, `libc`). |
| Streaming XXH3 over > 4 GiB on 32-bit targets | `xxhash-rust` | `twox-hash` 2.1.5 returns a wrong digest there (G7). |
| `HashMap` with small keys | usually neither XXH3 streaming hasher | XXH3 streaming state is 360–576 B (plus a heap allocation for twox's `XxHash3_64`); per-key cost loses to XXH64, SipHash, or a direct `oneshot` (§7.3). |
| Format-mandated XXH32/XXH64 (LZ4 frame, zstd checksum, Parquet Bloom filter) | either | Identical outputs. |

---

## 1. Algorithm facts shared by both crates

- **XXH32**: `u32` seed and output, 16-byte stripes, 4×u32 accumulators. The tail is consumed 4 bytes, then 1 byte, at a time.
- **XXH64**: `u64` seed and output, 32-byte stripes, 4×u64 accumulators.
- **XXH3** (64- and 128-bit): parameterised by `seed: u64` **and** `secret: [u8; ≥136]`. The default secret (`kSecret`) is 192 bytes. Size classes are 0, 1–3, 4–8, 9–16, 17–128, 129–240, and >240 ("long"). The long path processes 64-byte stripes into 8×u64 accumulators, with `(secret_len − 64) / 8` stripes per block (16 for the default secret, so 1 KiB blocks). **SIMD only affects the >240-byte path**; ≤240 is scalar in both crates.
- **Seed/secret semantics** (C reference, implemented identically by both; verified):
  - `with_seed(s)`: for ≤240 B, the default secret with `s` mixed into the short-input formulas. For >240 B, a derived secret is used: kSecret with each 16-byte pair transformed as `lo = lo + s`, `hi = hi − s` (LE u64). `s = 0` is the default hash.
  - `with_secret(k)`: seed 0, secret `k` for **all** lengths.
  - `with_seed_and_secret(s, k)` (C `XXH3_*_withSecretandSeed`): ≤240 B behaves exactly as `with_seed(s)` (**`k` ignored**). >240 B behaves exactly as `with_secret(k)` (**`s` ignored**).
- **Stability**: XXH3 output has been frozen since C v0.8.0. Output is independent of endianness and pointer width (twox runs Miri on s390x and i686; xxhash-rust cross-tests powerpc BE). The one exception is the twox 32-bit streaming bug in G7.
- **Known vectors** (verified in both crates): XXH32("",0)=`0x02CC5D05`, XXH64("",0)=`0xEF46DB3751D8E999`, XXH3_64("")=`0x2D06800538D394C2`, XXH3_128("")=`0x99AA06D3014798D86001C324468D497F`.
- **Canonical external form**: zero-padded lowercase hex (`{:08x}`/`{:016x}`/`{:032x}`) or `to_be_bytes()` matches `xxhsum` / python-xxhash (verified against libxxhash 0.8.3; for XXH3-128 the high 64 bits come first).
- **Not cryptographic**, and neither crate claims HashDoS resistance. A random seed is not a substitute for SipHash when keys are adversarial.

---

## 2. Crate profiles

| | `twox-hash` 2.1.5 | `xxhash-rust` 0.8.19 |
|---|---|---|
| Maintainer | Jake Goulding (shepmaster) | Douman (DoumanAsh) |
| License | MIT | **BSL-1.0** (Boost); check `cargo-deny` allowlists |
| Edition / MSRV | 2021 / `rust-version = "1.81"` (declared, CI-checked) | 2018 / none declared; CI checks 1.64.0 |
| Dependencies | `rand` (optional, on by default via `random`), `serde` (optional) | none |
| Default features | `random, xxhash32, xxhash64, xxhash3_64, xxhash3_128, std` | **none**; the crate is empty until features are enabled |
| `no_std` | `default-features = false`; `alloc` gates heap-backed XXH3 streaming | always `#![no_std]`; `std` only adds `io::Write` |
| SIMD (XXH3 long path) | **Runtime**: x86_64 AVX2 → SSE2 → scalar; aarch64 NEON. Requires the `std` feature. No AVX-512; scalar on x86-32, wasm and `no_std`. | **Compile-time** `cfg(target_feature)`: AVX-512F > AVX2 > SSE2 \| NEON \| wasm `simd128` > scalar. x86, x86_64, aarch64, arm64ec (NEON since 0.8.19), arm+neon, wasm. |
| `const fn` hashing | no (only `const fn with_seed` constructors for XXH32/64) | yes: `const_xxh32`, `const_xxh64`, `const_xxh3` |
| `core::hash::Hasher` impls | `XxHash32` (zero-extended), `XxHash64`, `XxHash3_64`, `xxhash3_64::RawHasher<S>`. **Not** `XxHash3_128`. | `Xxh64`, `Xxh3`, `Xxh3Default`. **Not** `Xxh32`. |
| `BuildHasher` | `xxhash32::{State, RandomState}`, `xxhash64::{State, RandomState}`. Nothing for XXH3 (use `BuildHasherDefault`, which allocates per hasher). | `Xxh64Builder`, `Xxh3Builder` (seed/secret, `const`), `Xxh3DefaultBuilder`. No random state. |
| `io::Write` | no | `Xxh32`, `Xxh64`, `Xxh3`, `Xxh3Default` (feature `std`) |
| serde | `XxHash32`, `XxHash64` (`serialize`) | no |
| Bad-secret handling | `Result` (`OneshotWithSecretError`, `SecretTooShortError<S>`, `SecretWithSeedError<S>`) | `assert!` panic, `Option`, or const panic |
| Reset | none (construct a new hasher) | `reset()` / `reset(seed)` |
| Assurance | Miri (x86_64, i686, s390x); proptest comparison against C via FFI (`comparison` crate); `thumbv6m` `no_std` CI build; feature-matrix checks | Valgrind runs (default, `-sse2`, `+avx2`); cross tests on arm-musleabi, i586, powerpc, aarch64; tests against `xxhash-c-sys` |
| crates.io (2026-10-08) | 261 M downloads, 60.8 M in last 90 days, 381 dependents | 114 M, 34.2 M, 933 dependents |

---

## 3. `twox-hash` 2.x

### 3.1 Cargo recipes

```toml
# Library crates: opt out of defaults. Each algorithm is a feature (the changelog recommends this for compile time).
twox-hash = { version = "2.1", default-features = false, features = ["xxhash3_64", "std"] }

# no_std, no allocator: oneshot functions + RawHasher only; XXH3 is scalar.
twox-hash = { version = "2.1", default-features = false, features = ["xxhash64", "xxhash3_64"] }

# no_std with an allocator: adds XxHash3_*::new/with_seed/with_seed_and_secret (still scalar).
twox-hash = { version = "2.1", default-features = false, features = ["xxhash3_64", "alloc"] }
```

| Feature | Effect |
|---|---|
| `xxhash32`, `xxhash64`, `xxhash3_64`, `xxhash3_128` | Compile the module and the root alias. |
| `alloc` | Heap-held secret enables `XxHash3_64/128::{new, with_seed, with_seed_and_secret, into_secret}` and streaming via those types. |
| `std` | Implies `alloc` and **enables runtime SIMD dispatch** for XXH3. |
| `random` | `rand` (`>=0.9, <=0.11`, `thread_rng`) provides `xxhash32::RandomState` and `xxhash64::RandomState`. There is no random state for XXH3. |
| `serialize` | serde `Serialize`/`Deserialize` for `XxHash32`/`XxHash64` state. |

### 3.2 API surface (transcribed)

```rust,ignore
// Root aliases
pub use xxhash32::Hasher as XxHash32;      // feature xxhash32
pub use xxhash64::Hasher as XxHash64;      // feature xxhash64
pub use xxhash3_64::Hasher as XxHash3_64;  // feature xxhash3_64
pub use xxhash3_128::Hasher as XxHash3_128;// feature xxhash3_128

// ---- XXH32 ---- derives Debug, Clone, PartialEq; Default = with_seed(0); NOT Copy (removed in 2.0)
impl XxHash32 {
    pub fn oneshot(seed: u32, data: &[u8]) -> u32;
    pub const fn with_seed(seed: u32) -> Self;
    pub const fn seed(&self) -> u32;
    pub const fn total_len(&self) -> u64;
    pub const fn total_len_32(&self) -> u32;
    pub fn finish_32(&self) -> u32;               // the real 32-bit result
}
impl core::hash::Hasher for XxHash32 { /* finish() = finish_32() as u64, upper 32 bits always 0 */ }
pub struct xxhash32::State;        // Clone; State::with_seed(u32); BuildHasher<Hasher = XxHash32>
pub struct xxhash32::RandomState;  // feature random; Default draws the seed via rand::random()

// ---- XXH64 ---- same shape, u64 seed/output
impl XxHash64 {
    pub fn oneshot(seed: u64, data: &[u8]) -> u64;   // fully #[inline]; matches C speed per upstream
    pub const fn with_seed(seed: u64) -> Self;
    pub const fn seed(&self) -> u64;
    pub const fn total_len(&self) -> u64;
}
impl core::hash::Hasher for XxHash64 {}
pub struct xxhash64::State;        // with_seed(u64)
pub struct xxhash64::RandomState;  // feature random

// ---- XXH3-64 ---- derives Clone only
impl XxHash3_64 {
    pub fn oneshot(input: &[u8]) -> u64;
    pub fn oneshot_with_seed(seed: u64, input: &[u8]) -> u64;
    pub fn oneshot_with_secret(secret: &[u8], input: &[u8]) -> Result<u64, OneshotWithSecretError>;
    pub fn oneshot_with_seed_and_secret(seed: u64, secret: &[u8], input: &[u8])
        -> Result<u64, OneshotWithSecretError>;   // secret only validated when input.len() > 240 (G4)
    // feature alloc (each constructor heap-allocates the secret; clone() allocates too):
    pub fn new() -> Self;
    pub fn with_seed(seed: u64) -> Self;
    pub fn with_seed_and_secret(seed: u64, secret: impl Into<Box<[u8]>>)
        -> Result<Self, SecretTooShortError<Box<[u8]>>>;   // any length >= 136
    pub fn into_secret(self) -> Box<[u8]>;
}
impl core::hash::Hasher for XxHash3_64 {}          // feature alloc; Default = new()

// ---- XXH3-128 ---- same oneshot family returning u128; does NOT impl Hasher
impl XxHash3_128 {
    // ...oneshot, oneshot_with_seed, oneshot_with_secret, oneshot_with_seed_and_secret -> u128
    // feature alloc: new, with_seed, with_seed_and_secret, into_secret, plus:
    pub fn write(&mut self, input: &[u8]);
    pub fn finish_128(&self) -> u128;
}

// ---- Zero-alloc streaming (both xxhash3_64:: and xxhash3_128::) ----
pub struct RawHasher<S>;                           // Clone
impl<S> RawHasher<S> { pub fn new(sb: SecretBuffer<S>) -> Self; pub fn into_secret(self) -> S; }
impl<S: FixedBuffer> core::hash::Hasher for xxhash3_64::RawHasher<S> {}
impl<S: FixedBuffer> xxhash3_128::RawHasher<S> { pub fn write(&mut self, &[u8]); pub fn finish_128(&self) -> u128; }

pub struct SecretBuffer<S>;   // Clone; holds seed + secret S + 256-byte stripe buffer
impl<S: FixedBuffer> SecretBuffer<S> {
    pub fn new(seed: u64, secret: S) -> Result<Self, SecretTooShortError<S>>;  // validates len >= 136, no modification
}
impl<S: FixedMutBuffer> SecretBuffer<S> {
    pub fn with_seed(seed: u64, secret: S) -> Result<Self, SecretWithSeedError<S>>; // S must be exactly 192 B; overwritten with derived secret
}
impl SecretBuffer<&'static [u8; 192]> { pub const fn default() -> Self; }  // default seed + secret, no allocation
impl<S> SecretBuffer<S> { pub fn into_secret(self) -> S; }

pub unsafe trait FixedBuffer: AsRef<[u8]> {}                    // [u8; N], &[u8; N], &mut [u8; N], Box<[u8]> (alloc)
pub unsafe trait FixedMutBuffer: FixedBuffer + AsMut<[u8]> {}   // [u8; N], &mut [u8; N], Box<[u8]> (alloc)
pub const DEFAULT_SECRET_LENGTH: usize = 192;
pub const SECRET_MINIMUM_LENGTH: usize = 136;
// Errors implement Debug + Display + core::error::Error; SecretTooShortError/SecretWithSeedError::into_secret() return the buffer.
// Display: "The secret must have at least 136 bytes" / "The secret must be exactly 192 bytes"
```

### 3.3 SIMD dispatch internals

- The `dispatch!` macro (`src/xxhash3/large.rs`) wraps each long-path entry point. On `x86_64` with `std`: `is_x86_feature_detected!("avx2")` → AVX2 path, else `"sse2"` → SSE2, else scalar. On `aarch64` with `std`: `is_aarch64_feature_detected!("neon")` → NEON. All other configurations (no `std`, i686, wasm, riscv, …) use scalar.
- `is_*_feature_detected!` short-circuits to `true` when the feature is statically enabled, and std caches CPUID. Per-call cost is one predictable branch.
- There is **no AVX-512 path**.
- Internal `--cfg _internal_xxhash3_force_{scalar,sse2,avx2,neon}` flags force a path. They exist for testing and benchmarking and are not a stable interface.
- `oneshot` keeps ≤240 B inline and outlines the long path (`#[inline(never)]`) so latency-sensitive short hashes stay small. `oneshot_with_seed` outlines secret derivation for long inputs. 2.1.5 retuned these paths to match or beat C (*upstream claim*).

### 3.4 Serde state (verified)

`XxHash64` serialises to `{"total_len":u64,"seed":u64,"core":{"v1".."v4"},"buffer":[u8;32],"buffer_usage":usize}`, mirroring C's `XXH64_state_t` naming. `XxHash32` is analogous. Round-trip mid-stream followed by further writes reproduces the uninterrupted hash. There is no serde support for XXH3 state.

### 3.5 Migrating from 1.x (still pinned by `sp-crypto-hashing`, `gxhash`, `fastcrypto`, `deno_graph`, …)

| 1.x | 2.x |
|---|---|
| `XxHash` (alias) | `XxHash64` |
| `RandomXxHashBuilder`, `RandomXxHashBuilder64` / `32` | `xxhash64::RandomState` / `xxhash32::RandomState` (feature `random`) |
| `Xxh3Hash64`, `xxh3::Hash64` | `XxHash3_64`, `xxhash3_64::Hasher` |
| `xxh3::hash64(data)` / `hash64_with_seed(data, seed)` / `hash64_with_secret(data, secret)` | `XxHash3_64::oneshot(data)` / `oneshot_with_seed(seed, data)` / `oneshot_with_secret(secret, data)?`. **Argument order flipped.** |
| `hasher.write(data); hasher.finish()` for one-shot use | `XxHash64::oneshot(seed, data)`: zero allocation, faster |
| `digest` crate integration | removed (xxHash is not cryptographic) |
| `XxHash32`/`XxHash64: Copy` | `Clone` only |
| Random XXH3 builder | removed |
| XXH3-128 | added back in 2.1.0 |

Duplicate 1.x/2.x copies are common in dependency graphs. Inspect them with `cargo tree -d -i twox-hash`.

---

## 4. `xxhash-rust` 0.8

### 4.1 Cargo recipes

```toml
xxhash-rust = { version = "0.8", features = ["xxh3"] }                       # runtime XXH3
xxhash-rust = { version = "0.8", features = ["xxh3", "const_xxh3"] }         # + compile-time XXH3 and const_custom_default_secret
xxhash-rust = { version = "0.8", features = ["xxh64", "std"] }               # XXH64 + io::Write
```

Features: `xxh32`, `const_xxh32`, `xxh64`, `const_xxh64`, `xxh3`, `const_xxh3`, `std` (adds `io::Write` only). Enabling `xxh3` pulls in shared XXH32/XXH64 constants internally but **not** the public `xxh32`/`xxh64` modules.

**Versioning policy:** `0.8.x` tracks C `0.8.x`, and the author will not bump major/minor until C does. New API therefore lands in patch releases. A caret `0.8` requirement accepts all of it.

### 4.2 API surface (transcribed)

```rust,ignore
// ---- xxh32 (feature xxh32) ----
pub fn xxh32(input: &[u8], seed: u32) -> u32;
pub struct Xxh32;     // Clone, Default(seed 0), io::Write(std); NO core::hash::Hasher impl; 44 B
impl Xxh32 { pub const fn new(seed: u32) -> Self; pub fn update(&mut self, input: &[u8]);
             pub fn digest(&self) -> u32; pub fn reset(&mut self, seed: u32); }

// ---- xxh64 (feature xxh64) ----
pub fn xxh64(input: &[u8], seed: u64) -> u64;
pub struct Xxh64;     // Clone, Default, Hasher, io::Write(std); 80 B
impl Xxh64 { pub const fn new(seed: u64) -> Self; pub fn update(&mut self, &[u8]);
             pub fn digest(&self) -> u64; pub fn reset(&mut self, seed: u64); }
pub struct Xxh64Builder; // Clone, Copy, Default; const fn new(seed: u64); const fn build(self) -> Xxh64; BuildHasher

// ---- xxh3 (feature xxh3) ----
pub fn xxh3_64(input: &[u8]) -> u64;
pub fn xxh3_64_with_seed(input: &[u8], seed: u64) -> u64;
pub fn xxh3_64_with_secret(input: &[u8], secret: &[u8]) -> u64;     // assert!(secret.len() >= 136): PANICS
pub fn xxh3_64_with_secret_input(input: &[u8], secret: &SecretInput<impl AsRef<[u8]>>) -> u64;
pub fn xxh3_128(input: &[u8]) -> u128;                                // + _with_seed, _with_secret, _with_secret_input
// No one-shot seed+secret function: use Xxh3Builder::new().with_seed(s).with_secret(k).build() + update + digest.

pub struct SecretInput<T>;
impl<T: AsRef<[u8]>> SecretInput<T> { pub fn try_new(input: T) -> Option<Self>; }        // len >= 136
impl<const N: usize> SecretInput<[u8; N]> { pub const fn new(input: [u8; N]) -> Self; } // const assert N >= 136

pub struct Xxh3Default;  // Clone, Default, Hasher, io::Write(std). Default seed+secret only. 384 B, align 64
impl Xxh3Default { pub const fn new() -> Self; pub fn update(&mut self, &[u8]);
                   pub fn digest(&self) -> u64; pub fn digest128(&self) -> u128; pub fn reset(&mut self); }

pub struct Xxh3;         // Clone, Default, Hasher, io::Write(std). Owns a 192-B secret + seed. 576 B, align 64
impl Xxh3 {
    pub const fn new() -> Self;
    pub const fn with_secret(secret: [u8; 192]) -> Self;   // exactly 192 B (the doc comment wrongly says "custom seed")
    pub fn with_seed(seed: u64) -> Self;                   // not const; Xxh3Builder::with_seed is
    pub fn update(&mut self, &[u8]); pub fn digest(&self) -> u64; pub fn digest128(&self) -> u128;
    pub fn reset(&mut self);                               // keeps seed and secret
}
pub struct Xxh3Builder;  // Clone, Copy, Default; 216 B; BuildHasher<Hasher = Xxh3>
impl Xxh3Builder { pub const fn new() -> Self; pub const fn with_seed(self, seed: u64) -> Self;
                   pub const fn with_secret(self, secret: [u8; 192]) -> Self; pub const fn build(self) -> Xxh3; }
pub struct Xxh3DefaultBuilder; // Clone, Copy, Default; const fn new(); const fn build(self) -> Xxh3Default; BuildHasher

// ---- const_xxh32 / const_xxh64 / const_xxh3 ----
pub const fn xxh32(input: &[u8], seed: u32) -> u32;
pub const fn xxh64(input: &[u8], seed: u64) -> u64;
pub const fn xxh3_64(input: &[u8]) -> u64;                  // + xxh3_64_with_seed(input, seed)
pub const fn xxh3_64_with_secret(input: &[u8], secret: &[u8; 192]) -> u64;  // fixed-size secret
pub const fn xxh3_128(input: &[u8]) -> u128;                // + _with_seed, _with_secret(&[u8; 192])
pub const fn const_custom_default_secret(seed: u64) -> [u8; 192];  // re-exported from const_xxh3 only
```

The size constants (192, 136) are **not exported**, so write them as literals. `digest*()` takes `&self` and is non-destructive (verified).

### 4.3 SIMD selection

The path is chosen once, when the crate is compiled, from `cfg(target_feature)`:

| Target / flags | Path |
|---|---|
| `x86_64-*` default (`x86-64` baseline includes SSE2) | SSE2 |
| `-Ctarget-feature=+avx2` or `-Ctarget-cpu=x86-64-v3` | AVX2 |
| `-Ctarget-cpu=x86-64-v4` / `native` on AVX-512 host | AVX-512F |
| `i686-*` | SSE2; `i586-*` gets scalar |
| `aarch64-*` | NEON (on by default) |
| `arm64ec-pc-windows-msvc` | NEON (0.8.19+) |
| `thumbv7neon-*` / `armv7` + `+neon` | NEON via `core::arch::arm` (builds on stable, verified) |
| `wasm32-*` + `-Ctarget-feature=+simd128` | wasm SIMD128 |
| anything else | scalar |

`RUSTFLAGS` apply to the whole build graph. A binary compiled for AVX2 or AVX-512 dies with **SIGILL** on CPUs without those features. A `#[target_feature]` attribute in your own code cannot change which path the dependency compiled.

### 4.4 `const fn` rules

- Force CTFE by binding to `const`/`static`, or by using the call in array lengths or match patterns. A `const fn` called in runtime position runs at runtime on the slower scalar const implementation.
- CTFE cost on rustc 1.97: hashing an `include_bytes!` blob in a `const` added about 0.5 s of build time at 64 KiB and about 4.4 s at 1 MiB, with no lint tripped (verified). That is fine for identifiers and small assets. For large assets, hash in `build.rs` and emit a constant instead.
- `const_xxh3::*_with_secret` only accepts `&[u8; 192]`.

---

## 5. Call translation table

| Operation | `twox-hash` | `xxhash-rust` |
|---|---|---|
| XXH32 one-shot | `XxHash32::oneshot(seed, d)` | `xxh32::xxh32(d, seed)` |
| XXH32 stream | `XxHash32::with_seed(s)`; `.write(d)`; `.finish_32()` | `Xxh32::new(s)`; `.update(d)`; `.digest()` |
| XXH64 one-shot | `XxHash64::oneshot(seed, d)` | `xxh64::xxh64(d, seed)` |
| XXH64 stream | `XxHash64::with_seed(s)` / `.write` / `.finish()` | `Xxh64::new(s)` / `.update` / `.digest()` |
| XXH3-64 | `XxHash3_64::oneshot(d)` | `xxh3::xxh3_64(d)` |
| XXH3-64 seeded | `XxHash3_64::oneshot_with_seed(s, d)` | `xxh3::xxh3_64_with_seed(d, s)` |
| XXH3-64 secret | `XxHash3_64::oneshot_with_secret(k, d)?` | `xxh3::xxh3_64_with_secret(d, k)` (panics) or `_with_secret_input(d, &SecretInput)` |
| XXH3-64 seed+secret | `XxHash3_64::oneshot_with_seed_and_secret(s, k, d)?` | `Xxh3Builder::new().with_seed(s).with_secret(k192).build()` + `update` + `digest` |
| XXH3-128 | `XxHash3_128::oneshot*` | `xxh3::xxh3_128*` |
| XXH3 stream, default | `XxHash3_64::new()` (allocates) or `xxhash3_64::RawHasher::new(SecretBuffer::default())` | `Xxh3Default::new()` |
| XXH3 stream, seeded | `XxHash3_64::with_seed(s)` (allocates) or `RawHasher::new(SecretBuffer::with_seed(s, [0u8; 192])?)` | `Xxh3::with_seed(s)` or `const` `Xxh3Builder::new().with_seed(s).build()` |
| XXH3 stream, custom secret | `XxHash3_64::with_seed_and_secret(0, k)?` (any len ≥136) or `RawHasher::new(SecretBuffer::new(0, k)?)` | `Xxh3::with_secret(k)` (**exactly** 192) |
| XXH3-128 stream | `XxHash3_128::new()`; `.write`; `.finish_128()` | `Xxh3Default`/`Xxh3` → `.digest128()` |
| Reset | none | `.reset()` / `.reset(seed)` |
| Derive seeded secret | `SecretBuffer::with_seed(s, [0u8; 192])?.into_secret()` | `const_xxh3::const_custom_default_secret(s)` (const) |
| Compile-time | none | `const_xxh3::xxh3_64(b"..")`, etc. |

---

## 6. Gotchas (each verified unless stated)

**G1 — the `Hash` trait is not "hash these bytes."** `"abc".hash(&mut h)` appends a `0xFF` terminator, and `b"abc"[..].hash(&mut h)` prefixes the length via `write_length_prefix`/`write_usize`. Neither equals `oneshot(b"abc")`. `usize`-based prefixes differ between 32- and 64-bit targets, and std's `Hash` impls are not guaranteed stable across Rust versions. For anything persisted, sent over the wire, or compared across processes, hash an explicit canonical byte encoding with `oneshot`/`write`/`update` (recipe R6). Never use `#[derive(Hash)]` for persistent hashes.

**G2 — seed+secret is a length-dependent split.** At ≤240 B the secret is ignored; at >240 B the seed is ignored. This matches C and both crates agree, but it surprises anyone expecting both parameters to always contribute.

**G3 — a seed-derived secret is not equivalent to the seed.** Compare `xxh3_64_with_secret(d, &const_custom_default_secret(s))` with `xxh3_64_with_seed(d, s)`. For XXH3-64 they are equal only for lengths 17–128 and >240; they differ for 0–16 and 129–240. For XXH3-128 they are equal only for >240. The doc comment on `xxhash_rust::xxh3::xxh3_64_with_seed` suggests precomputing the secret this way "for efficiency", which silently changes short-input hashes. `Xxh3Builder::with_seed` handles this correctly: it stores both and uses the default secret plus seed for ≤240 B.

**G4 — twox `oneshot_with_seed_and_secret` does not validate the secret for short input.** A 10-byte secret returns `Ok` for a 4-byte input and `Err` only once input exceeds 240 B. Test secret plumbing with long inputs.

**G5 — xxhash-rust secret failures panic.** `xxh3_*_with_secret` uses `assert!(secret.len() >= 136)`. Use `SecretInput::try_new` (returns `Option`) for runtime secrets. `SecretInput::new` in a `const` turns a short secret into a compile error. Streaming `Xxh3::with_secret` takes exactly `[u8; 192]`.

**G6 — XXH32 as a `HashMap` hasher.** twox's `XxHash32::finish()` zero-extends to `u64`. hashbrown takes its 7-bit control tag from the top bits, which are then always zero. Here 1 M inserts plus 1 M lookups took 307 ms with `xxhash32::State` against 210 ms with `xxhash64::State`. xxhash-rust avoids this by not implementing `Hasher` for `Xxh32`.

**G7 — twox-hash XXH3 streaming is wrong past 4 GiB on 32-bit targets.** The stream length counter in `RawHasherCore` is `usize`. Streaming 4097 × 1 MiB on `i686-unknown-linux-musl` (release) gave `0x5e81d979626f93e4`; the correct value, produced by twox on x86_64 and by xxhash-rust on both, is `0xc8860983f0cffb8a`. With overflow checks enabled (debug builds) the `+=` should panic instead. One-shot is unaffected, and XXH32/XXH64 streaming use `u64`. Not checked against the upstream issue tracker.

**G8 — twox XXH3 hashers allocate.** `XxHash3_64::new`, `with_seed` and `clone` each heap-allocate the 192-byte secret (1 allocation each, verified with a counting allocator). As a result, `BuildHasherDefault<XxHash3_64>` performs **one allocation per `hash_one`/map operation** (1000 hashes, 1000 allocations). Use `RawHasher` (R4). The one-shot functions never allocate. xxhash-rust never allocates.

**G9 — argument order differs between crates and can fail silently.** Seeded calls differ in type (`(seed, data)` vs `(data, seed)`), so swapping them is a compile error. Secret calls are `(&[u8], &[u8])` in both crates, in **opposite order**: twox `oneshot_with_secret(secret, input)`, xxhash-rust `xxh3_64_with_secret(input, secret)`. A swap compiles and hashes the wrong buffer whenever both are ≥136 B.

**G10 — const fns in runtime position are slow.** See §4.4.

**G11 — SIMD reality.** A generic x86_64 build of xxhash-rust runs SSE2 even on an AVX-512 host (bulk XXH3 here: ~14 GiB/s vs twox's ~21–25). twox picks AVX2 at runtime but has no AVX-512 path and no SIMD at all without `std` or off x86_64/aarch64.

**G12 — finish is non-destructive and there is no twox reset.** `finish`/`finish_32`/`finish_128`/`digest*` take `&self`; you can keep writing afterwards (verified). twox has no `reset`, so construct a new hasher; for XXH3 that allocates (G8).

**G13 — XXH3 does not compose across chunks.** Hashing chunks in parallel and combining them does not equal the whole-input hash. Define your own tree or merge scheme and version it.

**G14 — the low 64 bits of XXH3-128 equal XXH3-64 for inputs >240 B** (all 1759 long lengths tested) but not for ≤240 B. Don't derive one from the other.

**G15 — HashDoS.** `RandomState` randomises only the seed. Neither crate claims flooding resistance, so use std's SipHash (or a hasher designed for it) for attacker-controlled keys.

**G16 — dependency hygiene.** twox's defaults pull `rand` 0.10 (+`chacha20`, `getrandom`, `libc`) through `random`; libraries should set `default-features = false`. xxhash-rust's BSL-1.0 license may need allowlisting.

---

## 7. Performance

Measured on one machine: 2-vCPU Intel Xeon @ 2.80 GHz (AVX2 + AVX-512F), rustc 1.97.0, `--release`, median of 7 runs. Run-to-run noise is ±15–20%, so treat differences under ~30% as parity.

### 7.1 Bulk one-shot XXH3-64 (GiB/s)

| Build | twox 64 KiB | xxhash-rust 64 KiB | twox 1 MiB | xxhash-rust 1 MiB |
|---|---|---|---|---|
| default (`x86-64`) | 25.0 | 14.1 (SSE2) | 20.9 | 14.7 |
| `-Ctarget-feature=+avx2` | 28.5 | 25.8 (AVX2) | 20.6 | 21.0 |
| `-Ctarget-cpu=native` (AVX-512) | 23.9 | **40.6** (AVX-512) | 19.2 | **32.2** |

XXH3-128 tracks XXH3-64 at these sizes. XXH64 was ~11–12 GiB/s in both crates under every build on this host.

**Correction (2026-10-08, second host):** on an Intel Xeon @ 2.10 GHz with AVX-512, re-run on a quiet machine, `-Ctarget-cpu=native` dropped twox-hash's XXH64 to ~4.9 GiB/s at ≥16 KiB inputs. xxhash-rust stayed at ~10.3 GiB/s, and both crates were ~10.4 GiB/s on the default build. The first host didn't show this, so the regression is CPU-dependent. If you build with native flags and rely on XXH64 throughput, benchmark it on your target CPU.

### 7.2 Short inputs, default build (ns per one-shot)

| len | twox XXH3-64 | xxrs XXH3-64 | twox XXH3-128 | xxrs XXH3-128 | twox XXH64 | xxrs XXH64 |
|---|---|---|---|---|---|---|
| 8 | 4.6 | 3.9 | 4.1 | 3.5 | 6.0 | 7.4 |
| 16 | 3.4 | 3.7 | 5.3 | 5.0 | 6.4 | 8.3 |
| 64 | 7.0 | 6.4 | 13.8 | 6.4 | 13.1 | 13–22 |
| 128 | 11.2 | 10.6 | 23.0 | 11.6 | 19.5 | 19.4 |
| 200 | **16.5** | 27.9 | 36.1 | **24.8** | 28.9 | 25.1 |
| 240 | **20.9** | 30.5 | 37.9 | **26.9** | 30.8 | 29.6 |
| 512 | 41.5 | 48.1 | 42.9 | 53.2 | 48.5 | 49.6 |

These patterns held across all three builds (default, `+avx2`, `native`):
- For XXH3-64 at 200–240 B (the 129–240 size class), twox is ~1.5–2× faster.
- For XXH3-128 at 64–240 B, xxhash-rust is ~1.4–2.2× faster.
- At the other measured sizes the two are at parity.

### 7.3 Streaming and hasher-construction costs

| Scenario | twox | xxhash-rust |
|---|---|---|
| Stream 1 MiB XXH3, 64 B chunks | 5.9 GiB/s | 5.6 |
| … 256 B chunks | 11.2 | 8.5 |
| … 4 KiB chunks | 21.4 | 12.2 |
| … 64 KiB chunks | 23.4 | 12.8 |

`BuildHasher::hash_one(u64)` costs per call:

| Hasher | ns/call |
|---|---|
| twox `BuildHasherDefault<XxHash3_64>` (allocates) | 40–43 |
| twox custom `RawHasher` builder, default secret (R4) | 19.5 |
| twox custom `RawHasher` builder, seeded | 25 |
| xxhash-rust `Xxh3DefaultBuilder` | 11–12 |
| xxhash-rust `Xxh3Builder` seeded (builds a 576-B state with its own 192-B secret per hash) | 53 |
| twox `xxhash64::State` | 8 |
| xxhash-rust `Xxh64Builder` | 12 |
| std `RandomState` (SipHash-1-3) | 13.7 |
| **direct `oneshot(&k.to_le_bytes())`, either crate** | **2.0** |

`HashMap<u64,u64>`, 1 M inserts (ms): twox `BuildHasherDefault<XxHash3_64>` 368, xxhash-rust `Xxh3DefaultBuilder` 211, twox `xxhash64::State` 159, xxhash-rust `Xxh64Builder` 210, std 161.

The lesson: XXH3's advantage is bulk throughput. Through the streaming `Hasher` interface with small keys, the state setup dominates. For fixed-size keys, hash bytes directly with `oneshot` (e.g. precomputed-hash tables), or use a `HashMap`-oriented hasher.

*Upstream claim* (twox `comparison/README.md`): XXH3-64 one-shot at 57.7 GiB/s on a Ryzen 9 3950X, against 57.2 GiB/s for C with AVX2 and 25.8 GiB/s for C's default MSVC build; 35.2 GiB/s on an M1 Max against 35.3 for C. XXH64 at 16.7 vs 16.6 (x86_64) and 13.4 vs 13.4 (aarch64). xxhash-rust publishes no benchmark suite.

---

## 8. Deployment playbook

### 8.1 Build flags

```toml
# .cargo/config.toml: opt the whole build into AVX2 (benefits xxhash-rust; twox already dispatches at runtime).
# Every deployment CPU must support x86-64-v3 (roughly Haswell / Zen 1 and newer), or the binary will SIGILL.
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "target-cpu=x86-64-v3"]
```

- **Fleet-portable artifacts** (distro packages, generic containers): prefer twox for bulk XXH3, or accept SSE2 speed from xxhash-rust.
- **Pinned hardware** (internal services on known AVX-512 nodes): xxhash-rust with `target-cpu=x86-64-v4`/`native` is the fastest option measured.
- **wasm**: `RUSTFLAGS="-C target-feature=+simd128"` for xxhash-rust (verified to build). twox is scalar.
- **Embedded / `no_std`**: both build on `thumbv7em-none-eabihf` (verified). twox needs `default-features = false`; without `alloc` it offers only one-shot functions and `RawHasher`, and calling `XxHash3_64::new()` is a compile error (verified).
- **Startup self-check** of the SIMD path:

```rust
fn log_hash_backends() {
    // xxhash-rust: decided at compile time by the same flags your crate sees
    let xxrs = if cfg!(target_feature = "avx512f") { "avx512" } else if cfg!(target_feature = "avx2") { "avx2" }
        else if cfg!(any(target_feature = "sse2", target_feature = "neon")) { "sse2/neon" } else { "scalar" };
    // twox-hash: decided at runtime (requires its `std` feature)
    #[cfg(target_arch = "x86_64")]
    let twox = if std::arch::is_x86_feature_detected!("avx2") { "avx2" } else { "sse2" };
    #[cfg(not(target_arch = "x86_64"))]
    let twox = "neon-or-scalar";
    eprintln!("xxhash-rust xxh3 path: {xxrs}; twox-hash xxh3 path: {twox}");
}
fn main() { log_hash_backends(); }
```

### 8.2 Persisted / cross-system hashes

- Persist an algorithm tag, seed and secret identifier next to every stored hash. Changing any of them changes every value.
- **Collision budget**: the chance of any collision among *n* random items is about n²/2^(b+1). For XXH3-64 at n = 10⁹ that is **≈2.7%**; for XXH3-128 it is ≈1.5·10⁻²¹. Use 128-bit hashes for content addressing and deduplication keys.
- Hash a **canonical encoding** (fixed-width little-endian integers, length-prefixed variable fields), not `Hash` impls (G1, R6).
- Export as `format!("{h:032x}")` or `h.to_be_bytes()` to match `xxhsum` and other bindings.
- Custom secrets: 136–192+ bytes of CSPRNG output, generated once and versioned. Treat the secret as configuration, not as a cryptographic key.

### 8.3 Hash maps

- Small keys: neither crate's XXH3 streaming hasher is a win (§7.3). XXH64 builders are competitive with SipHash. Prefer std for untrusted keys, or a dedicated map hasher.
- If you need a twox XXH3 `BuildHasher`, use R4 rather than `BuildHasherDefault<XxHash3_64>`.
- With xxhash-rust, prefer `Xxh3DefaultBuilder` over a seeded `Xxh3Builder` (11 vs 53 ns per hash) unless you need the seed.

---

## 9. Ecosystem (top reverse dependencies by downloads, with features enabled)

**twox-hash**: `parquet` (xxhash64; Parquet split-block Bloom filters use XXH64), `lz4_flex` (xxhash32; LZ4 frame checksums), `ruzstd` (xxhash64; zstd content checksum), `zerovec` / `icu_provider_blob` (xxhash64), `tame-index`, `foyer-common`/`foyer-storage`, `deno_core` (std, xxhash64), `mysql`/`mysql_async`, `cargo-deny` (xxhash32), `hakari`, `smoldot`, `glyph_brush`, `two-face`, `xwin`, `lance-core`/`lance-index`, `datafusion-spark`. Still on 1.6.x: `sp-crypto-hashing` (`digest_0_10`), `gxhash`, `fastcrypto`, `deno_graph`.

**xxhash-rust**: `polars-core` (xxh3), `polars-parquet` / `parquet2` (xxh64), `redis` (xxh3), `nextest-runner` / `quick-junit` (xxh3, xxh64), `vrl` (all three), `server_fn` / `dioxus-fullstack` (**const_xxh64**), `ssri`, `growable-bloom-filter`, `lance-encoding`, `wasmer-wasix` (xxh64), `re_log_types` / `re_log_encoding` (xxh32, xxh64), `cairo-lang-defs`, `sequoia-openpgp`, `lsm-tree` / `fjall` (xxh3), `solana-svm` (xxh64), `cubecl-common` (xxh3 + const_xxh3), `s3s` (std, xxh64, xxh3).

---

## 10. Recipes (all compiled and run)

**R1 — hash a file.**

```rust
use std::{fs::File, io::{self, Read}, path::Path};

/// xxhash-rust: `Xxh3Default: io::Write` (feature "std"), so io::copy drives the loop.
fn file_xxh3_128_xxrs(path: &Path) -> io::Result<u128> {
    let mut h = xxhash_rust::xxh3::Xxh3Default::new();
    io::copy(&mut File::open(path)?, &mut h)?;
    Ok(h.digest128())
}

/// twox-hash: no io::Write impl. Read in large chunks (>= 4 KiB keeps you near bulk speed, see 7.3).
fn file_xxh3_128_twox(path: &Path) -> io::Result<u128> {
    let mut f = File::open(path)?;
    let mut h = twox_hash::XxHash3_128::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 { break; }
        h.write(&buf[..n]);
    }
    Ok(h.finish_128())
}

fn main() -> io::Result<()> {
    let p = std::env::temp_dir().join("xxh_recipe_r1.bin");
    std::fs::write(&p, vec![7u8; 300_000])?;
    assert_eq!(file_xxh3_128_xxrs(&p)?, file_xxh3_128_twox(&p)?);
    // For multi-GiB files on 64-bit targets, mmap + one-shot is fastest.
    Ok(())
}
```

**R2 — compile-time dispatch on string hashes.**

```rust
use xxhash_rust::{const_xxh3::xxh3_64 as cxxh3, xxh3::xxh3_64};

const GET: u64 = cxxh3(b"GET");
const PUT: u64 = cxxh3(b"PUT");
const _: () = assert!(GET != PUT); // compile-time uniqueness check of the table

fn route(method: &str) -> &'static str {
    match xxh3_64(method.as_bytes()) {
        // A different string can collide with GET's hash. If misrouting matters, confirm with method == "GET".
        GET => "get",
        PUT => "put",
        _ => "other",
    }
}

fn main() { assert_eq!(route("GET"), "get"); assert_eq!(route("PATCH"), "other"); }
```

**R3 — keyed XXH3 with a versioned custom secret, identical across crates.**

```rust
use xxhash_rust::xxh3::{self, SecretInput};

// Generated once: `head -c 192 /dev/urandom > xxh3_secret.bin`, then versioned with your data.
const SECRET_BYTES: &[u8; 192] = include_bytes!("xxh3_secret.bin");
const SECRET: SecretInput<[u8; 192]> = SecretInput::new(*SECRET_BYTES); // short secret: compile error

fn keyed_xxrs(data: &[u8]) -> u64 { xxh3::xxh3_64_with_secret_input(data, &SECRET) }

fn keyed_twox(data: &[u8]) -> u64 {
    // Note the argument order: (secret, input). See G9.
    twox_hash::XxHash3_64::oneshot_with_secret(SECRET_BYTES, data).expect("secret is >= 136 bytes")
}

fn main() {
    for n in [0usize, 10, 200, 5000] {
        let d = vec![3u8; n];
        assert_eq!(keyed_xxrs(&d), keyed_twox(&d));
    }
}
```

**R4 — zero-allocation XXH3 `BuildHasher` for twox-hash** (fixes G8; also works in `no_std` without `alloc`).

```rust
use std::hash::BuildHasher;
use twox_hash::xxhash3_64::{RawHasher, SecretBuffer, DEFAULT_SECRET_LENGTH};

#[derive(Clone, Copy, Default)]
pub struct Xxh3DefaultState;
impl BuildHasher for Xxh3DefaultState {
    type Hasher = RawHasher<&'static [u8; DEFAULT_SECRET_LENGTH]>;          // 360 B, borrows the static secret
    fn build_hasher(&self) -> Self::Hasher { RawHasher::new(SecretBuffer::default()) }
}

/// Seeded variant: derive the secret once, then copy 192 B per hasher (no derivation, no allocation).
#[derive(Clone)]
pub struct Xxh3SeededState { seed: u64, secret: [u8; DEFAULT_SECRET_LENGTH] }
impl Xxh3SeededState {
    pub fn new(seed: u64) -> Self {
        let secret = SecretBuffer::with_seed(seed, [0u8; DEFAULT_SECRET_LENGTH]).unwrap().into_secret();
        Self { seed, secret }
    }
}
impl BuildHasher for Xxh3SeededState {
    type Hasher = RawHasher<[u8; DEFAULT_SECRET_LENGTH]>;                   // 544 B
    fn build_hasher(&self) -> Self::Hasher {
        RawHasher::new(SecretBuffer::new(self.seed, self.secret).unwrap())  // cannot fail: 192 >= 136
    }
}

fn main() {
    use std::collections::HashMap;
    let mut m: HashMap<&str, u32, Xxh3SeededState> = HashMap::with_hasher(Xxh3SeededState::new(42));
    m.insert("a", 1);
    assert_eq!(Xxh3SeededState::new(42).hash_one(7u64), {
        use std::hash::{Hash, Hasher};
        let mut h = twox_hash::XxHash3_64::with_seed(42); 7u64.hash(&mut h); h.finish()
    });
    assert_eq!(Xxh3DefaultState.hash_one(7u64), xxhash_rust::xxh3::Xxh3DefaultBuilder::new().hash_one(7u64));
}
```

**R5 — checkpoint and resume a long XXH64 stream (twox `serialize`).**

```rust
use std::hash::Hasher as _;
use twox_hash::XxHash64;

fn main() -> serde_json::Result<()> {
    let mut h = XxHash64::with_seed(0);
    h.write(b"first part");
    let saved = serde_json::to_string(&h)?;            // persist anywhere (any serde format)
    let mut resumed: XxHash64 = serde_json::from_str(&saved)?;
    resumed.write(b" second part");
    assert_eq!(resumed.finish(), XxHash64::oneshot(0, b"first part second part"));
    Ok(())
}
```

**R6 — portable structured hashing** (fixed-width LE integers with length prefixes, never `#[derive(Hash)]`).

```rust
struct Record<'a> { id: u64, name: &'a str, tags: &'a [&'a str] }

fn record_hash(r: &Record) -> u128 {
    let mut h = xxhash_rust::xxh3::Xxh3Default::new();
    h.update(b"record/v1");                               // domain + schema version tag
    h.update(&r.id.to_le_bytes());
    h.update(&(r.name.len() as u64).to_le_bytes());
    h.update(r.name.as_bytes());
    h.update(&(r.tags.len() as u64).to_le_bytes());
    for t in r.tags {
        h.update(&(t.len() as u64).to_le_bytes());
        h.update(t.as_bytes());
    }
    h.digest128()
}

fn main() {
    let a = record_hash(&Record { id: 1, name: "ab", tags: &["c"] });
    let b = record_hash(&Record { id: 1, name: "a", tags: &["bc"] });
    assert_ne!(a, b); // the length prefixes prevent concatenation ambiguity
    println!("{a:032x}");
}
```

**R7 — `HashMap` wiring (for reference; see §8.3 before choosing).**

```rust
use std::{collections::HashMap, hash::BuildHasherDefault};

fn main() {
    let _a: HashMap<u64, u32, BuildHasherDefault<twox_hash::XxHash64>> = HashMap::default(); // fixed seed 0
    let _b: HashMap<u64, u32, twox_hash::xxhash64::RandomState> = HashMap::default();         // feature "random"
    let _c: HashMap<u64, u32, _> = HashMap::with_hasher(twox_hash::xxhash64::State::with_seed(42));
    let _d: HashMap<u64, u32, xxhash_rust::xxh3::Xxh3DefaultBuilder> = HashMap::default();
    let _e: HashMap<u64, u32, _> = HashMap::with_hasher(xxhash_rust::xxh64::Xxh64Builder::new(42));
    // A const-constructed seeded XXH3 prototype (secret derived at compile time):
    static PROTO: xxhash_rust::xxh3::Xxh3 = xxhash_rust::xxh3::Xxh3Builder::new().with_seed(42).build();
    let mut h = PROTO.clone();
    h.update(b"x");
    assert_eq!(h.digest(), xxhash_rust::xxh3::xxh3_64_with_seed(b"x", 42));
}
```

**R8 — `no_std` streaming without an allocator** (verified on `thumbv7em-none-eabihf` and `wasm32`; in a real crate add `#![no_std]`).

```rust
use core::hash::Hasher;
use twox_hash::xxhash3_64::{RawHasher, SecretBuffer};
use xxhash_rust::xxh3::Xxh3Default;

pub fn stream(chunks: &[&[u8]]) -> (u64, u64) {
    let mut t = RawHasher::new(SecretBuffer::default()); // twox: features = ["xxhash3_64"], default-features = false
    let mut x = Xxh3Default::new();                      // xxhash-rust: features = ["xxh3"]
    for c in chunks { t.write(c); x.update(c); }
    (t.finish(), x.digest())
}

fn main() { let (a, b) = stream(&[b"ab", b"cd"]); assert_eq!(a, b); }
```

---

## Appendix A — what was executed

| Check | Result |
|---|---|
| Cross-crate equality: XXH32/XXH64/XXH3-64/XXH3-128 one-shot, lengths 0–600, 1023–1025, 4096, 65536, 100003; seeds 0, 1, 0xdeadbeef, u64::MAX; custom 200-B secret | all equal |
| Streaming == one-shot, both crates, 11 lengths × 18 chunk sizes (1…4097) | all equal |
| Same suite under `-Ctarget-cpu=native` (AVX-512 paths) | all equal |
| Known empty-input vectors; twox README vector `XxHash64::oneshot(1234, b"some bytes") = 0xeab55659a496d78b` | match |
| Hex / big-endian canonical forms vs python-xxhash (libxxhash 0.8.3) | match |
| Seed+secret split (G2), derived-secret ranges (G3), short-secret behaviour (G4, G5) | as stated |
| `Hash` trait vs raw bytes (G1) | differ for `&str` and `&[u8]` |
| i686-musl, 4097 MiB stream (G7) | twox wrong, xxhash-rust correct |
| Allocation counts with a counting global allocator (G8) | as stated |
| `size_of` (x86_64): twox `XxHash32` 56, `XxHash64` 88, `XxHash3_64`/`128` 368 (+192 heap), `RawHasher<[u8;192]>` 544, `RawHasher<&[u8;192]>` 360; xxrs `Xxh32` 44, `Xxh64` 80, `Xxh3` 576, `Xxh3Default` 384, `Xxh3Builder` 216, `Xxh64Builder` 8 | measured |
| `no_std` builds: `thumbv7em-none-eabihf`, `wasm32-unknown-unknown` (± `simd128`); `thumbv7neon` and `aarch64` checks | pass |
| CTFE of 64 KiB / 1 MiB `include_bytes!` via `const_xxh3` | +~0.5 s / +~4.4 s build time, correct values |
| serde round-trip mid-stream (XXH32, XXH64) | correct |
| All §8.1 and §10 code blocks compiled and run | pass |
