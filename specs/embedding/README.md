# Embedding specifications (DESIGN §11.1, ADR-0131)

- `qwen3-embedding-8b.json`: format3 configuration for the pinned checkpoint and custom service.
  Both clients request normalized full4096 F32 values. The model's typed encoder identity includes
  the actual checkpoint, tokenizer, pooling, precision and output realization; the complete
  configuration JSON is not the encoder key. Document rendering/token admission and query
  instruction/token admission have separate recipe identities. Python obtains these identities
  from the Rust declarations through the editable native extension.
- One canonical full value per encoder and exact rendered-input SHA-256 supplies a declared
  normalized1024 projection for native search and E1 analytics. Projection takes the first1024
  admitted F32 components, accumulates their squared norm in F64, then rounds normalized components
  once to F32. Full values supply bounded rescoring and independent numerical references.
  Projection changes require no inference; query-only recipe changes preserve document winners.
- Compilation verifies local tokenizer assets and counts the complete encoder input with saved
  truncation and padding disabled. Normal document admission is2048 tokens; query admission is8192.
  Headers, mandatory context and special tokens count. Oversized indivisible originals remain
  addressable with explicit vector unavailability.
- `conformance_inputs.json`: the shared inputs of the client conformance check (E2): both clients
  must build byte-identical request bodies for them (`request_bodies.json`), always checked, and
  vectors agreeing to cosine ≥ 0.9995 against a live service (`just embed-conformance`).
- `lctx-fake-embedder.json` and `fake_vectors.json` are mechanical client known answers, generated
  by the owning Rust control. They establish no checkpoint usefulness or retrieval-quality claim.
