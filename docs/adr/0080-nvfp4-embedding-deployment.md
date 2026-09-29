---
id: ADR-0080
title: Use the published SM120 wheel and NVFP4 embedding checkpoint
status: accepted
date: 2026-09-28
supersedes: []
superseded-by: null
design: [§11.1]
evidence: Tested
revisit: The checkpoint, wheel, runtime, accelerator or output contract changes.
---

## Context

The operator selected gpu-stack's published B3/r2 wheel and NVFP4 embedding checkpoint,
and requested basic working validation and background re-embedding, without accuracy assessment
or broad tests. [§11.1](../design/sections/synthesis-and-serving.md#section-11-1) owns embedding
identity; ADR-0078's cache, retrieval and 1024-dimensional output contracts remain in force.
The local source handoff is `/home/paul/gpu-stack/docs/HANDOFF.md` and its role E deployment.

## Options

1. Keep BF16 weights and change only the wheel. This minimizes vector changes, but is not the
   checkpoint the operator selected.
2. Adopt the published NVFP4 checkpoint and optimized SM120 wheel (selected). Reuse the existing
   clients, canonical spec and full-spec cache separation; rebuild vectors under a new identity.
3. Add throughput tuning or comparative accuracy gates now. These are outside the requested scope.

## Decision

`services/vllm` locks the B3/r2 wheel, torch 2.14.0+cu132 and CUDA runtime 13.4.2. The explicit local
wheel index is `/home/paul/wheelhouse/gpu-stack`. `scripts/embed_serve.py` runs the published
`Qwen3-Embedding-8B-NVFP4-r2` checkpoint with role E settings, loopback HTTP and no inherited
batch-invariant settings. Keep 1024-dimensional MRL prefix/L2 output, existing query/document
instructions, token admission and the float32 HTTP client contract.

The format-2 spec's revision and tokenizer_revision are `sha256:` identifiers of the checkpoint's
published `SHA256SUMS` bytes. That manifest covers weights, tokenizer, config and provenance.
The launcher verifies the manifest against the spec and every listed file before startup; these
identifiers are not Hugging Face revisions and are not passed as revision flags. The full wheel
version identifies the server. Paths are deployment details, not vector identity.

The unchanged canonical spec hash separates every new vector from BF16 cache entries. Rebuild
both compiler profiles in the background. Validate before current-only cutover, then retire
obsolete runtime generations under ADR-0078; retain benchmark artifacts. Do not add a historical
reader, alternate production model or compatibility path.

## Consequences

The local wheel and checkpoint are explicit reconstruction prerequisites. The endpoint name alone
still does not attest to a remote service's weights. Quantization changes vectors; no BF16
accuracy-equivalence or local throughput claim is made. Basic validation covers startup, tokenizer
admission and finite, correctly indexed, unit-normalized 1024-vectors through the production client.
Broad tests, live cosine comparison, ranking evaluation and benchmarking are not run by operator
instruction. [PR4 evidence](../design_review/evidence/2026-09-28_pr4/README.md) records the bounded
receipt; the forward plan owns current adoption state and remaining background work.
