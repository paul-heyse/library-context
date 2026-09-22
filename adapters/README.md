# Adapters

One Cargo workspace per analyzer (ADR-0003): `ruff-extract/` and `pyrefly-extract/`, each with its
own `Cargo.lock` and, if needed, `rust-toolchain.toml`. Adapters run as separate processes, emit
Arrow IPC conforming to `crates/cpg-schema`, and are exercised by `just test-all` and `just deps`.

None exist yet. The first decision of increment 1 is which Ruff and Pyrefly revisions to pin
(see `docs/pins.md`, "Analyzers").
