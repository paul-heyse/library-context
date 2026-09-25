# Embedding specs (DESIGN §11.1, ADR-0043)

- `qwen3-embedding-8b.json`: the one spec every live vector is made under. It is canonical JSON
  (fields in the order `cpg_core::embed::Spec` declares, no whitespace); its SHA-256 is the spec
  hash, which both the Rust compile-time client (`lctx-embed`) and the Python query client
  (`lctx_mcp`) recompute and compare.
- `conformance_inputs.json`: the shared inputs of the client conformance check (E2): both clients
  must build byte-identical request bodies for them (`request_bodies.json`), always checked, and
  vectors agreeing to cosine ≥ 0.9995 against a live service (`just embed-conformance`).
